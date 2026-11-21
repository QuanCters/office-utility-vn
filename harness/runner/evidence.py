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
    base_ref = os.environ.get("GITHUB_BASE_REF", "main")
    base_sha = run_cmd(["git", "merge-base", f"origin/{base_ref}", "HEAD"], cwd=repo_root)
    if not base_sha or "fatal" in base_sha:
        base_sha = run_cmd(["git", "rev-parse", "HEAD~1"], cwd=repo_root)

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
    fname = custom_name or f"evidence_ledger_{ledger['source']['head_sha'][:8]}.json"
    full_path = os.path.join(reports_dir, fname)

    with open(full_path, "w", encoding="utf-8") as f:
        json.dump(ledger, f, indent=2, ensure_ascii=False)

    # Cũng lưu bản sao manifest chuẩn cho CI artifact
    manifest_path = os.path.join(reports_dir, "evidence-manifest.json")
    with open(manifest_path, "w", encoding="utf-8") as f:
        json.dump(ledger, f, indent=2, ensure_ascii=False)

    return full_path

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

    if "--verify-artifacts" in sys.argv:
        ok = verify_artifacts(repo_root)
        sys.exit(0 if ok else 1)

    print("Evidence module loaded.")

if __name__ == "__main__":
    main()
