"""Proves incremental gate blocks new findings, rejects stale waivers and requires metadata."""
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from verify_guardrails import load_policy, scan_repo  # noqa: E402


class BaselinePolicyTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.source = self.root / 'crates/office-engine/src/doc_suite/pdf_merge_split.rs'
        self.source.parent.mkdir(parents=True)
        self.policy = self.root / 'guardrails.json'
        base = json.loads((ROOT / 'policy/guardrails.json').read_text())
        base['required_source_paths'] = []  # only isolate waiver logic in this unit test
        self.policy.write_text(json.dumps(base), encoding='utf-8')
        self.baseline = self.root / 'baseline.json'
        self.baseline.write_text(json.dumps({'approved_findings': [], 'expires_on': '2027-01-31'}), encoding='utf-8')

    def run_incremental(self):
        return subprocess.run([sys.executable, str(ROOT / 'scripts/verify_guardrails.py'),
                               '--repo', str(self.root), '--policy', str(self.policy),
                               '--baseline', str(self.baseline), '--mode', 'incremental'],
                              capture_output=True, text=True)

    def approve_current_legacy(self):
        finding = scan_repo(self.root, load_policy(self.policy), require_layout=False)[0]
        self.baseline.write_text(json.dumps({'expires_on': '2027-01-31',
            'approved_findings': [{'fingerprint': finding.fingerprint, 'ticket': 'SEC-123',
                                   'owner': 'approved-owner', 'reason': 'preexisting fix tracked'}]}),
            encoding='utf-8')

    def test_new_violation_blocks(self):
        self.source.write_text('let raw = String::from_utf8_lossy(bytes);', encoding='utf-8')
        self.assertEqual(1, self.run_incremental().returncode)

    def test_only_approved_unchanged_legacy_passes(self):
        self.source.write_text('let raw = String::from_utf8_lossy(bytes);', encoding='utf-8')
        self.approve_current_legacy()
        self.assertEqual(0, self.run_incremental().returncode)
        self.source.write_text('let raw = String::from_utf8_lossy(bytes);\nlet other = raw.find("endobj");')
        self.assertEqual(1, self.run_incremental().returncode)

    def test_stale_waiver_blocks(self):
        self.source.write_text('let raw = String::from_utf8_lossy(bytes);', encoding='utf-8')
        self.approve_current_legacy()
        self.source.write_text('let doc = lopdf::Document::load_mem(bytes)?;')
        self.assertEqual(1, self.run_incremental().returncode)

    def test_unapproved_placeholder_waiver_blocks(self):
        self.source.write_text('let raw = String::from_utf8_lossy(bytes);', encoding='utf-8')
        self.approve_current_legacy()
        base = json.loads(self.baseline.read_text())
        base['approved_findings'][0]['ticket'] = 'REQUIRES_HUMAN_TICKET'
        self.baseline.write_text(json.dumps(base))
        self.assertEqual(1, self.run_incremental().returncode)


if __name__ == '__main__':
    unittest.main()
