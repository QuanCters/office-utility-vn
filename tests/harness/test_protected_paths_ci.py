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
        """Khi có cờ ALLOW_SPINE_AMENDMENT=1, chính sách cho phép thay đổi commands/mod.rs và scripts/verify_spine_lock.sh"""
        for approved_file in ["src-tauri/src/commands/mod.rs", "scripts/verify_spine_lock.sh"]:
            with patch.object(policy, "get_git_diff_files", return_value=[approved_file]):
                os.environ["ALLOW_SPINE_AMENDMENT"] = "1"
                try:
                    ok, violations = policy.check_protected_paths(str(REPO_ROOT))
                    self.assertTrue(ok, f"Khi đã ủy quyền Spine Amendment, {approved_file} phải được thông qua: {violations}")
                finally:
                    os.environ.pop("ALLOW_SPINE_AMENDMENT", None)

    def test_protected_paths_guard_blocks_unauthorized_spine_even_with_amendment(self):
        """Khi có cờ ALLOW_SPINE_AMENDMENT=1, các tệp spine khác (shared, db, other commands) và vendor/CODEOWNERS vẫn BẮT BUỘC bị chặn"""
        blocked_files = [
            "crates/office-shared/src/lib.rs",
            "src-tauri/src/db/mod.rs",
            "src-tauri/src/commands/unauthorized.rs",
            "vendor/foo/lib.rs",
            ".github/CODEOWNERS",
        ]
        os.environ["ALLOW_SPINE_AMENDMENT"] = "1"
        try:
            for b_file in blocked_files:
                with patch.object(policy, "get_git_diff_files", return_value=[b_file]):
                    ok, violations = policy.check_protected_paths(str(REPO_ROOT))
                    self.assertFalse(ok, f"Tệp {b_file} không được phép tu chính ngay cả khi có ALLOW_SPINE_AMENDMENT=1")
                    self.assertTrue(
                        any(b_file in v for v in violations),
                        f"Vi phạm phải ghi rõ {b_file}: {violations}"
                    )
        finally:
            os.environ.pop("ALLOW_SPINE_AMENDMENT", None)

    def test_ci_fail_closed_diff_detection(self):
        """Trong môi trường CI (GITHUB_ACTIONS=true), nếu diff rỗng thì phải fail-closed (RuntimeError / Policy Violation)"""
        with patch.dict(os.environ, {"GITHUB_ACTIONS": "true", "GITHUB_BASE_REF": "main"}):
            with patch("subprocess.run") as mock_run:
                # Giả lập git diff trả về rỗng
                mock_run.return_value.returncode = 0
                mock_run.return_value.stdout = ""
                with self.assertRaises(RuntimeError):
                    policy.get_git_diff_files(str(REPO_ROOT))

                ok, violations = policy.check_protected_paths(str(REPO_ROOT))
                self.assertFalse(ok)
                self.assertTrue(any("Fail-closed" in v for v in violations))

    def test_protected_paths_guard_allows_authorized_harness_amendment(self):
        """Khi có cờ ALLOW_HARNESS_AMENDMENT=1, chỉ các tệp nằm trong danh mục AUTHORIZED_HARNESS_AMENDMENT_FILES mới được thông qua"""
        approved_harness_files = [
            "harness/runner/evidence.py",
            "harness/ci/verify-core.sh",
            ".github/workflows/verify-pr.yml",
        ]
        os.environ["ALLOW_HARNESS_AMENDMENT"] = "1"
        try:
            for h_file in approved_harness_files:
                with patch.object(policy, "get_git_diff_files", return_value=[h_file]):
                    ok, violations = policy.check_protected_paths(str(REPO_ROOT))
                    self.assertTrue(ok, f"Tệp {h_file} trong danh mục cho phép phải được chấp thuận: {violations}")
        finally:
            os.environ.pop("ALLOW_HARNESS_AMENDMENT", None)

    def test_protected_paths_guard_blocks_unauthorized_harness_amendment(self):
        """Khi có cờ ALLOW_HARNESS_AMENDMENT=1, các tệp harness/CI ngoài danh mục phê duyệt BẮT BUỘC bị chặn"""
        unauthorized_harness_files = [
            "harness/runner/rogue_exploit.py",
            ".github/workflows/malicious.yml",
            "harness/ci/unapproved_script.sh",
        ]
        os.environ["ALLOW_HARNESS_AMENDMENT"] = "1"
        try:
            for u_file in unauthorized_harness_files:
                with patch.object(policy, "get_git_diff_files", return_value=[u_file]):
                    ok, violations = policy.check_protected_paths(str(REPO_ROOT))
                    self.assertFalse(ok, f"Tệp {u_file} ngoài danh mục phê duyệt phải bị chặn")
                    self.assertTrue(
                        any(u_file in v for v in violations),
                        f"Thông báo vi phạm phải chứa {u_file}: {violations}"
                    )
        finally:
            os.environ.pop("ALLOW_HARNESS_AMENDMENT", None)

    def test_spine_amendment_alone_does_not_permit_harness_modifications(self):
        """Cờ ALLOW_SPINE_AMENDMENT=1 không bao giờ được phép dùng làm thẻ bài thông hành cho harness/CI"""
        os.environ["ALLOW_SPINE_AMENDMENT"] = "1"
        os.environ.pop("ALLOW_HARNESS_AMENDMENT", None)
        try:
            with patch.object(policy, "get_git_diff_files", return_value=["harness/runner/evidence.py"]):
                ok, violations = policy.check_protected_paths(str(REPO_ROOT))
                self.assertFalse(ok, "ALLOW_SPINE_AMENDMENT=1 không được phép thông qua sửa đổi harness")
                self.assertTrue(any("harness/runner/evidence.py" in v for v in violations))
        finally:
            os.environ.pop("ALLOW_SPINE_AMENDMENT", None)

    def test_harness_amendment_alone_does_not_permit_spine_modifications(self):
        """Cờ ALLOW_HARNESS_AMENDMENT=1 không bao giờ được phép thông qua sửa đổi Spine Layer"""
        os.environ["ALLOW_HARNESS_AMENDMENT"] = "1"
        os.environ.pop("ALLOW_SPINE_AMENDMENT", None)
        try:
            with patch.object(policy, "get_git_diff_files", return_value=["src-tauri/src/commands/mod.rs"]):
                ok, violations = policy.check_protected_paths(str(REPO_ROOT))
                self.assertFalse(ok, "ALLOW_HARNESS_AMENDMENT=1 không được phép thông qua sửa đổi Spine")
                self.assertTrue(any("src-tauri/src/commands/mod.rs" in v for v in violations))
        finally:
            os.environ.pop("ALLOW_HARNESS_AMENDMENT", None)

if __name__ == "__main__":
    unittest.main()
