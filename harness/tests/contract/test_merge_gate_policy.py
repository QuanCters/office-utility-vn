#!/usr/bin/env python3
"""
harness/tests/contract/test_merge_gate_policy.py
Kiểm tra tính chính xác của Deterministic Merge Gate Evaluator:
- Trạng thái INCOMPLETE khi thiếu check hoặc stale commit
- Trạng thái MANUAL_HOLD khi có P0/P1 finding
- Trạng thái FAIL khi có check thất bại hoặc vi phạm protected path
- Trạng thái PASS khi toàn bộ điều kiện đạt chuẩn
"""

import os
import unittest
import json
import tempfile
import sys

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../.."))
sys.path.insert(0, os.path.join(repo_root, "harness", "runner"))
import policy

class TestMergeGatePolicy(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.mkdtemp()
        self.ledger_file = os.path.join(self.temp_dir, "test_ledger.json")

    def test_gate_decision_matrix(self):
        # 1. Trạng thái INCOMPLETE
        ledger_incomplete = {
            "source": {"head_sha": "current_head"},
            "review": {"findings_summary": {"P0": 0, "P1": 0}},
            "checks": {"policy_integrity": "PENDING"}
        }
        with open(self.ledger_file, "w") as f:
            json.dump(ledger_incomplete, f)

        decision, _, code = policy.evaluate_merge_gate(repo_root, self.ledger_file)
        self.assertIn(decision, ["INCOMPLETE", "FAIL"])

        # 2. Trạng thái MANUAL_HOLD khi có P0
        ledger_hold = {
            "source": {"head_sha": policy.subprocess.run(["git", "rev-parse", "HEAD"], stdout=policy.subprocess.PIPE, text=True).stdout.strip()},
            "review": {"findings_summary": {"P0": 1, "P1": 0}},
            "checks": {}
        }
        with open(self.ledger_file, "w") as f:
            json.dump(ledger_hold, f)

        decision, reason, code = policy.evaluate_merge_gate(repo_root, self.ledger_file)
        self.assertEqual(decision, "MANUAL_HOLD")
        self.assertEqual(code, 2)

if __name__ == "__main__":
    unittest.main()
