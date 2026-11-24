#!/usr/bin/env python3
"""Fail-closed inventory gate; checks *presence*, not test semantics. Run cargo tests too."""
import argparse
import json
from pathlib import Path
import re

# Do not allow an Agent to make the gate green by deleting required cases.
REQUIRED_IDS = {
    "IPC-ALL-TOOLS", "IPC-COVERAGE", "PDF-OBJECTS", "PDF-INVALID",
    "OCR-NO-HARDCODE", "VIDEO-NO-FAKE", "PNG-RESOURCE-BOUND",
    "PNG-NO-WHITE-FALLBACK", "FILE-TRANSACTION", "VNI-ROUNDTRIP",
    "PASSWORD-LIFETIME", "SESSION-CONCURRENT", "TOKIO-ASYNC-RESPONSE",
    "SIGN-PADES-CRYPTO", "PDF-REDACTION", "PDF-BITMAP-REAL", "IME-WINDOWS-E2E"
}


def check(repo: Path, plan: Path) -> list[str]:
    requirements = json.loads(plan.read_text(encoding="utf-8"))["required_source_tests"]
    if not requirements:
        return ["source-test inventory is empty"]
    errs = []
    present_ids = {req.get("id") for req in requirements}
    if missing := REQUIRED_IDS - present_ids:
        errs.append("required source-test ids deleted: " + ", ".join(sorted(missing)))
    seen = set()
    for req in requirements:
        case = req.get("id")
        if not case or case in seen:
            errs.append(f"duplicate/empty test id {case!r}")
            continue
        seen.add(case)
        path = repo / req["file"]
        if not path.is_file():
            errs.append(f"{case}: missing {req['file']}")
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        if ("NOT IMPLEMENTED" in text or "todo!()" in text or "unimplemented!()" in text
                or "#[ignore]" in text or re.search(r"assert!\s*\(\s*true\s*\)", text)):
            errs.append(f"{case}: forbidden skipped/placeholder/trivially passing test in {req['file']}")
        if "test_function" in req:
            symbol = req["test_function"]
            if not re.search(r"\bfn\s+" + re.escape(symbol) + r"\s*\(", text):
                errs.append(f"{case}: missing fn {symbol} in {req['file']}")
        elif "required_text" in req:
            if req["required_text"] not in text:
                errs.append(f"{case}: missing test marker {req['required_text']} in {req['file']}")
        else:
            errs.append(f"{case}: no test identity defined")
    return errs


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--plan", type=Path, default=Path(__file__).resolve().parent.parent / "tests/verification/source_test_inventory.json")
    args = parser.parse_args()
    for error in (errors := check(args.repo, args.plan)):
        print("FAIL", error)
    if errors:
        raise SystemExit(1)
    print("PASS source-test inventory (existence only; cargo tests + review still required)")


if __name__ == "__main__":
    main()
