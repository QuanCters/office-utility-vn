#!/usr/bin/env python3
"""
harness/runner/policy.py
Deterministic Policy Engine & Merge Gate Evaluator:
- Thực thi kiểm tra Protected Paths & Secrets
- Phân tích rủi ro tự động theo Git Diff (PR-aware)
- Đánh giá cổng Merge Gate xác định (PASS, FAIL, INCOMPLETE, MANUAL_HOLD) dựa trên Evidence Ledger & quality-gates.yaml
"""

import os
import sys
import re
import subprocess
import json

def parse_simple_yaml(text: str) -> dict:
    """Fallback YAML parser sử dụng Python Standard Library (Zero-Dependency)"""
    try:
        import yaml
        return yaml.safe_load(text)
    except ImportError:
        pass

    result = {}
    current_key = None
    sub_key = None
    list_target = None

    for line in text.splitlines():
        line = line.rstrip()
        if not line or line.startswith("#"):
            continue

        indent = len(line) - len(line.lstrip())
        stripped = line.strip()

        # Top level key
        if indent == 0 and ":" in stripped:
            parts = stripped.split(":", 1)
            k = parts[0].strip()
            v = parts[1].strip()
            if not v:
                result[k] = {}
                current_key = k
                sub_key = None
            else:
                result[k] = v.strip("'\"")
                current_key = None

        # Level 1 indentation (2 spaces)
        elif indent == 2 and current_key is not None:
            if stripped.startswith("- "):
                if not isinstance(result[current_key], list):
                    result[current_key] = []
                result[current_key].append(stripped[2:].strip().strip("'\""))
            elif ":" in stripped:
                parts = stripped.split(":", 1)
                k = parts[0].strip()
                v = parts[1].strip()
                if not v:
                    result[current_key][k] = {}
                    sub_key = k
                else:
                    if v.lower() == "true":
                        val = True
                    elif v.lower() == "false":
                        val = False
                    elif v.isdigit():
                        val = int(v)
                    else:
                        val = v.strip("'\"")
                    result[current_key][k] = val
                    sub_key = None

        # Level 2 indentation (4 spaces)
        elif indent == 4 and current_key is not None and sub_key is not None:
            if stripped.startswith("- "):
                if not isinstance(result[current_key][sub_key], list):
                    result[current_key][sub_key] = []
                result[current_key][sub_key].append(stripped[2:].strip().strip("'\""))
            elif ":" in stripped:
                parts = stripped.split(":", 1)
                k = parts[0].strip()
                v = parts[1].strip()
                val = v.strip("'\"")
                if val.lower() == "true":
                    val = True
                elif val.lower() == "false":
                    val = False
                elif val.isdigit():
                    val = int(val)
                result[current_key][sub_key][k] = val

    return result

def get_git_diff_files(repo_root: str) -> list[str]:
    """
    Xác định danh sách các tệp tin bị thay đổi một cách toàn diện (PR-aware).
    Khắc phục điểm mù trong môi trường CI checkout sạch (clean checkout):
    1. So sánh merge-base với nhánh cơ sở (GITHUB_BASE_REF hoặc BASE_REF)
    2. Fallback sang origin/main hoặc HEAD~1 nếu không có base_ref
    3. Kết hợp thêm các thay đổi chưa commit (unstaged, staged, untracked) trong working tree
    4. Trong môi trường CI, nếu không xác định được diff thì fail-closed (raise RuntimeError).
    """
    files = set()
    is_ci = os.environ.get("GITHUB_ACTIONS") == "true" or os.environ.get("CI") == "true"
    base_ref = os.environ.get("GITHUB_BASE_REF") or os.environ.get("BASE_REF")
    diff_targets = []
    if base_ref:
        diff_targets.append(f"origin/{base_ref}...HEAD")
        diff_targets.append(f"{base_ref}...HEAD")
    else:
        diff_targets.append("origin/main...HEAD")
        diff_targets.append("main...HEAD")

    diff_success = False
    for target in diff_targets:
        res = subprocess.run(
            ["git", "diff", "--name-only", target],
            cwd=repo_root,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )
        if res.returncode == 0:
            diff_success = True
            for line in res.stdout.splitlines():
                if line.strip():
                    files.add(line.strip())
            break

    # Luôn kiểm tra thêm các thay đổi chưa commit trong working tree (local dev)
    res_head = subprocess.run(["git", "diff", "--name-only", "HEAD"], cwd=repo_root, stdout=subprocess.PIPE, text=True)
    if res_head.returncode == 0:
        for line in res_head.stdout.splitlines():
            if line.strip():
                files.add(line.strip())

    res_cached = subprocess.run(["git", "diff", "--cached", "--name-only"], cwd=repo_root, stdout=subprocess.PIPE, text=True)
    if res_cached.returncode == 0:
        for line in res_cached.stdout.splitlines():
            if line.strip():
                files.add(line.strip())

    res_untracked = subprocess.run(["git", "status", "--porcelain"], cwd=repo_root, stdout=subprocess.PIPE, text=True)
    if res_untracked.returncode == 0:
        for line in res_untracked.stdout.splitlines():
            if line.startswith("??"):
                files.add(line[3:].strip())

    if is_ci and not files:
        raise RuntimeError("Fail-closed: unable to determine committed PR diff in CI environment (files list is empty)")

    return sorted(list(files))

# Tệp duy nhất được phép tu chính trong phạm vi RFC PR-01 đã được phê duyệt
AUTHORIZED_SPINE_AMENDMENT_FILES = {
    "src-tauri/src/commands/mod.rs",
    "scripts/verify_spine_lock.sh",
}

# Danh mục tệp duy nhất thuộc Harness & CI được phép tu chính theo phê duyệt của maintainer
AUTHORIZED_HARNESS_AMENDMENT_FILES = {
    "harness/policies/quality-gates.yaml",
    "harness/runner/evidence.py",
    "harness/runner/policy.py",
    "harness/runner/run.py",
    "harness/ci/verify-core.sh",
    "harness/ci/verify-tauri.sh",
    "harness/ci/verify-wasm.sh",
    "harness/ci/verify-native-e2e.sh",
    ".github/workflows/verify-pr.yml",
    ".github/workflows/native-e2e.yml",
    ".github/workflows/release-gate.yml",
}

def check_protected_paths(repo_root: str) -> tuple[bool, list[str]]:
    """
    Kiểm tra các đường dẫn bất biến được bảo vệ (Protected Paths Guard).
    - Spine Layer: crates/office-shared/, src-tauri/src/db/, src-tauri/src/commands/, scripts/verify_spine_lock.sh
    - Pinned Vendor: vendor/
    - Governance: .github/CODEOWNERS
    - Harness & CI: harness/policies/, harness/ci/, harness/runner/, .github/workflows/
    """
    violations = []
    try:
        changed_files = get_git_diff_files(repo_root)
    except Exception as e:
        return (False, [f"Fail-closed policy violation: unable to determine changed files in CI: {e}"])

    is_agent_feature_run = os.environ.get("AGENT_FEATURE_RUN", "0") == "1"
    allow_spine_amendment = (
        os.environ.get("ALLOW_SPINE_AMENDMENT", "0") == "1"
        or os.environ.get("SPINE_AMENDMENT_AUTHORIZED", "0") == "1"
    )
    allow_harness_amendment = (
        os.environ.get("ALLOW_HARNESS_AMENDMENT", "0") == "1"
    )

    spine_roots = [
        "crates/office-shared/",
        "src-tauri/src/db/",
        "src-tauri/src/commands/",
        "scripts/verify_spine_lock.sh",
    ]
    vendor_and_governance = [
        "vendor/",
        ".github/CODEOWNERS",
    ]
    harness_and_ci = [
        "harness/policies/",
        "harness/ci/",
        "harness/runner/",
        ".github/workflows/",
    ]

    for f in changed_files:
        # 1. Kiểm tra Spine Layer
        is_spine = any(f.startswith(p) or f == p for p in spine_roots)
        if is_spine:
            if not is_agent_feature_run and allow_spine_amendment:
                if f in AUTHORIZED_SPINE_AMENDMENT_FILES:
                    # Thuộc danh sách cho phép có phạm vi cụ thể của RFC PR-01
                    pass
                else:
                    violations.append(f"Protected spine path modification not authorized under amendment: '{f}'")
            else:
                violations.append(f"Protected spine path modification detected: '{f}'")

        # 2. Kiểm tra Pinned Vendor & Governance (Tuyệt đối không có ngoại lệ)
        is_vendor_gov = any(f.startswith(p) or f == p for p in vendor_and_governance)
        if is_vendor_gov:
            violations.append(f"Protected invariant path modification detected: '{f}' (guard: vendor/governance)")

        # 3. Kiểm tra Harness & CI (Có phạm vi danh mục cụ thể được maintainer phê duyệt)
        is_harness_ci = any(f.startswith(p) or f == p for p in harness_and_ci)
        if is_harness_ci:
            if not is_agent_feature_run and allow_harness_amendment:
                if f in AUTHORIZED_HARNESS_AMENDMENT_FILES:
                    # Thuộc danh sách cho phép có phạm vi cụ thể của Harness
                    pass
                else:
                    violations.append(f"Protected harness/CI path modification not authorized under amendment: '{f}'")
            else:
                violations.append(f"Protected path modification detected: '{f}' (guard: harness/ci)")

    return (len(violations) == 0, violations)

def scan_secrets(repo_root: str) -> tuple[bool, list[str]]:
    secret_patterns = [
        re.compile(r"(?i)(api[_-]?key|secret[_-]?key|private[_-]?key|auth[_-]?token)\s*[:=]\s*['\"][A-Za-z0-9_\-]{16,}['\"]"),
        re.compile(r"-----BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY-----"),
        re.compile(r"ghp_[A-Za-z0-9]{36}"),
    ]
    violations = []
    changed_files = get_git_diff_files(repo_root)

    for f in changed_files:
        full_path = os.path.join(repo_root, f)
        if not os.path.isfile(full_path):
            continue
        if f.endswith((".png", ".jpg", ".pdf", ".lock", ".ico", ".wasm")):
            continue
        try:
            with open(full_path, "r", encoding="utf-8", errors="ignore") as file_obj:
                for line_idx, line in enumerate(file_obj, 1):
                    for pat in secret_patterns:
                        if pat.search(line):
                            violations.append(f"Possible secret detected in '{f}' line {line_idx}")
                            break
        except Exception:
            pass

    return (len(violations) == 0, violations)

def classify_risk(repo_root: str) -> dict:
    changed_files = get_git_diff_files(repo_root)
    if not changed_files:
        return {"risk_level": "LOW", "reasons": ["No changes detected"], "required_tiers": ["L0", "L1"]}

    risk = "LOW"
    reasons = []
    required_tiers = ["L0", "L1", "L2"]

    for f in changed_files:
        if "safe_persistence" in f or "persistence" in f or f.startswith("src-tauri/src/db/"):
            risk = "HIGH"
            reasons.append(f"Persistence layer touched: {f}")
            required_tiers = ["L0", "L1", "L2", "L3"]
        elif f.startswith("src-tauri/src/commands/") or f.startswith("src-tauri/capabilities/"):
            risk = "CRITICAL"
            reasons.append(f"IPC capability/boundary touched: {f}")
            required_tiers = ["L0", "L1", "L2", "L3", "L4"]
        elif f.startswith("vendor/") or f.startswith("patches/"):
            risk = "CRITICAL"
            reasons.append(f"Vendor pinned code touched: {f}")
            required_tiers = ["L0", "L1", "L2", "L3", "L4", "L5"]
        elif f.startswith("scripts/verify_spine_lock.sh") or f.startswith("harness/"):
            risk = "CRITICAL"
            reasons.append(f"Spine lock / verification harness touched: {f}")
            required_tiers = ["L0", "L1", "L2", "L3", "L4", "L5"]
        elif f.startswith(".github/"):
            risk = "CRITICAL"
            reasons.append(f"CI workflow touched: {f}")
            required_tiers = ["L0", "L1", "L2", "L3", "L4", "L5"]
        elif "pdf" in f.lower() or "doc_suite" in f:
            if risk != "CRITICAL":
                risk = "HIGH"
            reasons.append(f"PDF AST / byte manipulation touched: {f}")
            if "L3" not in required_tiers:
                required_tiers.append("L3")

    return {
        "risk_level": risk,
        "reasons": reasons,
        "required_tiers": required_tiers,
        "changed_files_count": len(changed_files)
    }

def evaluate_merge_gate(repo_root: str, ledger_path: str = None) -> tuple[str, str, int]:
    """
    Đánh giá Merge Gate xác định dựa trên Evidence Ledger & quality-gates.yaml
    Trả về: (Decision, Reason, ExitCode)
      - PASS (0): Đạt tất cả điều kiện
      - FAIL (1): Có lỗi test hoặc vi phạm chính sách
      - MANUAL_HOLD (2): Có AI Finding P0/P1 hoặc rủi ro cần xử lý
      - INCOMPLETE (3): Thiếu bằng chứng, đang dở dang hoặc bị skip
    """
    policy_path = os.path.join(repo_root, "harness", "policies", "quality-gates.yaml")
    if not os.path.isfile(policy_path):
        return ("FAIL", "Quality gates policy file missing", 1)

    with open(policy_path, "r", encoding="utf-8") as f:
        policy = parse_simple_yaml(f.read())

    if not ledger_path:
        ledger_path = os.path.join(repo_root, "harness", "reports", "evidence-manifest.json")

    if not os.path.isfile(ledger_path):
        return ("INCOMPLETE", f"Evidence ledger file not found at: {ledger_path}", 3)

    with open(ledger_path, "r", encoding="utf-8") as f:
        ledger = json.load(f)

    # 1. Kiểm tra Stale Evidence (require_current_commit)
    current_head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=repo_root, stdout=subprocess.PIPE, text=True).stdout.strip()
    recorded_head = ledger.get("source", {}).get("head_sha", "")

    if policy.get("merge_policy", {}).get("require_current_commit", True):
        if current_head != recorded_head:
            return ("INCOMPLETE", f"STALE EVIDENCE: Recorded HEAD ({recorded_head[:8]}) does not match current HEAD ({current_head[:8]}). Re-run CI & review.", 3)

    # 2. Kiểm tra AI Findings (P0, P1, P2)
    findings_summary = ledger.get("review", {}).get("findings_summary", {})
    p0_count = findings_summary.get("P0", 0)
    p1_count = findings_summary.get("P1", 0)
    p2_count = findings_summary.get("P2", 0)

    if p0_count > 0 or p1_count > 0:
        return ("MANUAL_HOLD", f"Unresolved critical/high findings detected (P0: {p0_count}, P1: {p1_count}). Requires maintainer disposition.", 2)

    # 3. Kiểm tra Required Checks (Tuyệt đối không coi SKIPPED là PASS)
    required_checks = policy.get("merge_policy", {}).get("required_checks", [])
    checks_state = ledger.get("checks", {})

    missing_or_pending = []
    failed_checks = []

    for req in required_checks:
        st = checks_state.get(req.replace("-", "_"), "NOT_RUN")
        if st == "FAILED":
            failed_checks.append(req)
        elif st in ("PENDING", "NOT_RUN", "SKIPPED"):
            missing_or_pending.append(req)

    if failed_checks:
        return ("FAIL", f"Required checks failed: {', '.join(failed_checks)}", 1)

    if missing_or_pending:
        return ("INCOMPLETE", f"Required checks incomplete, missing, or skipped: {', '.join(missing_or_pending)}", 3)

    # 4. Kiểm tra Protected Paths và Secrets
    ok_paths, path_errs = check_protected_paths(repo_root)
    if not ok_paths:
        return ("FAIL", f"Protected path violations: {path_errs}", 1)

    ok_sec, sec_errs = scan_secrets(repo_root)
    if not ok_sec:
        return ("FAIL", f"Secrets detected: {sec_errs}", 1)

    return ("PASS", "All quality gates and invariant checks passed cleanly.", 0)

def main():
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
    if "--evaluate-gate" in sys.argv:
        ledger_arg = None
        for i, a in enumerate(sys.argv):
            if a == "--ledger" and i + 1 < len(sys.argv):
                ledger_arg = sys.argv[i + 1]
        decision, reason, code = evaluate_merge_gate(repo_root, ledger_arg)
        print(f"🚦 [MERGE GATE DECISION]: {decision}")
        print(f"📋 Reason: {reason}")
        sys.exit(code)

    if "--check-protected-paths" in sys.argv:
        ok, violations = check_protected_paths(repo_root)
        if not ok:
            print("❌ Protected paths violation:")
            for v in violations:
                print(f"  - {v}")
            sys.exit(1)
        print("✅ Protected paths policy verified successfully.")
        sys.exit(0)

    if "--scan-secrets" in sys.argv:
        ok, violations = scan_secrets(repo_root)
        if not ok:
            print("❌ Secrets detected in changed files:")
            for v in violations:
                print(f"  - {v}")
            sys.exit(1)
        print("✅ Secrets scan passed cleanly.")
        sys.exit(0)

    if "--classify-risk" in sys.argv:
        risk_info = classify_risk(repo_root)
        print(json.dumps(risk_info, indent=2))
        sys.exit(0)

    print("Usage: policy.py [--evaluate-gate [--ledger PATH] | --check-protected-paths | --scan-secrets | --classify-risk]")
    sys.exit(1)

if __name__ == "__main__":
    main()
