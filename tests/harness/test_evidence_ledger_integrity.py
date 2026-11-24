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

    def test_tier_aggregation_merges_independent_tier_artifacts(self):
        """Kiểm tra chức năng aggregate_tier_results gom các tier-*.json độc lập vào manifest mà không bị ghi đè"""
        # Tạo riêng rẽ từng file tier artifact
        evidence.record_tier_file(str(self.root), "L0", "PASSED")
        evidence.record_tier_file(str(self.root), "L1", "PASSED")
        evidence.record_tier_file(str(self.root), "L2", "PASSED")
        evidence.record_tier_file(str(self.root), "L3", "PASSED")
        evidence.record_tier_file(str(self.root), "L4", "PASSED")
        evidence.record_tier_file(str(self.root), "L5", "PASSED")

        # Gọi tổng hợp
        manifest = evidence.aggregate_tier_results(str(self.root))

        self.assertEqual(manifest["checks"]["policy_integrity"], "PASSED")
        self.assertEqual(manifest["checks"]["rust_static"], "PASSED")
        self.assertEqual(manifest["checks"]["rust_core"], "PASSED")
        self.assertEqual(manifest["checks"]["integration_tests"], "PASSED")
        self.assertEqual(manifest["checks"]["wasm_build"], "PASSED")
        self.assertEqual(manifest["checks"]["verification_gate"], "PASSED")
        self.assertEqual(manifest["decision"], "PASS")

    def test_l5_verify_artifacts_fails_when_dist_empty_and_passes_when_present(self):
        """Kiểm tra verify_artifacts trả về False khi dist rỗng và True khi có artifact WASM/JS hợp lệ"""
        dist_dir = self.root / "dist"

        # 1. Khi chưa có thư mục dist/
        self.assertFalse(evidence.verify_artifacts(str(self.root)))

        # 2. Khi dist/ tồn tại nhưng không có file wasm/js
        dist_dir.mkdir(parents=True, exist_ok=True)
        (dist_dir / "random.txt").write_text("not wasm")
        self.assertFalse(evidence.verify_artifacts(str(self.root)))

        # 3. Khi dist/ có file .wasm hợp lệ
        (dist_dir / "office_ui.wasm").write_bytes(b"\x00asm\x01\x00\x00\x00fake_wasm_bytes")
        self.assertTrue(evidence.verify_artifacts(str(self.root)))

    def test_tier_aggregation_fails_on_source_identity_head_mismatch(self):
        """Tier artifact có head_sha khác với ledger gốc phải bị từ chối và fail-closed (Source Identity Enforcement)"""
        ledger = evidence.create_evidence_ledger(str(self.root))
        evidence.save_ledger(str(self.root), ledger)

        # Ghi nhận tier L1 nhưng giả mạo head_sha khác
        tier_path = evidence.record_tier_file(str(self.root), "L1", "PASSED")
        with open(tier_path, "r", encoding="utf-8") as f:
            tdata = json.load(f)
        tdata["source"]["head_sha"] = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
        with open(tier_path, "w", encoding="utf-8") as f:
            json.dump(tdata, f)

        manifest = evidence.aggregate_tier_results(str(self.root))
        self.assertEqual(manifest["decision"], "FAIL")
        self.assertIn("source mismatch (head_sha)", manifest["decision_reason"])

    def test_tier_aggregation_fails_on_source_identity_tree_or_base_mismatch(self):
        """Tier artifact có tree_sha hoặc base_sha sai lệch phải bị từ chối"""
        ledger = evidence.create_evidence_ledger(str(self.root))
        evidence.save_ledger(str(self.root), ledger)

        # Ghi nhận tier L2 nhưng giả mạo tree_sha
        tier_path = evidence.record_tier_file(str(self.root), "L2", "PASSED")
        with open(tier_path, "r", encoding="utf-8") as f:
            tdata = json.load(f)
        tdata["source"]["tree_sha"] = "baadf00dbaadf00dbaadf00dbaadf00dbaadf00d"
        with open(tier_path, "w", encoding="utf-8") as f:
            json.dump(tdata, f)

        manifest = evidence.aggregate_tier_results(str(self.root))
        self.assertEqual(manifest["decision"], "FAIL")
        self.assertIn("source mismatch (tree_sha)", manifest["decision_reason"])

    def test_tier_aggregation_fails_on_corrupt_tier_json(self):
        """File tier artifact bị hỏng format JSON phải kích hoạt FAIL trong aggregation"""
        ledger = evidence.create_evidence_ledger(str(self.root))
        evidence.save_ledger(str(self.root), ledger)

        corrupt_path = self.root / "harness" / "reports" / "tier-l1.json"
        corrupt_path.write_text("{ unparseable json corrupt content ...")

        manifest = evidence.aggregate_tier_results(str(self.root))
        self.assertEqual(manifest["decision"], "FAIL")
        self.assertIn("Corrupt tier artifact", manifest["decision_reason"])

    def test_tier_aggregation_fails_on_duplicate_tier_artifacts(self):
        """Phát hiện 2 file artifact cùng báo cáo cho một tier (duplicate collision)"""
        ledger = evidence.create_evidence_ledger(str(self.root))
        evidence.save_ledger(str(self.root), ledger)

        # Tạo file tier L1 chuẩn
        evidence.record_tier_file(str(self.root), "L1", "PASSED")

        # Tạo thêm file thứ hai cũng khai báo tier: L1
        dup_path = self.root / "harness" / "reports" / "tier-l1-dup.json"
        dup_data = {
            "schema_version": 1,
            "tier": "L1",
            "status": "PASSED",
            "checks": {"rust_static": "PASSED"},
            "source": ledger["source"],
            "timestamp_utc": "2026-11-23T00:00:00Z"
        }
        with open(dup_path, "w", encoding="utf-8") as f:
            json.dump(dup_data, f)

        manifest = evidence.aggregate_tier_results(str(self.root))
        self.assertEqual(manifest["decision"], "FAIL")
        self.assertIn("Duplicate tier artifact detected", manifest["decision_reason"])

    def test_tier_aggregation_fails_on_invalid_status_enum(self):
        """Trạng thái status không thuộc enum chuẩn phải bị từ chối"""
        ledger = evidence.create_evidence_ledger(str(self.root))
        evidence.save_ledger(str(self.root), ledger)

        tier_path = self.root / "harness" / "reports" / "tier-l1.json"
        bad_data = {
            "schema_version": 1,
            "tier": "L1",
            "status": "ARBITRARY_NON_ENUM",
            "checks": {"rust_static": "PASSED"},
            "source": ledger["source"],
            "timestamp_utc": "2026-11-23T00:00:00Z"
        }
        with open(tier_path, "w", encoding="utf-8") as f:
            json.dump(bad_data, f)

        manifest = evidence.aggregate_tier_results(str(self.root))
        self.assertEqual(manifest["decision"], "FAIL")
        self.assertIn("invalid status", manifest["decision_reason"])

    def test_validate_evidence_ledger_detects_tier_report_tampering_and_mismatch(self):
        """validate_evidence_ledger phát hiện checksum mismatch và source identity mismatch của tier artifact"""
        ledger = evidence.create_evidence_ledger(str(self.root))
        evidence.save_ledger(str(self.root), ledger)

        # Ghi nhận tier hợp lệ
        tier_path = evidence.record_tier_file(str(self.root), "L1", "PASSED")
        manifest = evidence.aggregate_tier_results(str(self.root))

        manifest_path = str(self.root / "harness" / "reports" / "evidence-manifest.json")
        ok, errors = evidence.validate_evidence_ledger(str(self.root), manifest_path)
        self.assertTrue(ok, f"Ban đầu phải hợp lệ: {errors}")

        # Giả mạo nội dung file tier trên đĩa (thay đổi checksum và head_sha)
        with open(tier_path, "r", encoding="utf-8") as f:
            tdata = json.load(f)
        tdata["source"]["head_sha"] = "1111111111111111111111111111111111111111"
        with open(tier_path, "w", encoding="utf-8") as f:
            json.dump(tdata, f)

        # Validate lại phải phát hiện sai lệch checksum và head_sha
        ok2, errors2 = evidence.validate_evidence_ledger(str(self.root), manifest_path)
        self.assertFalse(ok2)
        self.assertTrue(any("Checksum mismatch" in e or "Source identity mismatch" in e for e in errors2))

if __name__ == "__main__":
    unittest.main()
