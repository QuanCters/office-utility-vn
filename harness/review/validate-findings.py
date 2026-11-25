#!/usr/bin/env python3
"""
harness/review/validate-findings.py
Kiểm định tính hợp lệ của các finding phát hiện bởi reviewer agent:
- Kiểm tra schema JSON
- Khử trùng lặp (deduplication)
- Xác thực file và line range thực sự tồn tại trong repository
- Đảm bảo bằng chứng cụ thể, loại bỏ false positives
"""

import sys
import os
import json
import re

def validate_finding(finding: dict, repo_root: str) -> list[str]:
    errors = []
    
    # 1. Required fields
    required_fields = ["id", "title", "severity", "category", "file", "line_range", "evidence", "recommendation"]
    for field in required_fields:
        if field not in finding:
            errors.append(f"Missing required field: '{field}'")

    if errors:
        return errors

    # 2. Severity check
    valid_severities = {"CRITICAL", "HIGH", "MEDIUM", "LOW", "INFO"}
    if finding["severity"] not in valid_severities:
        errors.append(f"Invalid severity '{finding['severity']}'. Must be one of {valid_severities}")

    # 3. Category check
    valid_categories = {"CORRECTNESS", "SECURITY", "INTEGRATION", "PERFORMANCE", "CODE_STYLE"}
    if finding["category"] not in valid_categories:
        errors.append(f"Invalid category '{finding['category']}'. Must be one of {valid_categories}")

    # 4. File existence & line bounds
    file_path = os.path.join(repo_root, finding["file"])
    if not os.path.isfile(file_path):
        errors.append(f"File '{finding['file']}' does not exist in repository")
    else:
        line_range = finding.get("line_range", [])
        if not (isinstance(line_range, list) and len(line_range) == 2 and line_range[0] <= line_range[1]):
            errors.append(f"Invalid line_range format: {line_range}. Expected [start_line, end_line]")
        else:
            try:
                with open(file_path, "r", encoding="utf-8", errors="ignore") as f:
                    total_lines = sum(1 for _ in f)
                if line_range[0] < 1 or line_range[1] > total_lines:
                    errors.append(f"Line range {line_range} is out of bounds for '{finding['file']}' (total lines: {total_lines})")
            except Exception as e:
                errors.append(f"Cannot read file '{finding['file']}': {e}")

    # 5. Evidence check
    if not finding["evidence"] or len(finding["evidence"].strip()) < 10:
        errors.append("Evidence is too short or empty. Must provide concrete code/log citation.")

    return errors

def main():
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <findings.json>")
        sys.exit(1)

    findings_file = sys.argv[1]
    if not os.path.isfile(findings_file):
        print(f"Error: file not found: {findings_file}")
        sys.exit(1)

    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))

    with open(findings_file, "r", encoding="utf-8") as f:
        try:
            data = json.load(f)
        except json.JSONDecodeError as e:
            print(f"❌ JSON syntax error in {findings_file}: {e}")
            sys.exit(1)

    findings = data if isinstance(data, list) else data.get("findings", [])

    print(f"🔍 Validating {len(findings)} findings against repository at: {repo_root}")

    seen_ids = set()
    total_errors = 0

    for i, finding in enumerate(findings):
        f_id = finding.get("id", f"Finding #{i+1}")
        if f_id in seen_ids:
            print(f"❌ Duplicate finding ID: {f_id}")
            total_errors += 1
        seen_ids.add(f_id)

        errs = validate_finding(finding, repo_root)
        if errs:
            total_errors += len(errs)
            print(f"❌ Validation errors in [{f_id}]:")
            for err in errs:
                print(f"   - {err}")
        else:
            print(f"✅ [{finding['severity']}] {f_id}: {finding['title']}")

    if total_errors > 0:
        print(f"\n❌ Validation failed with {total_errors} errors!")
        sys.exit(1)
    else:
        print(f"\n🎉 All {len(findings)} findings validated successfully!")
        sys.exit(0)

if __name__ == "__main__":
    main()
