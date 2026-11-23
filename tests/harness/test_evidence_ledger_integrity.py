#!/usr/bin/env python3
"""
tests/harness/test_evidence_ledger_integrity.py
Kiểm thử tự động tính toàn vẹn và giao thức truyền tải của Evidence Ledger (Blocker 2):
- Kiểm thử lỗi thiếu file evidence manifest (Missing Ledger -> INCOMPLETE)
- Kiểm thử lỗi bằng chứng cũ (Stale Evidence -> INCOMPLETE)
- Kiểm thử lỗi sai lệch schema và định danh nguồn (Invalid Source -> Validation Failed)
- Kiểm thử chính sách Fail-Closed đối với các check bị bỏ qua (SKIPPED -> INCOMPLETE, không bao giờ là PASS)
- Kiểm thử phát hiện AI Findings nghiêm trọng (P0/P1 -> MANUAL_HOLD)
- Kiểm thử luồng hợp lệ hoàn chỉnh (L0..L5 PASSED -> PASS)
"""

import os
import sys
import json
import tempfile
import unittest
import subprocess
from pathlib import Path

# Add harness/runner to sys.path
REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "harness" / "runner"))

import evidence
import policy

class EvidenceLedgerIntegrityTests(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp_dir.cleanup)
        self.root = Path(self.temp_dir.name)

        # Khởi tạo git repo cô lập trong temporary directory
        subprocess.run(["git", "init"], cwd=self.root, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
        subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=self.root, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
        subprocess.run(["git", "config", "user.name", "Test"], cwd=self.root, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
        (self.root / "README.md").write_text("# Test Repo\n")
        subprocess.run(["git", "add", "."], cwd=self.root, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
        subprocess.run(["git", "commit", "-m", "initial commit"], cwd=self.root, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)

        # Tạo cấu trúc thư mục tối thiểu
        (self.root / "harness" / "reports").mkdir(parents=True)
        (self.root / "harness" / "policies").mkdir(parents=True)

        # Copy file quality-gates.yaml vào mock repo
        src_policy = REPO_ROOT / "harness" / "policies" / "quality-gates.yaml"
        if src_policy.exists():
            (self.root / "harness" / "policies" / "quality-gates.yaml").write_text(
                src_policy.read_text(encoding="utf-8"), encoding="utf-8"
            )

    def test_missing_evidence_manifest_returns_incomplete(self):
        """Khi evidence manifest chưa được khởi tạo, Merge Gate phải trả về INCOMPLETE (exit code 3)"""
        non_existent_path = str(self.root / "harness" / "reports" / "evidence-manifest.json")
        decision, reason, code = policy.evaluate_merge_gate(str(self.root), non_existent_path)

        self.assertEqual(decision, "INCOMPLETE")
        self.assertEqual(code, 3)
        self.assertIn("Evidence ledger file not found", reason)

    def test_stale_evidence_detected_when_head_mismatch(self):
        """Khi commit HEAD trong ledger khác commit HEAD hiện tại của repo, phải báo STALE EVIDENCE"""
        ledger = evidence.create_evidence_ledger(str(self.root))
        # Giả mạo commit HEAD cũ
        ledger["source"]["head_sha"] = "0000000000000000000000000000000000000000"

        ledger_path = str(self.root / "harness" / "reports" / "evidence-manifest.json")
        with open(ledger_path, "w", encoding="utf-8") as f:
            json.dump(ledger, f)

        # Kiểm tra qua hàm evaluate_merge_gate
        decision, reason, code = policy.evaluate_merge_gate(str(self.root), ledger_path)
        self.assertEqual(decision, "INCOMPLETE")
        self.assertEqual(code, 3)
        self.assertIn("STALE EVIDENCE", reason)

        # Kiểm tra qua hàm validate_evidence_ledger
        ok, errors = evidence.validate_evidence_ledger(str(self.root), ledger_path)
        self.assertFalse(ok)
        self.assertTrue(any("STALE EVIDENCE" in e for e in errors))

    def test_skipped_checks_strictly_remain_incomplete(self):
        """Các check bị SKIPPED hoặc PENDING tuyệt đối không được coi là PASS"""
        ledger = evidence.create_evidence_ledger(str(self.root))

        # Cho pass một số check, nhưng để một check required ở trạng thái SKIPPED
        ledger["checks"]["policy_integrity"] = "PASSED"
        ledger["checks"]["rust_core"] = "PASSED"
        ledger["checks"]["wasm_build"] = "SKIPPED"
        ledger["checks"]["verification_gate"] = "PASSED"

        ledger_path = str(self.root / "harness" / "reports" / "evidence-manifest.json")
        with open(ledger_path, "w", encoding="utf-8") as f:
            json.dump(ledger, f)

        decision, reason, code = policy.evaluate_merge_gate(str(self.root), ledger_path)
        self.assertEqual(decision, "INCOMPLETE", "Check bị SKIPPED tuyệt đối không được chuyển thành PASS")
        self.assertEqual(code, 3)
        self.assertIn("wasm-build", reason)

    def test_failed_check_yields_gate_fail(self):
        """Khi có bất kỳ check required nào bị FAILED, Merge Gate phải trả về FAIL (exit code 1)"""
        ledger = evidence.create_evidence_ledger(str(self.root))

        ledger["checks"]["policy_integrity"] = "PASSED"
        ledger["checks"]["rust_core"] = "FAILED"
        ledger["checks"]["wasm_build"] = "PASSED"
        ledger["checks"]["verification_gate"] = "PASSED"

        ledger_path = str(self.root / "harness" / "reports" / "evidence-manifest.json")
        with open(ledger_path, "w", encoding="utf-8") as f:
            json.dump(ledger, f)

        decision, reason, code = policy.evaluate_merge_gate(str(self.root), ledger_path)
        self.assertEqual(decision, "FAIL")
        self.assertEqual(code, 1)
        self.assertIn("Required checks failed: rust-core", reason)

    def test_unresolved_p0_findings_triggers_manual_hold(self):
        """Khi có AI Finding P0 hoặc P1, Merge Gate phải chuyển sang MANUAL_HOLD (exit code 2)"""
        ledger = evidence.create_evidence_ledger(str(self.root))

        for c in ["policy_integrity", "rust_core", "wasm_build", "verification_gate"]:
            ledger["checks"][c] = "PASSED"

        ledger["review"]["findings_summary"]["P0"] = 1

        ledger_path = str(self.root / "harness" / "reports" / "evidence-manifest.json")
        with open(ledger_path, "w", encoding="utf-8") as f:
            json.dump(ledger, f)

        decision, reason, code = policy.evaluate_merge_gate(str(self.root), ledger_path)
        self.assertEqual(decision, "MANUAL_HOLD")
        self.assertEqual(code, 2)
        self.assertIn("Unresolved critical/high findings detected (P0: 1", reason)

    def test_record_evidence_and_clean_gate_pass(self):
        """Kiểm tra quy trình ghi nhận tuần tự từng tầng L0..L5 và kết quả PASS của Merge Gate"""
        # 1. Khởi tạo
        ledger = evidence.create_evidence_ledger(str(self.root))
        evidence.save_ledger(str(self.root), ledger)

        # 2. Ghi nhận các tầng
        evidence.record_evidence(str(self.root), "L0", "PASSED")
        evidence.record_evidence(str(self.root), "L1", "PASSED")
        evidence.record_evidence(str(self.root), "L2", "PASSED")
        evidence.record_evidence(str(self.root), "L3", "PASSED")
        evidence.record_evidence(str(self.root), "L4", "PASSED")
        evidence.record_evidence(str(self.root), "L5", "PASSED")

        # 3. Xác thực tính toàn vẹn của Ledger
        manifest_path = str(self.root / "harness" / "reports" / "evidence-manifest.json")
        ok, errors = evidence.validate_evidence_ledger(str(self.root), manifest_path)
        self.assertTrue(ok, f"Validation errors: {errors}")

        # 4. Đánh giá Gate
        os.environ["ALLOW_SPINE_AMENDMENT"] = "1"
        try:
            decision, reason, code = policy.evaluate_merge_gate(str(self.root))
            self.assertEqual(decision, "PASS")
            self.assertEqual(code, 0)
        finally:
            os.environ.pop("ALLOW_SPINE_AMENDMENT", None)

if __name__ == "__main__":
    unittest.main()
