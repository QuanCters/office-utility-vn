#!/usr/bin/env python3
"""
tests/harness/test_protected_paths_ci.py
Kiểm thử tự động chính sách đường dẫn bất biến và phân tích rủi ro trong môi trường CI (Blocker 3):
- Kiểm thử phát hiện thay đổi trong checkout sạch thông qua GITHUB_BASE_REF
- Kiểm thử phân loại rủi ro (classify_risk) cho các file nhạy cảm:
  + src-tauri/src/commands/mod.rs (CRITICAL: IPC capability/boundary)
  + scripts/verify_spine_lock.sh (CRITICAL: Spine lock / verification harness)
  + harness/policies/ & harness/runner/ (CRITICAL: Spine lock / verification harness)
  + vendor/ (CRITICAL: Vendor pinned code)
  + .github/workflows/ (CRITICAL: CI workflow)
- Kiểm thử phát hiện vi phạm Protected Paths trong lịch sử commit ngay cả khi working tree sạch
- Kiểm thử cơ chế ủy quyền sửa đổi Spine (ALLOW_SPINE_AMENDMENT=1)
"""

import os
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "harness" / "runner"))

import policy

class ProtectedPathsCiTests(unittest.TestCase):
    def test_classify_risk_detects_all_protected_boundaries(self):
        """Phân loại mức độ rủi ro CRITICAL cho mọi thay đổi thuộc vùng nhạy cảm"""
        test_cases = [
            ("src-tauri/src/commands/mod.rs", "CRITICAL", "IPC capability/boundary touched"),
            ("scripts/verify_spine_lock.sh", "CRITICAL", "Spine lock / verification harness touched"),
            ("harness/policies/quality-gates.yaml", "CRITICAL", "Spine lock / verification harness touched"),
            ("harness/runner/evidence.py", "CRITICAL", "Spine lock / verification harness touched"),
            ("vendor/third_party/lib.rs", "CRITICAL", "Vendor pinned code touched"),
            (".github/workflows/verify-pr.yml", "CRITICAL", "CI workflow touched"),
            ("src-tauri/src/db/mod.rs", "HIGH", "Persistence layer touched"),
        ]

        for filepath, expected_risk, expected_reason_keyword in test_cases:
            with patch.object(policy, "get_git_diff_files", return_value=[filepath]):
                risk_info = policy.classify_risk(str(REPO_ROOT))
                self.assertEqual(
                    risk_info["risk_level"],
                    expected_risk,
                    f"Tệp {filepath} phải được phân loại là {expected_risk}"
                )
                self.assertTrue(
                    any(expected_reason_keyword in r for r in risk_info["reasons"]),
                    f"Lý do phân loại của {filepath} phải chứa '{expected_reason_keyword}': {risk_info['reasons']}"
                )
                self.assertIn("L0", risk_info["required_tiers"])
                self.assertIn("L3", risk_info["required_tiers"])

    def test_clean_ci_checkout_diff_detection_via_base_ref(self):
        """Kiểm chứng get_git_diff_files sử dụng GITHUB_BASE_REF khi working tree sạch"""
        os.environ["GITHUB_BASE_REF"] = "baseline/gridcraft-prerequisites"
        try:
            diff_files = policy.get_git_diff_files(str(REPO_ROOT))
            # Trong worktree của PR-01, các commit PR-01 đã sửa đổi commands/mod.rs và tests/
            # Hàm phải phát hiện ra mà không trả về mảng rỗng []
            self.assertTrue(
                len(diff_files) > 0,
                "get_git_diff_files không được trả về danh sách rỗng khi so sánh với GITHUB_BASE_REF"
            )
            self.assertTrue(
                any("src-tauri/src/commands/mod.rs" in f for f in diff_files),
                "Phải phát hiện src-tauri/src/commands/mod.rs đã thay đổi giữa base và HEAD"
            )
        finally:
            os.environ.pop("GITHUB_BASE_REF", None)

    def test_protected_paths_guard_flags_violations_without_authorization(self):
        """Khi có thay đổi vào protected paths mà không có ủy quyền, check_protected_paths phải trả về False"""
        sensitive_files = [
            "src-tauri/src/commands/mod.rs",
            "scripts/verify_spine_lock.sh",
            "harness/policies/quality-gates.yaml",
            "vendor/some_crate/src/lib.rs",
            ".github/workflows/verify-pr.yml",
        ]

        for s_file in sensitive_files:
            with patch.object(policy, "get_git_diff_files", return_value=[s_file]):
                # Chưa cấp quyền Spine Amendment
                os.environ.pop("ALLOW_SPINE_AMENDMENT", None)
                os.environ.pop("SPINE_AMENDMENT_AUTHORIZED", None)
                ok, violations = policy.check_protected_paths(str(REPO_ROOT))

                self.assertFalse(ok, f"Phải chặn sửa đổi trái phép vào {s_file}")
                self.assertTrue(
                    any(s_file in v for v in violations),
                    f"Vi phạm phải nêu rõ tệp {s_file}: {violations}"
                )

    def test_protected_paths_guard_allows_authorized_spine_amendment(self):
        """Khi có cờ ALLOW_SPINE_AMENDMENT=1, chính sách cho phép thay đổi có kiểm soát"""
        with patch.object(policy, "get_git_diff_files", return_value=["src-tauri/src/commands/mod.rs"]):
            os.environ["ALLOW_SPINE_AMENDMENT"] = "1"
            try:
                ok, violations = policy.check_protected_paths(str(REPO_ROOT))
                self.assertTrue(ok, f"Khi đã ủy quyền Spine Amendment, check_protected_paths phải thông qua: {violations}")
            finally:
                os.environ.pop("ALLOW_SPINE_AMENDMENT", None)

if __name__ == "__main__":
    unittest.main()
