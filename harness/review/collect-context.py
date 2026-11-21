#!/usr/bin/env python3
"""
harness/review/collect-context.py
Thu thập ngữ cảnh thay đổi trong PR hoặc working tree:
- Danh sách file đã sửa đổi, thêm mới, xóa
- Phát hiện các vùng nhạy cảm (Spine, Persistence, IPC, Vendor)
- Phục vụ quá trình review tự động của các reviewer agent
"""

import subprocess
import json
import os
import sys

def run_cmd(cmd: list[str]) -> str:
    result = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    return result.stdout.strip()

def main():
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
    os.chdir(repo_root)

    # 1. Git status
    changed_files_raw = run_cmd(["git", "status", "--porcelain"])
    changed_files = []
    for line in changed_files_raw.splitlines():
        if len(line) > 3:
            status = line[:2].strip()
            filepath = line[3:].strip()
            changed_files.append({"status": status, "path": filepath})

    # 2. Git diff stats
    diff_stat = run_cmd(["git", "diff", "--stat"])

    # 3. Phân loại vùng bị ảnh hưởng
    impacted_domains = set()
    spine_violation = False
    persistence_impact = False
    ipc_impact = False
    vendor_impact = False

    for item in changed_files:
        p = item["path"]
        if p.startswith("crates/office-shared/") or p.startswith("src-tauri/src/db/") or p.startswith("src-tauri/src/commands/"):
            spine_violation = True
        if "persistence" in p or "safe_persistence" in p:
            persistence_impact = True
        if p.startswith("src-tauri/src/commands/") or p.startswith("src-tauri/capabilities/"):
            ipc_impact = True
        if p.startswith("vendor/") or p.startswith("patches/"):
            vendor_impact = True

        if "doc_suite" in p or "pdf" in p:
            impacted_domains.add("doc-suite")
        elif "media_studio" in p or "image" in p:
            impacted_domains.add("media-studio")
        elif "data_lab" in p or "gridcraft" in p:
            impacted_domains.add("data-lab")
        elif "vn_office" in p or "vietnamese" in p:
            impacted_domains.add("vn-office")
        elif "sys_hub" in p:
            impacted_domains.add("sys-hub")

    context = {
        "repo_root": repo_root,
        "changed_files_count": len(changed_files),
        "changed_files": changed_files,
        "diff_stat": diff_stat,
        "impacted_domains": sorted(list(impacted_domains)),
        "flags": {
            "spine_violation": spine_violation,
            "persistence_impact": persistence_impact,
            "ipc_impact": ipc_impact,
            "vendor_impact": vendor_impact,
        }
    }

    output_json = json.dumps(context, indent=2, ensure_ascii=False)
    if len(sys.argv) > 1 and sys.argv[1] == "--json":
        print(output_json)
    else:
        print("=== Review Context Summary ===")
        print(f"Total changed files: {len(changed_files)}")
        print(f"Impacted domains: {', '.join(impacted_domains) if impacted_domains else 'None'}")
        print("Security & Integrity Flags:")
        for k, v in context["flags"].items():
            icon = "🚨" if v else "✅"
            print(f"  {icon} {k}: {v}")
        print("\nDiff Stats:\n" + diff_stat)

if __name__ == "__main__":
    main()
