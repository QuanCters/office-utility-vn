#!/usr/bin/env python3
"""Fail-closed black-box verification runner.

The adapter MUST call the real production Engine/IPC path; see docs/ADAPTER_CONTRACT.md.
Synthetic inputs are permitted for negative/fuzz tests, NOT as the only positive fixtures.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import sys
import tempfile
import unicodedata


class VerificationError(Exception):
    pass


# Removing a case from the manifest must NOT make the release gate green.
REQUIRED_CASE_IDS = {
    "IPC-UNIMPLEMENTED-RETURNS-ERROR",
    "PDF-MERGE-REAL-FIXTURES",
    "PDF-SPLIT-KEEP-RESOURCES",
    "PDF-CORRUPT-REJECTED-NO-OUTPUT",
    "OCR-VIETNAMESE-REAL-SCAN",
    "VIDEO-REAL-FRAMES",
    "PDF-RENDER-REAL-CONTENT",
    "VNI-CONVERT-BYTE-LEVEL",
    "PNG-BROKEN-STREAM-NO-SILENT-FALLBACK",
    "OUTPUT-DESTINATION-ERROR-NO-FAKE-SUCCESS",
    "PDF-SIGN-CMS-VERIFIED",
    "PNG-DECOMPRESSION-OVERLIMIT-REJECTED",
}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def run_checked(args: list[str], *, timeout: int = 60, input_text: str | None = None) -> str:
    try:
        process = subprocess.run(args, input=input_text, text=True, capture_output=True,
                                 check=False, timeout=timeout)
    except FileNotFoundError as exc:
        raise VerificationError(f"Missing external dependency: {args[0]}") from exc
    except subprocess.TimeoutExpired as exc:
        raise VerificationError(f"Timed out executing: {args[0]}") from exc
    if process.returncode != 0:
        raise VerificationError(f"Command failed ({process.returncode}): {args[0]}\n{process.stderr[-900:]}")
    return process.stdout


def assert_text_tokens(text: str, tokens: list[str], case_id: str) -> None:
    norm = unicodedata.normalize("NFC", text).casefold()
    for token in tokens:
        if unicodedata.normalize("NFC", token).casefold() not in norm:
            raise VerificationError(f"{case_id}: missing text token: {token!r}")


def check_output(path: Path, expectation: dict, case_id: str) -> None:
    if not path.is_file() or path.stat().st_size == 0:
        raise VerificationError(f"{case_id}: output missing or empty: {path}")
    kind = expectation["type"]
    if kind == "pdf":
        run_checked(["qpdf", "--check", str(path)], timeout=30)
        info = run_checked(["pdfinfo", str(path)], timeout=30)
        page_match = re.search(r"(?m)^Pages:\s*(\d+)\s*$", info)
        if not page_match:
            raise VerificationError(f"{case_id}: pdfinfo cannot report page count")
        pages = int(page_match.group(1))
        if pages <= 0 or ("pages" in expectation and pages != expectation["pages"]):
            raise VerificationError(f"{case_id}: wrong PDF pages={pages}")
        if expectation.get("tokens"):
            extracted = run_checked(["pdftotext", "-layout", str(path), "-"], timeout=40)
            assert_text_tokens(extracted, expectation["tokens"], case_id)
        if expectation.get("signature_valid", False):
            # Cryptographic validity only. Certificate trust/revocation is a separate gate.
            sig_report = run_checked(["pdfsig", "-nocert", str(path)], timeout=30)
            if not re.search(r"Signature Validation:\s*Signature is Valid\b", sig_report):
                raise VerificationError(f"{case_id}: pdfsig does not confirm valid CMS signature")
        if expectation.get("render_pages", False):
            if pages > 50:
                raise VerificationError("Rendering >50 pages requires explicit workload policy")
            with tempfile.TemporaryDirectory(prefix="office-pdf-render-") as folder:
                prefix = str(Path(folder) / "page")
                run_checked(["pdftoppm", "-f", "1", "-l", str(pages), "-r", "72",
                             "-png", str(path), prefix], timeout=90)
                if len(list(Path(folder).glob("page-*.png"))) != pages:
                    raise VerificationError(f"{case_id}: not all pages rendered")
    elif kind in ("png", "jpeg", "gif", "webp"):
        try:
            from PIL import Image
        except ImportError as exc:
            raise VerificationError("Pillow is required for image validations") from exc
        with Image.open(path) as img:
            img.verify()  # decode integrity, not content correctness
        with Image.open(path) as img:
            if "min_frames" in expectation and getattr(img, "n_frames", 1) < expectation["min_frames"]:
                raise VerificationError(f"{case_id}: output does not contain enough frames")
            if "dimensions" in expectation and list(img.size) != expectation["dimensions"]:
                raise VerificationError(f"{case_id}: dimensions mismatch")
            if expectation.get("must_be_non_uniform", False):
                # Cheap guard against white/grey image fallbacks; NOT a semantic oracle.
                img.seek(0)
                img.load()
                palette = img.convert("RGB")
                if len(palette.getcolors(palette.width * palette.height) or []) <= 1:
                    raise VerificationError(f"{case_id}: uniform bitmap; possible fallback")
    elif kind == "text":
        decoded = path.read_text(encoding="utf-8", errors="strict")
        assert_text_tokens(decoded, expectation.get("tokens", []), case_id)
        if "exact_text_file" in expectation:
            expected_path = Path(expectation["exact_text_file"])
            if not expected_path.is_file():
                raise VerificationError(f"{case_id}: missing expected text oracle: {expected_path}")
            if unicodedata.normalize("NFC", decoded) != unicodedata.normalize("NFC", expected_path.read_text(encoding="utf-8")):
                raise VerificationError(f"{case_id}: text differs from independent golden file")
    else:
        raise VerificationError(f"{case_id}: unknown expected output type: {kind}")


def validate_manifest(cases: list[dict], root: Path) -> None:
    if not cases:
        raise VerificationError("No behavioral cases supplied")
    ids = set()
    for case in cases:
        case_id = case.get("id")
        if not case_id or case_id in ids:
            raise VerificationError(f"Empty/duplicate test id: {case_id!r}")
        ids.add(case_id)
        if case.get("state") != "ready":
            raise VerificationError(f"{case_id}: state is TODO; release gate cannot pass")
        if ("REPLACE_WITH" in json.dumps(case, ensure_ascii=False)
                or "MUST_BE_DONE" in json.dumps(case, ensure_ascii=False)):
            raise VerificationError(f"{case_id}: unresolved placeholder in ready case")
        if not case.get("tool_slug") or case["tool_slug"].startswith("REPLACE_"):
            raise VerificationError(f"{case_id}: missing actual tool slug")
        if not case.get("inputs"):
            raise VerificationError(f"{case_id}: missing real input fixture")
        if case.get("expect") not in ("SUCCESS", "ERROR"):
            raise VerificationError(f"{case_id}: expect must be SUCCESS or ERROR")
        for entry in case["inputs"]:
            fixture = root / entry["path"]
            if not fixture.is_file():
                raise VerificationError(f"{case_id}: missing fixture: {fixture}")
            digest = entry.get("sha256", "")
            if not re.fullmatch(r"[0-9a-f]{64}", digest):
                raise VerificationError(f"{case_id}: fixture sha256 missing/invalid")
            if sha256(fixture) != digest:
                raise VerificationError(f"{case_id}: fixture SHA256 mismatch")
        if case["expect"] == "SUCCESS":
            expected = case.get("output", {})
            if expected.get("count", 0) < 1:
                raise VerificationError(f"{case_id}: require positive expected output count")
            if expected.get("type") not in ("pdf", "png", "jpeg", "gif", "webp", "text"):
                raise VerificationError(f"{case_id}: unsupported/missing output type")
            if expected["type"] == "pdf" and not (expected.get("pages") or expected.get("tokens")):
                raise VerificationError(f"{case_id}: PDF output must have independent page/text oracle")
            if case_id == "PDF-SIGN-CMS-VERIFIED" and not expected.get("signature_valid", False):
                raise VerificationError(f"{case_id}: must enable independent pdfsig cryptographic validation")
        else:
            if not case.get("error_code"):
                raise VerificationError(f"{case_id}: negative case must assert an error_code")
    missing = REQUIRED_CASE_IDS - ids
    if missing:
        raise VerificationError(f"Missing mandatory case ids: {', '.join(sorted(missing))}")


def execute_case(root: Path, case: dict, adapter: list[str]) -> None:
    case_id = case["id"]
    with tempfile.TemporaryDirectory(prefix="office-verification-") as folder:
        work = Path(folder).resolve()
        sources = work / "source"
        outputs = work / "output"
        sources.mkdir()
        outputs.mkdir()
        initial_outputs: dict[str, str] = {}
        setup = case.get("setup", {})
        for entry in setup.get("precreate_files", []):
            relative = Path(entry["path"])
            if relative.is_absolute() or ".." in relative.parts:
                raise VerificationError(f"{case_id}: unsafe precreated output path")
            target = (outputs / relative).resolve()
            if not target.is_relative_to(outputs):
                raise VerificationError(f"{case_id}: precreated output path escapes sandbox")
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(entry.get("text", "EXISTING_SENTINEL"), encoding="utf-8")
            initial_outputs[relative.as_posix()] = sha256(target)
        copied = []
        for idx, fixture in enumerate(case["inputs"]):
            original = root / fixture["path"]
            dst = sources / f"{idx:02d}_{original.name}"
            shutil.copy2(original, dst)
            copied.append((dst, sha256(dst)))
        request = {
            "case_id": case_id,
            "tool_slug": case["tool_slug"],
            "input_paths": [str(path) for path, _ in copied],
            "output_dir": str(outputs),
            "options": case.get("options", {}),
        }
        response_raw = run_checked(adapter, timeout=int(case.get("timeout_seconds", 120)),
                                   input_text=json.dumps(request, ensure_ascii=False) + "\n")
        try:
            response = json.loads(response_raw)
        except json.JSONDecodeError as exc:
            raise VerificationError(f"{case_id}: adapter did not return strict JSON") from exc
        if not isinstance(response, dict) or response.get("status") != case["expect"]:
            raise VerificationError(f"{case_id}: expected {case['expect']}, got {response}")
        for source, digest in copied:
            if sha256(source) != digest:
                raise VerificationError(f"{case_id}: input was modified: {source}")
        paths = response.get("output_paths")
        if not isinstance(paths, list):
            raise VerificationError(f"{case_id}: output_paths must be an array")
        if case["expect"] == "ERROR":
            if paths or response.get("error_code") != case["error_code"]:
                raise VerificationError(f"{case_id}: failure must report correct error_code and no outputs")
            actual_outputs = {p.relative_to(outputs).as_posix(): sha256(p)
                              for p in outputs.rglob("*") if p.is_file()}
            if actual_outputs != initial_outputs:
                raise VerificationError(f"{case_id}: failed operation modified preexisting files or left staged output")
            return
        expected = case["output"]
        if len(paths) != expected["count"]:
            raise VerificationError(f"{case_id}: expected {expected['count']} outputs, got {len(paths)}")
        resolved_paths = []
        for output in paths:
            p = Path(output).resolve()
            if not p.is_relative_to(outputs):
                raise VerificationError(f"{case_id}: output path outside temp output dir: {p}")
            if p in resolved_paths:
                raise VerificationError(f"{case_id}: duplicate output: {p}")
            resolved_paths.append(p)
            check_output(p, expected, case_id)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--cases", type=Path, default=Path(__file__).resolve().parent.parent / "tests/verification/behavior_cases.json")
    parser.add_argument("--validate-only", action="store_true", help="Fail if manifest has TODO/missing fixtures; does not run Engine")
    args = parser.parse_args()
    root = args.repo.resolve()
    try:
        cases = json.loads(args.cases.read_text(encoding="utf-8"))["cases"]
        validate_manifest(cases, root)
        if args.validate_only:
            print(f"Manifest ready: {len(cases)} real fixture contract(s). NOTE: Engine not executed.")
            return 0
        command = os.environ.get("OFFICE_VERIFY_ADAPTER", "").strip()
        if not command:
            raise VerificationError("OFFICE_VERIFY_ADAPTER is not configured; FAIL-CLOSED")
        adapter = shlex.split(command)
        for case in cases:
            execute_case(root, case, adapter)
            print(f"PASS {case['id']}")
        print(f"PASS all {len(cases)} behavioral contract(s) via production adapter")
        return 0
    except (VerificationError, FileNotFoundError, OSError, ValueError, KeyError) as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
