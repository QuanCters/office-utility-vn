"""Self-tests proving source inventory cannot pass missing/placeholder cases."""
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from verify_source_test_inventory import check  # noqa: E402


class SourceInventoryTests(unittest.TestCase):
    def test_deleting_required_test_ids_fails(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            plan = root / "inventory.json"
            plan.write_text(json.dumps({"required_source_tests": [{"id":"FAKE", "file":"x.rs", "test_function":"x"}]}))
            self.assertTrue(any("deleted" in issue for issue in check(root, plan)))

    def test_placeholder_test_fails(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "verification.rs").write_text('''#[test]\nfn real_test() { panic!("NOT IMPLEMENTED"); }''')
            plan = root / "inventory.json"
            plan.write_text(json.dumps({"required_source_tests": [{"id":"IPC-ALL-TOOLS", "file":"verification.rs", "test_function":"real_test"}]}))
            self.assertTrue(any("placeholder" in issue for issue in check(root, plan)))

    def test_ignored_test_fails(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "verification.rs").write_text('#[test]\n#[ignore]\nfn real_test() {}')
            plan = root / "inventory.json"
            plan.write_text(json.dumps({"required_source_tests": [{"id":"IPC-ALL-TOOLS", "file":"verification.rs", "test_function":"real_test"}]}))
            self.assertTrue(any("placeholder" in issue for issue in check(root, plan)))


if __name__ == "__main__":
    unittest.main()
