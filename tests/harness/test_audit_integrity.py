"""
test_audit_integrity.py — Unit and Negative Tests for Audit Integrity Verifier.

Demonstrates that verify_audit_integrity.py strictly fails (nonzero exit code / passed=False)
when any audit invariant, summary count, readiness classification, evidence reference,
or source code registration is violated.
"""

import copy
import json
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from verify_audit_integrity import audit_integrity  # noqa: E402


class AuditIntegrityUnitAndNegativeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.inv_path = ROOT / "docs/engineering-baseline/tool_inventory.json"
        cls.findings_path = ROOT / "docs/engineering-baseline/findings_register.json"
        cls.manifest_path = ROOT / "docs/engineering-baseline/AUDIT_EVIDENCE_MANIFEST.json"
        cls.matrix_path = ROOT / "docs/engineering-baseline/VERIFICATION_COVERAGE_MATRIX.md"
        cls.registry_path = ROOT / "src/micro_apps/registry.rs"
        cls.commands_path = ROOT / "src-tauri/src/commands/mod.rs"

        cls.base_inventory = json.loads(cls.inv_path.read_text(encoding="utf-8"))
        cls.base_findings = json.loads(cls.findings_path.read_text(encoding="utf-8"))
        cls.base_manifest = json.loads(cls.manifest_path.read_text(encoding="utf-8"))
        cls.base_matrix_text = cls.matrix_path.read_text(encoding="utf-8")
        cls.base_registry_text = cls.registry_path.read_text(encoding="utf-8")
        cls.base_commands_text = cls.commands_path.read_text(encoding="utf-8")

    def test_baseline_passes_cleanly(self):
        """Sanity test: Authoritative unmodified repository data passes 100%."""
        report = audit_integrity(
            self.base_inventory,
            self.base_findings,
            self.base_manifest,
            self.base_matrix_text,
            self.base_registry_text,
            self.base_commands_text,
        )
        self.assertTrue(report.passed, f"Baseline should pass cleanly: {report.errors}")
        self.assertEqual(len(report.errors), 0)
        self.assertEqual(report.stats["total_tools"], 58)
        self.assertEqual(report.stats["total_findings"], 18)
        self.assertEqual(report.stats["source_registered_slugs_verified"], 58)
        expected_routed = 2 if "ENABLED_TOOL_ALLOWLIST" in self.base_commands_text else 8
        self.assertEqual(report.stats["source_routed_slugs_verified"], expected_routed)

    # -------------------------------------------------------------
    # Negative Test 1: Tool Count Mismatch (Summary / Array Length)
    # -------------------------------------------------------------
    def test_negative_tool_count_mismatch(self):
        inv = copy.deepcopy(self.base_inventory)
        inv.pop()  # Drop one tool -> 57 tools
        report = audit_integrity(inv, self.base_findings, self.base_manifest)
        self.assertFalse(report.passed)
        self.assertTrue(any("Tool inventory count mismatch" in e for e in report.errors))

    # -------------------------------------------------------------
    # Negative Test 2: Duplicate Tool Slug
    # -------------------------------------------------------------
    def test_negative_duplicate_tool_slug(self):
        inv = copy.deepcopy(self.base_inventory)
        inv[1]["slug"] = inv[0]["slug"]  # duplicate slug
        report = audit_integrity(inv, self.base_findings, self.base_manifest)
        self.assertFalse(report.passed)
        self.assertTrue(any("Duplicate tool slug" in e for e in report.errors))

    # -------------------------------------------------------------
    # Negative Test 3: Contradictory Routing vs Source Routed List
    # -------------------------------------------------------------
    def test_negative_contradictory_routing(self):
        inv = copy.deepcopy(self.base_inventory)
        # Mark an unrouted tool as ROUTED
        inv[1]["routing_state"] = "ROUTED"
        report = audit_integrity(inv, self.base_findings, self.base_manifest, commands_mod_rs_text=self.base_commands_text)
        self.assertFalse(report.passed)
        self.assertTrue(any("marked ROUTED in inventory but not in actual source match arms" in e for e in report.errors))

    # -------------------------------------------------------------
    # Negative Test 4: Contradictory Readiness vs Engine Type
    # -------------------------------------------------------------
    def test_negative_contradictory_readiness_and_engine(self):
        inv = copy.deepcopy(self.base_inventory)
        for t in inv:
            if t["engine_type"] == "FAKE_ENGINE":
                t["readiness_state"] = "UNROUTED_REAL"
                break
        report = audit_integrity(inv, self.base_findings, self.base_manifest)
        self.assertFalse(report.passed)
        self.assertTrue(any("Contradiction: Fake tool" in e for e in report.errors))

    # -------------------------------------------------------------
    # Negative Test 5: Tool Marked Release-Eligible without Evidence
    # -------------------------------------------------------------
    def test_negative_premature_release_eligibility(self):
        inv = copy.deepcopy(self.base_inventory)
        inv[0]["release_eligible"] = True
        report = audit_integrity(inv, self.base_findings, self.base_manifest)
        self.assertFalse(report.passed)
        self.assertTrue(any("Pre-certification violation" in e or "Safety Violation" in e for e in report.errors))

    # -------------------------------------------------------------
    # Negative Test 6: Fake/Unsafe Tool Falsely Marked Release-Eligible
    # -------------------------------------------------------------
    def test_negative_unsafe_tool_marked_release_eligible(self):
        inv = copy.deepcopy(self.base_inventory)
        for t in inv:
            if t["engine_type"] == "UNSAFE_PDF":
                t["release_eligible"] = True
                break
        report = audit_integrity(inv, self.base_findings, self.base_manifest)
        self.assertFalse(report.passed)
        self.assertTrue(any("Safety Violation: Fake/Unsafe tool" in e for e in report.errors))

    # -------------------------------------------------------------
    # Negative Test 7: Findings Summary Mismatch
    # -------------------------------------------------------------
    def test_negative_findings_summary_count_mismatch(self):
        findings_data = copy.deepcopy(self.base_findings)
        findings_data["summary"]["total_findings"] = 999  # false total
        report = audit_integrity(self.base_inventory, findings_data, self.base_manifest)
        self.assertFalse(report.passed)
        self.assertTrue(any("Findings summary count mismatch" in e for e in report.errors))

    # -------------------------------------------------------------
    # Negative Test 8: Finding Severity Count Mismatch
    # -------------------------------------------------------------
    def test_negative_finding_severity_count_mismatch(self):
        findings_data = copy.deepcopy(self.base_findings)
        findings_data["summary"]["p0_count"] = 0  # false P0 count
        report = audit_integrity(self.base_inventory, findings_data, self.base_manifest)
        self.assertFalse(report.passed)
        self.assertTrue(any("P0 count mismatch" in e for e in report.errors))

    # -------------------------------------------------------------
    # Negative Test 9: Duplicate Finding ID
    # -------------------------------------------------------------
    def test_negative_duplicate_finding_id(self):
        findings_data = copy.deepcopy(self.base_findings)
        findings_data["findings"].append(copy.deepcopy(findings_data["findings"][0]))
        findings_data["summary"]["total_findings"] += 1
        findings_data["summary"]["p0_count"] += 1
        report = audit_integrity(self.base_inventory, findings_data, self.base_manifest)
        self.assertFalse(report.passed)
        self.assertTrue(any("Duplicate finding ID" in e for e in report.errors))

    # -------------------------------------------------------------
    # Negative Test 10: Evidence Pointing to Nonexistent Finding
    # -------------------------------------------------------------
    def test_negative_dangling_evidence_reference(self):
        manifest_data = copy.deepcopy(self.base_manifest)
        manifest_data["findings_evidence"].append({
            "id": "FINDING-P0-NONEXISTENT-999",
            "severity": "P0",
            "evidence_state": "CONFIRMED_STATIC",
        })
        report = audit_integrity(self.base_inventory, self.base_findings, manifest_data)
        self.assertFalse(report.passed)
        self.assertTrue(any("Dangling evidence" in e for e in report.errors))

    # -------------------------------------------------------------
    # Negative Test 11: Missing Evidence for Finding
    # -------------------------------------------------------------
    def test_negative_missing_evidence_for_finding(self):
        manifest_data = copy.deepcopy(self.base_manifest)
        manifest_data["findings_evidence"].pop()  # Drop one evidence entry
        report = audit_integrity(self.base_inventory, self.base_findings, manifest_data)
        self.assertFalse(report.passed)
        self.assertTrue(any("Missing evidence: finding" in e for e in report.errors))

    # -------------------------------------------------------------
    # Negative Test 12: Source Registration vs Inventory Mismatch
    # -------------------------------------------------------------
    def test_negative_source_registration_mismatch(self):
        # Simulate a tool added to registry.rs that is not in inventory
        simulated_registry = self.base_registry_text + '\nslug: "brand-new-untracked-tool"\n'
        report = audit_integrity(
            self.base_inventory,
            self.base_findings,
            self.base_manifest,
            registry_rs_text=simulated_registry,
        )
        self.assertFalse(report.passed)
        self.assertTrue(any("missing in inventory" in e or "Source registration mismatch" in e for e in report.errors))

    # -------------------------------------------------------------
    # Negative Test 13: CLI Execution Clean Exit 0
    # -------------------------------------------------------------
    def test_cli_execution_clean_exit_zero(self):
        proc = subprocess.run(
            [sys.executable, str(ROOT / "scripts/verify_audit_integrity.py"), "--repo", str(ROOT)],
            capture_output=True,
            text=True,
        )
        self.assertEqual(proc.returncode, 0, f"CLI should exit with code 0: {proc.stderr}")
        self.assertIn("TẤT CẢ DỮ LIỆU KIỂM TOÁN VÀ SỔ BỘ ĐẠT CHUẨN ĐỒNG BỘ 100%", proc.stdout)
        self.assertIn("Đã đối soát trực tiếp mã nguồn Leptos Registry: 58 slugs", proc.stdout)


if __name__ == "__main__":
    unittest.main()
