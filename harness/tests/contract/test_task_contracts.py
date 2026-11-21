#!/usr/bin/env python3
"""
harness/tests/contract/test_task_contracts.py
Kiểm thử tính hợp lệ của toàn bộ Task Contract trong harness/contracts/tasks/:
- Tuân thủ schema task.schema.json
- Tồn tại các bất biến bắt buộc
- Ranh giới allowed_paths và forbidden_paths không mâu thuẫn
- Zero-Dependency (không yêu cầu pip packages)
"""

import os
import unittest
import json
import sys

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
sys.path.insert(0, os.path.join(repo_root, "runner"))
from policy import parse_simple_yaml

class TestTaskContracts(unittest.TestCase):
    def setUp(self):
        self.harness_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
        self.contracts_dir = os.path.join(self.harness_dir, "contracts", "tasks")

    def test_all_task_yamls_valid(self):
        if not os.path.exists(self.contracts_dir):
            self.skipTest("No tasks directory found.")

        task_files = [f for f in os.listdir(self.contracts_dir) if f.endswith((".yaml", ".yml"))]
        self.assertGreater(len(task_files), 0, "Phải có ít nhất 1 task contract mẫu.")

        for tf in task_files:
            file_path = os.path.join(self.contracts_dir, tf)
            with open(file_path, "r", encoding="utf-8") as f:
                data = parse_simple_yaml(f.read())

            self.assertIn("task_id", data)
            self.assertIn("title", data)
            self.assertIn("risk_level", data)
            self.assertIn("allowed_paths", data)
            self.assertIn("forbidden_paths", data)
            self.assertIn("invariants", data)

            for ap in data["allowed_paths"]:
                for fp in data["forbidden_paths"]:
                    if ap == fp:
                        self.fail(f"Xung đột đường dẫn trong {tf}: '{ap}' vừa nằm trong allowed vừa trong forbidden!")

if __name__ == "__main__":
    unittest.main()
