#!/usr/bin/env python3
"""Conservative, fail-closed source policy checks for Office Utility VN.

NOT a Rust parser, security proof, or substitute for behavioral tests.
Run unit tests first; run --mode enforce from actual repository root.
"""
from __future__ import annotations

import argparse
import hashlib
from datetime import date
import fnmatch
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass(frozen=True)
class Finding:
    rule_id: str
    category: str
    path: str
    line: int
    message: str
    fingerprint: str = ""

    def to_dict(self) -> dict[str, Any]:
        return {"rule_id": self.rule_id, "category": self.category, "path": self.path,
                "line": self.line, "message": self.message, "fingerprint": self.fingerprint}


def line_no(text: str, offset: int) -> int:
    return text.count("\n", 0, max(0, offset)) + 1


def extract_brace_block(text: str, open_index: int) -> tuple[str, int] | None:
    """Extract brace-delimited body, roughly skipping literals/comments.

    Conservative heuristic (not tree-sitter). Review hand-written Rust macros.
    """
    if open_index >= len(text) or text[open_index] != "{":
        return None
    depth = 0
    mode = "code"
    i = open_index
    while i < len(text):
        c = text[i]
        n = text[i + 1] if i + 1 < len(text) else ""
        if mode == "line":
            if c == "\n":
                mode = "code"
        elif mode == "block":
            if c == "*" and n == "/":
                mode = "code"
                i += 1
        elif mode == "string":
            if c == "\\":
                i += 1
            elif c == '"':
                mode = "code"
        elif mode == "code":
            if c == "/" and n == "/":
                mode = "line"
                i += 1
            elif c == "/" and n == "*":
                mode = "block"
                i += 1
            elif c == '"':
                mode = "string"
            elif c == "{":
                depth += 1
            elif c == "}":
                depth -= 1
                if depth == 0:
                    return (text[open_index + 1:i], i + 1)
        i += 1
    return None


def specialized_matches(kind: str, text: str) -> list[int]:
    offsets: list[int] = []
    if kind == "ipc_fallback":
        dispatch = re.search(r"\b(?:async\s+)?fn\s+dispatch_tool_task\b", text)
        if not dispatch:
            return offsets
        start = text.find("{", dispatch.end())
        extracted = extract_brace_block(text, start)
        if not extracted:
            return [dispatch.start()]  # cannot inspect safely: block rather than silently pass
        body, _ = extracted
        for m in re.finditer(r"_\s*=>\s*\{", body):
            inner = extract_brace_block(body, m.end() - 1)
            if inner and re.search(r'\bstatus\s*:\s*"SUCCESS"', inner[0]):
                offsets.append(start + 1 + m.start())
    elif kind == "async_blocking":
        m = re.search(r"\basync\s+fn\s+inspect_file_metadata\b", text)
        if not m:
            return offsets
        start = text.find("{", m.end())
        extracted = extract_brace_block(text, start)
        if not extracted:
            return [m.start()]
        body, _ = extracted
        blocking = re.search(r"\bstd\s*::\s*fs\s*::|\bstd\s*::\s*io\s*::|\bFile\s*::\s*open\s*\(", body)
        offload = re.search(r"\b(?:spawn_blocking|tokio\s*::\s*fs\s*::)", body)
        if blocking and not offload:
            offsets.append(start + 1 + blocking.start())
    elif kind == "ime_unsafe":
        # Approximation for simple Leptos view! tags. Complex syntax requires AST/E2E review.
        for m in re.finditer(r"<(?:input|textarea)\b[\s\S]{0,3000}?/>", text):
            tag = m.group(0)
            if ("prop:value" in tag and "on:input" in tag
                    and not ("on:compositionstart" in tag and "on:compositionend" in tag)):
                offsets.append(m.start())
    return offsets


def load_policy(path: Path) -> dict[str, Any]:
    policy = json.loads(path.read_text(encoding="utf-8"))
    rule_ids = [r["id"] for r in policy["rules"]]
    if len(rule_ids) != len(set(rule_ids)) or not rule_ids:
        raise ValueError("Duplicate or empty policy rule ids")
    allowed = {"regex", "ipc_fallback", "async_blocking", "ime_unsafe"}
    for rule in policy["rules"]:
        if rule["kind"] not in allowed:
            raise ValueError(f"Unknown policy rule kind: {rule['kind']}")
        if rule["kind"] == "regex":
            re.compile(rule["pattern"], re.MULTILINE)
    return policy


def scan_repo(root: Path, policy: dict[str, Any], require_layout: bool = True) -> list[Finding]:
    root = root.resolve()
    findings: list[Finding] = []
    if require_layout:
        for rel in policy.get("required_source_paths", []):
            path = root / rel
            if not path.is_file():
                findings.append(Finding("LAYOUT001", "P0", rel, 0,
                    "Source path bắt buộc không tồn tại; audit migration và cập nhật policy qua PR được review."))

    rust_files = []
    for path in root.rglob("*.rs"):
        rel = path.relative_to(root).as_posix()
        if not any(part in {"target", "node_modules", ".git"} for part in path.parts):
            rust_files.append((rel, path))

    for rule in policy["rules"]:
        matched = [(rel, path) for rel, path in rust_files if fnmatch.fnmatch(rel, rule["file_glob"])]
        for rel, path in matched:
            source = path.read_text(encoding="utf-8", errors="replace")
            # Exclude full-line comments, not strings. Heuristic can raise false positives.
            source_for_scan = re.sub(r"(?m)^[ \t]*//[^\n]*", lambda m: " " * len(m.group(0)), source)
            if rule["kind"] == "regex":
                offsets = [m.start() for m in re.finditer(rule["pattern"], source_for_scan, re.MULTILINE)]
            else:
                offsets = specialized_matches(rule["kind"], source_for_scan)
            for offset in offsets:
                linenum = line_no(source, offset)
                code_line = source.splitlines()[linenum - 1].strip()
                fingerprint = hashlib.sha256(
                    f"{rule['id']}|{rel}|{linenum}|{code_line}".encode("utf-8")
                ).hexdigest()
                findings.append(Finding(rule["id"], rule["category"], rel,
                                        linenum, rule["message"], fingerprint))
    return sorted(findings, key=lambda x: (x.path, x.line, x.rule_id))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--policy", type=Path, default=Path(__file__).resolve().parent.parent / "policy/guardrails.json")
    parser.add_argument("--mode", choices=("audit", "incremental", "enforce"), default="enforce")
    parser.add_argument("--baseline", type=Path,
                        default=Path(__file__).resolve().parent.parent / "policy/approved_legacy_findings.json")
    parser.add_argument("--json", action="store_true", help="Print machine-readable results")
    args = parser.parse_args()
    try:
        policy = load_policy(args.policy)
        findings = scan_repo(args.repo, policy, require_layout=args.mode != "audit")
        blocking_findings = list(findings)
        baseline_errors = []
        if args.mode == "incremental":
            baseline = json.loads(args.baseline.read_text(encoding="utf-8"))
            approved = baseline.get("approved_findings", [])
            if not isinstance(approved, list):
                raise ValueError("approved_findings must be an array")
            fingerprints = [entry["fingerprint"] for entry in approved]
            if len(fingerprints) != len(set(fingerprints)):
                raise ValueError("baseline has duplicate approved fingerprints")
            accepted = set(fingerprints)
            if approved and baseline.get("expires_on") and date.today() > date.fromisoformat(baseline["expires_on"]):
                baseline_errors.append("Baseline expired; migration must be completed/reapproved")
            observed = {f.fingerprint for f in findings if f.rule_id != "LAYOUT001"}
            stale = accepted - observed
            if stale:
                baseline_errors.append(f"Stale baseline ({len(stale)} entry/entries). Delete fixed exceptions; cannot retain waiver")
            blocking_findings = [f for f in findings if f.fingerprint not in accepted or f.rule_id == "LAYOUT001"]
            for entry in approved:
                if any(not entry.get(key) or "REQUIRES_HUMAN" in str(entry.get(key)) for key in ("ticket", "owner", "reason")):
                    baseline_errors.append("Every legacy exception needs ticket, owner, reason")
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as exc:
        print(f"Policy error: {exc}", file=sys.stderr)
        return 2
    if args.json:
        print(json.dumps({"mode": args.mode, "findings": [x.to_dict() for x in findings]}, ensure_ascii=False, indent=2))
    else:
        for finding in findings:
            print(f"{finding.category} {finding.rule_id} {finding.path}:{finding.line}: {finding.message}")
        print(f"Detected {len(findings)} issue(s). MODE={args.mode}.")
    for error in baseline_errors:
        print("BASELINE ERROR:", error, file=sys.stderr)
    if args.mode in ("enforce", "incremental") and (blocking_findings or baseline_errors):
        if args.mode == "incremental":
            print(f"BLOCKED: {len(blocking_findings)} new/unapproved finding(s)", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
