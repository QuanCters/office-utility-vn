#!/usr/bin/env python3
"""
harness/runner/run.py
Trình điều phối chính (Orchestrator) của Verification Harness:
- Chạy theo từng tầng L0, L1, L2, L3, L4, L5
- Tự động đánh giá rủi ro (Risk classification)
- Tuân thủ nguyên tắc an toàn: lỗi bộ chọn test -> nâng cấp rủi ro lên HIGH và chạy rộng hơn
- Ghi nhận bằng chứng kiểm chứng vào harness/reports/
"""

import sys
import os
import argparse
import subprocess
import json
import time

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
sys.path.insert(0, os.path.join(repo_root, "harness", "runner"))

import policy
import evidence

def run_command(cmd: list[str], cwd=repo_root) -> tuple[int, str]:
    print(f"\n🚀 [CMD] {' '.join(cmd)}")
    start_t = time.time()
    res = subprocess.run(cmd, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    duration = time.time() - start_t
    print(res.stdout)
    print(f"⏱️ Finished in {duration:.2f}s (Exit code: {res.returncode})")
    return res.returncode, res.stdout

def run_l0() -> bool:
    print("\n==========================================")
    print("🔒 [Tier L0] Source & Policy Integrity")
    print("==========================================")
    
    # 1. Check spine lock
    code, _ = run_command(["./scripts/verify_spine_lock.sh"])
    if code != 0:
        return False

    # 2. Check protected paths
    ok, violations = policy.check_protected_paths(repo_root)
    if not ok:
        print(f"❌ Protected path violations: {violations}")
        return False

    # 3. Scan secrets
    ok, violations = policy.scan_secrets(repo_root)
    if not ok:
        print(f"❌ Secret leakage detected: {violations}")
        return False

    return True

def run_l1() -> bool:
    print("\n==========================================")
    print("🧹 [Tier L1] Rust Static Checks")
    print("==========================================")
    code, _ = run_command(["./harness/ci/verify-core.sh", "--static-only"])
    return code == 0

def run_l2() -> bool:
    print("\n==========================================")
    print("🧪 [Tier L2] Unit & Property Tests")
    print("==========================================")
    code, _ = run_command(["./harness/ci/verify-core.sh", "--unit-only"])
    return code == 0

def run_l3() -> bool:
    print("\n==========================================")
    print("🛡️ [Tier L3] Integration & Fault Injection")
    print("==========================================")
    code, _ = run_command(["./harness/ci/verify-tauri.sh"])
    return code == 0

def run_l4() -> bool:
    print("\n==========================================")
    print("🌐 [Tier L4] Frontend WASM & Native E2E")
    print("==========================================")
    code_wasm, _ = run_command(["./harness/ci/verify-wasm.sh"])
    if code_wasm != 0:
        return False
    return True

def run_l5() -> bool:
    print("\n==========================================")
    print("📦 [Tier L5] Release Assurance & Pinned Vendor Integrity")
    print("==========================================")
    # 1. Pre-build vendor source & patch verification
    ok_vendor, vendor_errors = evidence.verify_vendor_pin(repo_root)
    if not ok_vendor:
        print("❌ [Tier L5] Pinned vendor verification failed:")
        for e in vendor_errors:
            print(f"   - {e}")
        return False

    # 2. Post-build artifact checksum verification
    ok_artifacts, art_errors = evidence.verify_artifacts(repo_root)
    if not ok_artifacts:
        print("❌ [Tier L5] Build artifact checksum verification failed:")
        for e in art_errors:
            print(f"   - {e}")
        return False

    print("✅ [Tier L5] All vendor pins, patches, and release artifacts verified cleanly.")
    return True

def main():
    parser = argparse.ArgumentParser(description="Office Utility VN - Verification Harness CLI")
    parser.add_argument("--tier", choices=["L0", "L1", "L2", "L3", "L4", "L5"], help="Chạy kiểm chứng cho một tầng cụ thể")
    parser.add_argument("--all", action="store_true", help="Chạy toàn bộ các tầng kiểm chứng L0 -> L4")
    parser.add_argument("--task", type=str, help="Kiểm chứng theo hợp đồng Task ID (ví dụ: PDF-042)")
    parser.add_argument("--check-vendor-pin", action="store_true", help="Kiểm tra tính toàn vẹn phụ thuộc vendor")
    args = parser.parse_args()

    # Xử lý cờ kiểm tra vendor pin độc lập nếu được yêu cầu
    if args.check_vendor_pin:
        print("🔍 Explicit vendor pin check requested via --check-vendor-pin")
        ok_vendor, vendor_errors = evidence.verify_vendor_pin(repo_root)
        if not ok_vendor:
            print("\n❌ [Vendor Pin Check Failed]")
            for err in vendor_errors:
                print(f"  - {err}")
            sys.exit(1)
        print("✅ [Vendor Pin Check Passed] All pinned vendor sources and patches verified.")
        if not args.tier and not args.all:
            sys.exit(0)

    # Phân tích rủi ro tự động
    risk_info = policy.classify_risk(repo_root)
    print(f"🔍 [Harness Risk Assessment] Risk Level: {risk_info['risk_level']}")
    if risk_info['reasons']:
        for r in risk_info['reasons']:
            print(f"   - {r}")

    tiers_to_run = []
    if args.tier:
        tiers_to_run = [args.tier]
    elif args.all:
        tiers_to_run = ["L0", "L1", "L2", "L3", "L4"]
    else:
        # Tự động chọn tầng theo risk rules
        tiers_to_run = risk_info.get("required_tiers", ["L0", "L1", "L2"])

    print(f"📋 Scheduled Verification Tiers: {', '.join(tiers_to_run)}")

    overall_success = True
    tier_runners = {
        "L0": run_l0,
        "L1": run_l1,
        "L2": run_l2,
        "L3": run_l3,
        "L4": run_l4,
        "L5": run_l5,
    }

    for tier in tiers_to_run:
        runner = tier_runners.get(tier)
        if runner:
            success = runner()
            status_str = "PASSED" if success else "FAILED"
            evidence.record_evidence(repo_root, tier, status_str, {"risk_level": risk_info["risk_level"]})
            if not success:
                print(f"\n❌ [Verification Failed] Tier {tier} did not pass!")
                overall_success = False
                break

    if overall_success:
        print(f"\n🎉 [Verification Harness] Toàn bộ các tầng [{', '.join(tiers_to_run)}] đã vượt qua thành công!")
        sys.exit(0)
    else:
        print("\n🚨 [Verification Harness] Kiểm chứng thất bại. Vui lòng khắc phục lỗi trước khi mở PR.")
        sys.exit(1)

if __name__ == "__main__":
    main()
