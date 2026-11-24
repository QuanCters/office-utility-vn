#!/usr/bin/env python3
"""
Executable Negative Tests for Fail-Closed Vendor and Artifact Verification (P2 #4678429571).
Verifies that:
1. Modified pinned vendor source fails verification.
2. Modified vendor patch fails verification.
3. Stale bundle / manifest mismatch fails verification.
4. Missing required artifact fails verification.
5. Artifact checksum mismatch fails verification.
6. Verifier CLI fails closed (exit code 1) on errors.
"""

import os
import sys
import tempfile
import shutil
import subprocess

REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
sys.path.insert(0, REPO_ROOT)

from harness.runner import evidence

def test_modified_vendor_source():
    print("Testing Case 1: Modified pinned vendor source fails verification...")
    host_rs = os.path.join(REPO_ROOT, "vendor", "gridcraft", "apps", "gridcraft-web", "src", "host.rs")
    with open(host_rs, "r", encoding="utf-8") as f:
        original_src = f.read()

    try:
        # Strip confirmed save protocol from host.rs
        tampered_src = original_src.replace('Some("confirmed")', 'Some("unconfirmed")')
        with open(host_rs, "w", encoding="utf-8") as f:
            f.write(tampered_src)

        ok, errs = evidence.verify_vendor_pin(REPO_ROOT)
        assert not ok, "Verification MUST fail when vendor source is modified!"
        assert any("confirmed save" in e for e in errs) or any("reverse dry-run" in e for e in errs), f"Expected confirmed save or dry-run error, got: {errs}"
        print("  ✓ PASSED: Tampered vendor source was rejected fail-closed.")
    finally:
        with open(host_rs, "w", encoding="utf-8") as f:
            f.write(original_src)

def test_modified_vendor_patch():
    print("Testing Case 2: Modified vendor patch fails verification...")
    patch_path = os.path.join(REPO_ROOT, "patches", "0001-gridcraft-confirmed-save.patch")
    with open(patch_path, "rb") as f:
        original_bytes = f.read()

    try:
        # Append a byte to tamper with checksum and patch content
        with open(patch_path, "wb") as f:
            f.write(original_bytes + b"\n# TAMPERED LINE\n")

        ok, errs = evidence.verify_vendor_pin(REPO_ROOT)
        assert not ok, "Verification MUST fail when vendor patch is modified!"
        assert any("checksum mismatch" in e for e in errs), f"Expected patch checksum mismatch, got: {errs}"
        print("  ✓ PASSED: Tampered vendor patch was rejected fail-closed.")
    finally:
        with open(patch_path, "wb") as f:
            f.write(original_bytes)

def test_missing_required_artifact():
    print("Testing Case 3: Missing required artifact fails verification...")
    wasm_path = os.path.join(REPO_ROOT, "public", "gridcraft", "gridcraft-web-18eda47c1aa96e20_bg.wasm")
    bak_path = wasm_path + ".bak"

    try:
        os.rename(wasm_path, bak_path)
        ok, errs = evidence.verify_artifacts(REPO_ROOT)
        assert not ok, "Verification MUST fail when required artifact is missing!"
        assert any("Missing required artifact" in e for e in errs), f"Expected missing artifact error, got: {errs}"
        print("  ✓ PASSED: Missing artifact was detected and failed closed.")
    finally:
        if os.path.exists(bak_path):
            os.rename(bak_path, wasm_path)

def test_artifact_checksum_mismatch():
    print("Testing Case 4: Artifact checksum mismatch fails verification...")
    html_path = os.path.join(REPO_ROOT, "public", "gridcraft", "index.html")
    with open(html_path, "rb") as f:
        original_bytes = f.read()

    try:
        with open(html_path, "wb") as f:
            f.write(original_bytes + b"<!-- CORRUPT -->")

        ok, errs = evidence.verify_artifacts(REPO_ROOT)
        assert not ok, "Verification MUST fail when artifact checksum mismatches!"
        assert any("checksum mismatch" in e for e in errs), f"Expected checksum mismatch, got: {errs}"
        print("  ✓ PASSED: Corrupted artifact checksum was detected and failed closed.")
    finally:
        with open(html_path, "wb") as f:
            f.write(original_bytes)

def test_stale_bundle_manifest_mismatch():
    print("Testing Case 5: Stale bundle / manifest mismatch fails verification...")
    manifest_path = os.path.join(REPO_ROOT, "vendor", "gridcraft", "PROVENANCE.json")
    with open(manifest_path, "r", encoding="utf-8") as f:
        original_json = f.read()

    try:
        tampered_json = original_json.replace("519de3fc782bf5f7394fa13a19b21fefd91c30bd4c7483ae1f8aa0d166d41169", "0000000000000000000000000000000000000000000000000000000000000000")
        with open(manifest_path, "w", encoding="utf-8") as f:
            f.write(tampered_json)

        ok, errs = evidence.verify_artifacts(REPO_ROOT)
        assert not ok, "Verification MUST fail on stale manifest SHA!"
        assert any("checksum mismatch" in e for e in errs), f"Expected checksum mismatch, got: {errs}"
        print("  ✓ PASSED: Stale manifest checksum was detected and failed closed.")
    finally:
        with open(manifest_path, "w", encoding="utf-8") as f:
            f.write(original_json)

def test_verifier_cli_fail_closed():
    print("Testing Case 6: Verifier CLI fail-closed exit code on failure...")
    patch_path = os.path.join(REPO_ROOT, "patches", "0001-gridcraft-confirmed-save.patch")
    with open(patch_path, "rb") as f:
        original_bytes = f.read()

    try:
        with open(patch_path, "wb") as f:
            f.write(original_bytes + b"\n# TAMPER\n")

        res = subprocess.run(
            [sys.executable, os.path.join(REPO_ROOT, "harness", "runner", "run.py"), "--check-vendor-pin"],
            capture_output=True,
            cwd=REPO_ROOT,
        )
        assert res.returncode != 0, f"CLI MUST exit with non-zero on verification failure, got {res.returncode}"
        print("  ✓ PASSED: CLI exited with non-zero code on tampered state.")
    finally:
        with open(patch_path, "wb") as f:
            f.write(original_bytes)

def main():
    print("================================================================")
    print("🧪 Running Executable Negative Verification Tests (P2 #4678429571)")
    print("================================================================")
    test_modified_vendor_source()
    test_modified_vendor_patch()
    test_missing_required_artifact()
    test_artifact_checksum_mismatch()
    test_stale_bundle_manifest_mismatch()
    test_verifier_cli_fail_closed()
    print("\n🎉 ALL 6 NEGATIVE VERIFICATION TESTS PASSED CLEANLY (FAIL-CLOSED VERIFIED)!")

if __name__ == "__main__":
    main()
