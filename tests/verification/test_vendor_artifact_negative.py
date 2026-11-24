#!/usr/bin/env python3
"""
Executable Negative Tests for Fail-Closed Vendor and Artifact Verification (P2 #4678429571 & Corrections A, B).
Verifies that:
1. Modification of a vendor source file outside the existing patch fails verification.
2. Modification of a dependency lockfile fails verification.
3. Changed source with an otherwise intact old bundle fails verification.
4. Missing or corrupted output asset in dist fails verification.
5. Verifier CLI false-success regression is prevented (exits non-zero on failure, zero on success).
6. Modified vendor patch fails verification.
"""

import os
import sys
import tempfile
import shutil
import subprocess

REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
sys.path.insert(0, REPO_ROOT)

from harness.runner import evidence

def test_modified_vendor_source_outside_patch():
    print("Testing Case 1: Modification of a vendor source file outside the existing patch...")
    # Use a vendor file that is NOT touched by 0001 patch (e.g. apps/gridcraft-cli/src/main.rs)
    target_file = os.path.join(REPO_ROOT, "vendor", "gridcraft", "apps", "gridcraft-cli", "src", "main.rs")
    with open(target_file, "r", encoding="utf-8") as f:
        original_src = f.read()

    try:
        with open(target_file, "w", encoding="utf-8") as f:
            f.write(original_src + "\n// UNAUTHORIZED MODIFICATION OUTSIDE PATCH\n")

        ok, errs = evidence.verify_vendor_pin(REPO_ROOT)
        assert not ok, "Verification MUST fail when vendor source outside patch is modified!"
        assert any("source-tree fingerprint mismatch" in e for e in errs), f"Expected source-tree fingerprint mismatch, got: {errs}"
        print("  ✓ PASSED: Untracked modification outside patch was rejected by source-tree fingerprint.")
    finally:
        with open(target_file, "w", encoding="utf-8") as f:
            f.write(original_src)

def test_modified_dependency_lockfile():
    print("Testing Case 2: Modification of a dependency lockfile...")
    lock_file = os.path.join(REPO_ROOT, "vendor", "gridcraft", "Cargo.lock")
    with open(lock_file, "r", encoding="utf-8") as f:
        original_lock = f.read()

    try:
        with open(lock_file, "w", encoding="utf-8") as f:
            f.write(original_lock + "\n# UNPINNED DEPENDENCY TAMPER\n")

        ok, errs = evidence.verify_vendor_pin(REPO_ROOT)
        assert not ok, "Verification MUST fail when vendor dependency lockfile is modified!"
        assert any("lockfile checksum mismatch" in e for e in errs), f"Expected lockfile checksum mismatch, got: {errs}"
        print("  ✓ PASSED: Modified lockfile was rejected fail-closed.")
    finally:
        with open(lock_file, "w", encoding="utf-8") as f:
            f.write(original_lock)

def test_changed_source_with_intact_old_bundle():
    print("Testing Case 3: Changed source with an otherwise intact old bundle...")
    prefs_file = os.path.join(REPO_ROOT, "vendor", "gridcraft", "apps", "gridcraft-web", "src", "prefs.rs")
    with open(prefs_file, "r", encoding="utf-8") as f:
        original_prefs = f.read()

    try:
        with open(prefs_file, "w", encoding="utf-8") as f:
            f.write(original_prefs + "\n// CHANGED SOURCE WITHOUT REBUILD\n")

        # Even if public/gridcraft artifacts are completely valid, pre-build / source verification MUST fail!
        ok, errs = evidence.verify_vendor_pin(REPO_ROOT)
        assert not ok, "Verification MUST fail when source changed without updating provenance!"
        assert any("source-tree fingerprint mismatch" in e for e in errs), f"Expected tree mismatch, got: {errs}"
        print("  ✓ PASSED: Changed source with old bundle was caught by source provenance.")
    finally:
        with open(prefs_file, "w", encoding="utf-8") as f:
            f.write(original_prefs)

def test_missing_or_corrupted_dist_artifact():
    print("Testing Case 4: Missing or corrupted output asset in dist...")
    dist_wasm = os.path.join(REPO_ROOT, "dist", "gridcraft", "gridcraft-web-8ec302fc82564728_bg.wasm")
    dist_bak = dist_wasm + ".bak"

    # Ensure dist exists
    os.makedirs(os.path.dirname(dist_wasm), exist_ok=True)
    if not os.path.exists(dist_wasm):
        # copy from public
        src_wasm = os.path.join(REPO_ROOT, "public", "gridcraft", "gridcraft-web-8ec302fc82564728_bg.wasm")
        shutil.copy2(src_wasm, dist_wasm)

    try:
        # Test 4a: Missing dist artifact
        os.rename(dist_wasm, dist_bak)
        ok, errs = evidence.verify_artifacts(REPO_ROOT, check_dist=True)
        assert not ok, "Verification MUST fail when dist artifact is missing!"
        assert any("Missing required dist artifact" in e for e in errs), f"Expected missing dist artifact, got: {errs}"
        print("  ✓ PASSED: Missing dist artifact was detected and failed closed.")

        # Test 4b: Corrupted dist artifact
        with open(dist_wasm, "wb") as f:
            f.write(b"CORRUPTED_DIST_WASM_BYTES")
        ok, errs = evidence.verify_artifacts(REPO_ROOT, check_dist=True)
        assert not ok, "Verification MUST fail when dist artifact is corrupted!"
        assert any("Dist artifact checksum mismatch" in e for e in errs), f"Expected dist checksum mismatch, got: {errs}"
        print("  ✓ PASSED: Corrupted dist artifact was detected and failed closed.")
    finally:
        if os.path.exists(dist_wasm):
            os.remove(dist_wasm)
        if os.path.exists(dist_bak):
            os.rename(dist_bak, dist_wasm)

def test_cli_false_success_regression():
    print("Testing Case 5: CLI false-success regression...")
    evidence_cli = os.path.join(REPO_ROOT, "harness", "runner", "evidence.py")

    # 1. Valid artifact set MUST exit 0
    res_valid = subprocess.run([sys.executable, evidence_cli, "--verify-artifacts"], capture_output=True, cwd=REPO_ROOT)
    assert res_valid.returncode == 0, f"Valid artifacts MUST exit 0, got {res_valid.returncode}. Output: {res_valid.stdout.decode()}"
    print("  ✓ Subcase 5.1: Valid artifact set exited 0.")

    # 2. Corrupted artifact set MUST exit non-zero (proving CLI tuple unpacking works!)
    html_path = os.path.join(REPO_ROOT, "public", "gridcraft", "index.html")
    with open(html_path, "rb") as f:
        original_bytes = f.read()

    try:
        with open(html_path, "wb") as f:
            f.write(original_bytes + b"<!-- CLI_FALSE_SUCCESS_TEST -->")

        res_corrupt = subprocess.run([sys.executable, evidence_cli, "--verify-artifacts"], capture_output=True, cwd=REPO_ROOT)
        assert res_corrupt.returncode != 0, f"CLI MUST exit non-zero when artifact is corrupted, but exited {res_corrupt.returncode}!"
        out_str = res_corrupt.stdout.decode() + res_corrupt.stderr.decode()
        assert "checksum mismatch" in out_str, f"Expected checksum mismatch in output, got: {out_str}"
        print("  ✓ Subcase 5.2: Corrupted artifact caused CLI subprocess to exit non-zero (false success prevented).")
    finally:
        with open(html_path, "wb") as f:
            f.write(original_bytes)

def test_modified_vendor_patch():
    print("Testing Case 6: Modified vendor patch fails verification...")
    patch_path = os.path.join(REPO_ROOT, "patches", "0001-gridcraft-confirmed-save.patch")
    with open(patch_path, "rb") as f:
        original_bytes = f.read()

    try:
        with open(patch_path, "wb") as f:
            f.write(original_bytes + b"\n# TAMPERED LINE\n")

        ok, errs = evidence.verify_vendor_pin(REPO_ROOT)
        assert not ok, "Verification MUST fail when vendor patch is modified!"
        assert any("checksum mismatch" in e for e in errs), f"Expected patch checksum mismatch, got: {errs}"
        print("  ✓ PASSED: Tampered vendor patch was rejected fail-closed.")
    finally:
        with open(patch_path, "wb") as f:
            f.write(original_bytes)

def main():
    print("================================================================")
    print("🧪 Running Executable Negative Verification Tests (P2 #4678429571 & Mandatory Corrections A, B)")
    print("================================================================")
    test_modified_vendor_source_outside_patch()
    test_modified_dependency_lockfile()
    test_changed_source_with_intact_old_bundle()
    test_missing_or_corrupted_dist_artifact()
    test_cli_false_success_regression()
    test_modified_vendor_patch()
    print("\n🎉 ALL 6 NEGATIVE VERIFICATION TESTS PASSED CLEANLY (FAIL-CLOSED VERIFIED)!")

if __name__ == "__main__":
    main()
