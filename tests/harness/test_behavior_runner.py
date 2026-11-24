"""Tests runner failure semantics. Real Engine correctness requires production adapter."""
import hashlib
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from verify_behavior import VerificationError, validate_manifest, sha256, check_output  # noqa: E402


class BehavioralRunnerSafetyTests(unittest.TestCase):
    def test_missing_cases_fail(self):
        with self.assertRaises(VerificationError):
            validate_manifest([], Path("."))

    def test_todo_case_fails_even_if_file_exists(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "test.bin").write_bytes(b"test")
            cases = [{"id": "X", "state": "TODO", "tool_slug": "one", "expect": "ERROR",
                      "inputs": [{"path": "test.bin", "sha256": sha256(root / 'test.bin')}],
                      "error_code": "INVALID"}]
            with self.assertRaisesRegex(VerificationError, "TODO"):
                validate_manifest(cases, root)

    def test_fake_placeholder_hash_fails(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "test.bin").write_bytes(b"test")
            cases = [{"id": "X", "state": "ready", "tool_slug": "one", "expect": "ERROR",
                      "inputs": [{"path": "test.bin", "sha256": "0" * 64}],
                      "error_code": "INVALID"}]
            with self.assertRaisesRegex(VerificationError, "SHA256 mismatch"):
                validate_manifest(cases, root)

    def test_deleting_mandatory_cases_cannot_make_release_pass(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "test.bin").write_bytes(b"test")
            case = {"id": "CUSTOM", "state": "ready", "tool_slug": "one", "expect": "ERROR",
                    "inputs": [{"path": "test.bin", "sha256": sha256(root / "test.bin")}],
                    "error_code": "INVALID"}
            with self.assertRaisesRegex(VerificationError, "Missing mandatory case"):
                validate_manifest([case], root)

    def test_empty_output_rejected(self):
        with tempfile.TemporaryDirectory() as folder:
            out = Path(folder) / 'sample.txt'
            out.touch()
            with self.assertRaisesRegex(VerificationError, "empty"):
                check_output(out, {"type": "text", "tokens": []}, "TEXT")

    def test_text_content_oracle_rejects_wrong_output(self):
        with tempfile.TemporaryDirectory() as folder:
            out = Path(folder) / 'sample.txt'
            out.write_text("Sai nội dung", encoding="utf-8")
            with self.assertRaisesRegex(VerificationError, "missing text token"):
                check_output(out, {"type": "text", "tokens": ["HÓA ĐƠN"]}, "TEXT")


if __name__ == "__main__":
    unittest.main()
