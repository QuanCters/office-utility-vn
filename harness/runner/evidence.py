#!/usr/bin/env python3
"""
harness/runner/evidence.py
Hệ thống quản lý Evidence Ledger:
- Gắn chặt kết quả kiểm thử và review với đúng phiên bản code (base, head, checkout, tree)
- Ngăn chặn stale evidence khi tác giả push commit mới
- Tính SHA-256 checksum cho mọi log, report và build artifact
- Định dạng theo harness/contracts/evidence-ledger.schema.json
"""

import os
import sys
import subprocess
import json
import datetime
import hashlib
import platform

def run_cmd(cmd: list[str], cwd=None) -> str:
    try:
        res = subprocess.run(cmd, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        return res.stdout.strip()
    except Exception:
        return "UNKNOWN"

def get_file_sha256(filepath: str) -> str:
    if not os.path.isfile(filepath):
        return ""
    h = hashlib.sha256()
    with open(filepath, "rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest()

def get_git_source_info(repo_root: str) -> dict:
    head_sha = run_cmd(["git", "rev-parse", "HEAD"], cwd=repo_root)
    tree_sha = run_cmd(["git", "rev-parse", "HEAD^{tree}"], cwd=repo_root)

    # In GitHub Actions: GITHUB_SHA is the checkout commit (often a merge ref)
    checkout_sha = os.environ.get("GITHUB_SHA", head_sha)

    # Base SHA: determine merge-base with origin/main or env
    base_ref = os.environ.get("GITHUB_BASE_REF") or os.environ.get("BASE_REF") or "main"
    base_sha = run_cmd(["git", "merge-base", f"origin/{base_ref}", "HEAD"], cwd=repo_root)
    if not base_sha or "fatal" in base_sha or base_sha == "UNKNOWN":
        base_sha = run_cmd(["git", "merge-base", base_ref, "HEAD"], cwd=repo_root)
    if not base_sha or "fatal" in base_sha or base_sha == "UNKNOWN":
        base_sha = run_cmd(["git", "rev-parse", "HEAD~1"], cwd=repo_root)
    if not base_sha or "fatal" in base_sha:
        base_sha = head_sha

    return {
        "base_sha": base_sha,
        "head_sha": head_sha,
        "checkout_sha": checkout_sha,
        "tree_sha": tree_sha
    }

def get_tool_versions() -> dict:
    rustc_v = run_cmd(["rustc", "--version"])
    cargo_v = run_cmd(["cargo", "--version"])
    trunk_v = run_cmd(["trunk", "--version"])
    agy_v = run_cmd(["agy", "--version"])

    return {
        "rustc": rustc_v or "Not installed",
        "cargo": cargo_v or "Not installed",
        "trunk": trunk_v or "Not installed",
        "antigravity_cli": agy_v or "Not installed",
        "os": f"{platform.system()} {platform.release()} ({platform.machine()})"
    }

def create_evidence_ledger(repo_root: str, task_id: str = "PR-VERIFY", pr_number: int = None) -> dict:
    source_info = get_git_source_info(repo_root)
    tool_vers = get_tool_versions()

    ledger = {
        "schema_version": 1,
        "task_id": task_id,
        "pr_number": pr_number,
        "source": source_info,
        "tool_versions": tool_vers,
        "review": {
            "model": "gemini-3.8-flash-high",
            "requested_effort": "high",
            "status": "PENDING",
            "findings_summary": {"P0": 0, "P1": 0, "P2": 0, "P3": 0},
            "verified_findings_ref": None
        },
        "checks": {
            "policy_integrity": "PENDING",
            "rust_static": "PENDING",
            "rust_core": "PENDING",
            "wasm_build": "PENDING",
            "native_e2e": "NOT_RUN",
            "verification_gate": "PENDING"
        },
        "evidence": [],
        "decision": "INCOMPLETE",
        "decision_reason": "Verification in progress"
    }
    return ledger

def append_evidence_item(ledger: dict, name: str, item_type: str, filepath: str):
    sha = get_file_sha256(filepath)
    ledger["evidence"].append({
        "name": name,
        "type": item_type,
        "path": filepath,
        "sha256": sha,
        "timestamp_utc": datetime.datetime.now(datetime.timezone.utc).isoformat()
    })

def check_stale_evidence(ledger: dict, current_head_sha: str) -> bool:
    """Kiểm tra xem bằng chứng trong ledger có bị stale do commit mới không"""
    recorded_head = ledger.get("source", {}).get("head_sha", "")
    return recorded_head != current_head_sha

def save_ledger(repo_root: str, ledger: dict, custom_name: str = None) -> str:
    reports_dir = os.path.join(repo_root, "harness", "reports")
    os.makedirs(reports_dir, exist_ok=True)
    head_sha = ledger.get("source", {}).get("head_sha", "unknown")
    fname = custom_name or f"evidence_ledger_{head_sha[:8]}.json"
    full_path = os.path.join(reports_dir, fname)

    with open(full_path, "w", encoding="utf-8") as f:
        json.dump(ledger, f, indent=2, ensure_ascii=False)

    # Cũng lưu bản sao manifest chuẩn cho CI artifact
    manifest_path = os.path.join(reports_dir, "evidence-manifest.json")
    with open(manifest_path, "w", encoding="utf-8") as f:
        json.dump(ledger, f, indent=2, ensure_ascii=False)

    return full_path

def record_tier_file(repo_root: str, tier: str, status: str, metadata: dict = None) -> str:
    """
    Ghi nhận kết quả riêng của một tier vào file JSON riêng biệt:
    harness/reports/tier-<tier>.json
    Ngăn chặn việc các runner song song (như L3 và L4) ghi đè lên nhau.
    """
    reports_dir = os.path.join(repo_root, "harness", "reports")
    os.makedirs(reports_dir, exist_ok=True)
    tier_lower = tier.lower()
    source_info = get_git_source_info(repo_root)
    status_upper = status.upper()
    if status_upper not in ["PASSED", "FAILED", "PENDING", "NOT_RUN", "SKIPPED"]:
        status_upper = "PASSED" if status_upper in ["PASS", "OK", "SUCCESS"] else "FAILED"

    tier_map = {
        "L0": ["policy_integrity"],
        "L1": ["rust_static"],
        "L2": ["rust_core"],
        "L3": ["integration_tests"],
        "L4": ["wasm_build"],
        "L5": ["verification_gate"],
    }
    check_keys = tier_map.get(tier.upper(), [tier.lower().replace("-", "_")])
    checks = {k: status_upper for k in check_keys}

    tier_data = {
        "schema_version": 1,
        "tier": tier.upper(),
        "status": status_upper,
        "checks": checks,
        "source": source_info,
        "timestamp_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "metadata": metadata or {}
    }
    tier_path = os.path.join(reports_dir, f"tier-{tier_lower}.json")
    with open(tier_path, "w", encoding="utf-8") as f:
        json.dump(tier_data, f, indent=2, ensure_ascii=False)
    return tier_path

def aggregate_tier_results(repo_root: str) -> dict:
    """
    Thu thập và tổng hợp toàn bộ các artifact kiểm chứng độc lập (tier-*.json)
    vào evidence-manifest.json trước khi Merge Gate đánh giá.
    """
    reports_dir = os.path.join(repo_root, "harness", "reports")
    os.makedirs(reports_dir, exist_ok=True)
    manifest_path = os.path.join(reports_dir, "evidence-manifest.json")

    if os.path.isfile(manifest_path):
        try:
            with open(manifest_path, "r", encoding="utf-8") as f:
                ledger = json.load(f)
        except Exception:
            ledger = create_evidence_ledger(repo_root)
    else:
        ledger = create_evidence_ledger(repo_root)

    tier_files = []
    for root, _, files in os.walk(reports_dir):
        for f in files:
            if f.startswith("tier-") and f.endswith(".json"):
                tier_files.append(os.path.join(root, f))

    tier_files.sort()
    failed_tiers = []
    integrity_errors = []
    seen_tiers = set()
    valid_statuses = ["PASSED", "FAILED", "PENDING", "NOT_RUN", "SKIPPED"]

    ledger_source = ledger.get("source", {})
    ledger_head = ledger_source.get("head_sha", "")
    ledger_base = ledger_source.get("base_sha", "")
    ledger_tree = ledger_source.get("tree_sha", "")

    for tf in tier_files:
        try:
            with open(tf, "r", encoding="utf-8") as f:
                tdata = json.load(f)
        except Exception as e:
            integrity_errors.append(f"Corrupt tier artifact {os.path.basename(tf)}: {e}")
            continue

        tier_name = tdata.get("tier", os.path.basename(tf))
        if tier_name in seen_tiers:
            integrity_errors.append(f"Duplicate tier artifact detected: {tier_name}")
            continue
        seen_tiers.add(tier_name)

        # Enforce source identity matching across all tier artifacts
        t_source = tdata.get("source", {})
        t_head = t_source.get("head_sha", "")
        t_base = t_source.get("base_sha", "")
        t_tree = t_source.get("tree_sha", "")

        if not t_head or not t_base or not t_tree:
            integrity_errors.append(f"Tier {tier_name} missing required source identity fields")
            continue

        if ledger_head and t_head != ledger_head:
            integrity_errors.append(
                f"Tier {tier_name} source mismatch (head_sha): expected {ledger_head[:8]}, got {t_head[:8]}"
            )
            continue

        if ledger_base and t_base != ledger_base:
            integrity_errors.append(
                f"Tier {tier_name} source mismatch (base_sha): expected {ledger_base[:8]}, got {t_base[:8]}"
            )
            continue

        if ledger_tree and t_tree != ledger_tree:
            integrity_errors.append(
                f"Tier {tier_name} source mismatch (tree_sha): expected {ledger_tree[:8]}, got {t_tree[:8]}"
            )
            continue

        t_status = tdata.get("status", "NOT_RUN")
        if t_status not in valid_statuses:
            integrity_errors.append(f"Tier {tier_name} has invalid status '{t_status}'")
            continue

        for chk_name, chk_status in tdata.get("checks", {}).items():
            ledger["checks"][chk_name] = chk_status
        if t_status == "FAILED":
            failed_tiers.append(tier_name)

        meta = tdata.get("metadata", {})
        if "risk_level" in meta:
            ledger["risk_assessment"] = meta["risk_level"]
        if "evidence_items" in meta:
            for item in meta["evidence_items"]:
                append_evidence_item(ledger, item.get("name", "artifact"), item.get("type", "TEST_LOG"), item.get("path", ""))

        append_evidence_item(ledger, os.path.basename(tf), "TIER_REPORT", tf)

    if integrity_errors:
        ledger["decision"] = "FAIL"
        ledger["decision_reason"] = f"Evidence ledger integrity check failed: {'; '.join(integrity_errors)}"
    elif failed_tiers:
        ledger["decision"] = "FAIL"
        ledger["decision_reason"] = f"Verification checks failed in tiers: {', '.join(failed_tiers)}"
    else:
        req_checks = ["policy_integrity", "rust_core", "wasm_build", "verification_gate"]
        if all(ledger["checks"].get(c) == "PASSED" for c in req_checks):
            ledger["decision"] = "PASS"
            ledger["decision_reason"] = "All required quality gates passed."
        else:
            pending = [c for c in req_checks if ledger["checks"].get(c) != "PASSED"]
            ledger["decision"] = "INCOMPLETE"
            ledger["decision_reason"] = f"Pending required checks: {', '.join(pending)}"

    save_ledger(repo_root, ledger)
    print(f"📊 Aggregated {len(tier_files)} tier reports into {manifest_path} (Decision: {ledger['decision']})")
    return ledger

def record_evidence(repo_root: str, tier: str, status: str, metadata: dict = None) -> dict:
    """
    Ghi nhận trạng thái kiểm chứng của một tầng vào Evidence Ledger.
    Đồng bộ hóa tên check theo contract schema và lưu trữ ngay lập tức.
    Đồng thời ghi ra tier-<tier>.json riêng biệt.
    """
    record_tier_file(repo_root, tier, status, metadata)
    reports_dir = os.path.join(repo_root, "harness", "reports")
    manifest_path = os.path.join(reports_dir, "evidence-manifest.json")

    if os.path.isfile(manifest_path):
        try:
            with open(manifest_path, "r", encoding="utf-8") as f:
                ledger = json.load(f)
        except Exception:
            ledger = create_evidence_ledger(repo_root)
    else:
        ledger = create_evidence_ledger(repo_root)

    status_upper = status.upper()
    if status_upper not in ["PASSED", "FAILED", "PENDING", "NOT_RUN", "SKIPPED"]:
        status_upper = "PASSED" if status_upper in ["PASS", "OK", "SUCCESS"] else "FAILED"

    tier_map = {
        "L0": ["policy_integrity"],
        "L1": ["rust_static"],
        "L2": ["rust_core"],
        "L3": ["integration_tests"],
        "L4": ["wasm_build"],
        "L5": ["verification_gate"],
    }

    tier_key = tier.upper()
    check_keys = tier_map.get(tier_key, [tier.lower().replace("-", "_")])

    for k in check_keys:
        ledger["checks"][k] = status_upper

    # Cập nhật quyết định sơ bộ
    if status_upper == "FAILED":
        ledger["decision"] = "FAIL"
        ledger["decision_reason"] = f"Verification check failed at tier {tier}"
    elif status_upper == "PASSED":
        req_checks = ["policy_integrity", "rust_core", "wasm_build", "verification_gate"]
        if all(ledger["checks"].get(c) == "PASSED" for c in req_checks):
            ledger["decision"] = "PASS"
            ledger["decision_reason"] = "All required quality gates passed."

    if metadata:
        if "risk_level" in metadata:
            ledger["risk_assessment"] = metadata["risk_level"]
        if "evidence_items" in metadata:
            for item in metadata["evidence_items"]:
                append_evidence_item(
                    ledger,
                    item.get("name", "artifact"),
                    item.get("type", "TEST_LOG"),
                    item.get("path", "")
                )

    save_ledger(repo_root, ledger)
    return ledger

def validate_evidence_ledger(repo_root: str, ledger_path: str = None) -> tuple[bool, list[str]]:
    """
    Xác thực tính toàn vẹn của Evidence Ledger:
    - Kiểm tra schema bắt buộc
    - Xác thực định danh nguồn (base_sha, head_sha, checkout_sha, tree_sha)
    - Phát hiện stale evidence
    - Phát hiện trạng thái check không hợp lệ
    """
    errors = []
    if not ledger_path:
        ledger_path = os.path.join(repo_root, "harness", "reports", "evidence-manifest.json")

    if not os.path.isfile(ledger_path):
        return False, [f"Evidence manifest file does not exist at '{ledger_path}'"]

    try:
        with open(ledger_path, "r", encoding="utf-8") as f:
            ledger = json.load(f)
    except Exception as e:
        return False, [f"Failed to parse evidence manifest as JSON: {e}"]

    # 1. Required top-level keys
    required_keys = ["schema_version", "task_id", "source", "tool_versions", "review", "checks", "evidence", "decision"]
    for k in required_keys:
        if k not in ledger:
            errors.append(f"Missing required key: '{k}'")

    # 2. Source identity validation
    source = ledger.get("source", {})
    for s_key in ["base_sha", "head_sha", "checkout_sha", "tree_sha"]:
        val = source.get(s_key, "")
        if not val or val == "UNKNOWN":
            errors.append(f"Invalid or missing source identity: 'source.{s_key}' = '{val}'")

    # 3. Decision validation
    valid_decisions = ["PASS", "FAIL", "INCOMPLETE", "MANUAL_HOLD"]
    if ledger.get("decision") not in valid_decisions:
        errors.append(f"Invalid decision '{ledger.get('decision')}'. Must be one of {valid_decisions}")

    # 4. Check stale evidence
    current_head = run_cmd(["git", "rev-parse", "HEAD"], cwd=repo_root)
    if current_head and current_head != "UNKNOWN":
        if check_stale_evidence(ledger, current_head):
            errors.append(f"STALE EVIDENCE: Recorded HEAD ({source.get('head_sha')}) does not match repository HEAD ({current_head})")

    # 5. Check checks enum
    valid_check_states = ["PASSED", "FAILED", "PENDING", "NOT_RUN", "SKIPPED"]
    for chk_name, chk_val in ledger.get("checks", {}).items():
        if chk_val not in valid_check_states:
            errors.append(f"Invalid check state for '{chk_name}': '{chk_val}' (must be one of {valid_check_states})")

    # 6. Validate TIER_REPORT evidence items (source identity & checksums)
    seen_report_tiers = set()
    for ev in ledger.get("evidence", []):
        if ev.get("type") == "TIER_REPORT":
            ev_name = ev.get("name", "")
            if ev_name in seen_report_tiers:
                errors.append(f"Duplicate tier report in evidence ledger: '{ev_name}'")
            seen_report_tiers.add(ev_name)

            ev_path = ev.get("path", "")
            if os.path.isfile(ev_path):
                actual_sha = get_file_sha256(ev_path)
                if ev.get("sha256") and actual_sha != ev.get("sha256"):
                    errors.append(f"Checksum mismatch for '{ev_name}': recorded {ev.get('sha256')[:8]}, computed {actual_sha[:8]}")

                try:
                    with open(ev_path, "r", encoding="utf-8") as tf_obj:
                        tdata = json.load(tf_obj)
                    t_src = tdata.get("source", {})
                    for sk in ["head_sha", "base_sha", "tree_sha"]:
                        if t_src.get(sk) and source.get(sk) and t_src.get(sk) != source.get(sk):
                            errors.append(f"Source identity mismatch in '{ev_name}' for '{sk}': tier={t_src.get(sk)[:8]} vs ledger={source.get(sk)[:8]}")
                except Exception as e:
                    errors.append(f"Corrupt or unreadable tier report '{ev_name}': {e}")

    return (len(errors) == 0, errors)

def verify_artifacts(repo_root: str) -> bool:
    dist_dir = os.path.join(repo_root, "dist")
    print("📦 Verifying build artifacts and checksums...")
    found_any = False

    if os.path.exists(dist_dir):
        for fname in os.listdir(dist_dir):
            if fname.endswith((".wasm", ".js")):
                fpath = os.path.join(dist_dir, fname)
                sha256 = get_file_sha256(fpath)
                size_kb = os.path.getsize(fpath) / 1024
                print(f"  - [Artifact] {fname} ({size_kb:.1f} KB) SHA256: {sha256[:16]}...")
                found_any = True

    return found_any

def main():
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
    if "--init-ledger" in sys.argv:
        ledger = create_evidence_ledger(repo_root)
        saved = save_ledger(repo_root, ledger)
        print(f"✅ Initialized evidence ledger at: {saved}")
        sys.exit(0)

    if "--validate-ledger" in sys.argv:
        custom_path = None
        for i, a in enumerate(sys.argv):
            if a == "--validate-ledger" and i + 1 < len(sys.argv) and not sys.argv[i + 1].startswith("--"):
                custom_path = sys.argv[i + 1]
        ok, errs = validate_evidence_ledger(repo_root, custom_path)
        if ok:
            print("✅ Evidence ledger validation passed cleanly.")
            sys.exit(0)
        else:
            print("❌ Evidence ledger validation failed:")
            for e in errs:
                print(f"  - {e}")
            sys.exit(1)

    if "--record-tier" in sys.argv:
        try:
            idx = sys.argv.index("--record-tier")
            tier = sys.argv[idx + 1]
            status = sys.argv[idx + 2]
            record_evidence(repo_root, tier, status)
            print(f"✅ Recorded tier {tier} as {status}")
            sys.exit(0)
        except Exception as e:
            print(f"❌ Error recording tier: {e}")
            sys.exit(1)

    if "--aggregate-tiers" in sys.argv:
        ledger = aggregate_tier_results(repo_root)
        print(f"✅ Tiers aggregated cleanly. Decision: {ledger.get('decision')}")
        sys.exit(0)

    if "--record-tier-file" in sys.argv:
        try:
            idx = sys.argv.index("--record-tier-file")
            tier = sys.argv[idx + 1]
            status = sys.argv[idx + 2]
            p = record_tier_file(repo_root, tier, status)
            print(f"✅ Recorded tier file at: {p}")
            sys.exit(0)
        except Exception as e:
            print(f"❌ Error recording tier file: {e}")
            sys.exit(1)

    if "--verify-artifacts" in sys.argv:
        ok = verify_artifacts(repo_root)
        sys.exit(0 if ok else 1)

    print("Evidence module loaded.")

if __name__ == "__main__":
    main()
