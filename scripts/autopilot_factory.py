#!/usr/bin/env python3
"""
Dây chuyền Sản xuất Tự hành (Autonomous Software Factory) cho Office Utility VN.
Tự động: Bóc tách 02-srs.md -> Điều phối Antigravity/Harness2 -> Verify 4 Cổng -> Auto Git Commit.
"""

import json
import re
import subprocess
import time
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor

# Import AgHarnessAgent đã thiết lập ở bước trước
import sys
sys.path.append(str(Path("../harness2").resolve()))
from ag_substrate import AgHarnessAgent

WORKSPACE_DIR = Path(".").resolve()
SRS_FILE = (
    WORKSPACE_DIR / "docs" / "specs" / "02-srs.md"
    if (WORKSPACE_DIR / "docs" / "specs" / "02-srs.md").exists()
    else WORKSPACE_DIR / "02-srs.md"
)
QUEUE_FILE = WORKSPACE_DIR / ".harness" / "tasks_queue.jsonl"
DLQ_FILE = WORKSPACE_DIR / ".harness" / "dead_letter_queue.json"
MAX_RETRIES_PER_TOOL = 3


def extract_tasks_from_srs() -> list[dict]:
    """Tự động quét toàn bộ các dòng bảng FR-* trong 02-srs.md thành danh sách tác vụ."""
    content = SRS_FILE.read_text(encoding="utf-8")
    pattern = re.compile(
        r"\|\s*\*\*(FR-[A-Z]+-\d+)\*\*\s*\|\s*([^|]+)\|\s*`([^`]+)`\s*\|\s*([^|]+)\|\s*([^|]+)\|\s*([^|]+)\|\s*([^|]+)\|"
    )
    tasks = []
    for match in pattern.finditer(content):
        fr_id, name, slug, crates, inputs, outputs, desc = [m.strip() for m in match.groups()]
        domain = fr_id.split("-")[1].lower()  # doc, med, dat, vno, sys, hub, utl
        tasks.append({
            "fr_id": fr_id,
            "domain": domain,
            "name": name,
            "slug": slug,
            "crates": crates,
            "inputs": inputs,
            "outputs": outputs,
            "description": desc,
        })

    QUEUE_FILE.parent.mkdir(parents=True, exist_ok=True)
    with QUEUE_FILE.open("w", encoding="utf-8") as f:
        for t in tasks:
            f.write(json.dumps(t, ensure_ascii=False) + "\n")
    print(f"✅ Đã trích xuất tự động {len(tasks)} công cụ từ 02-srs.md vào {QUEUE_FILE}")
    return tasks


def is_tool_already_completed(slug: str) -> bool:
    """Kiểm tra trong git log xem công cụ này đã được hiện thực và commit thành công chưa (Hỗ trợ Resume)."""
    res = subprocess.run(
        ["git", "log", "--grep", f"feat(pod): {slug}", "--oneline"],
        cwd=WORKSPACE_DIR, capture_output=True, text=True
    )
    return bool(res.stdout.strip())


def run_verify_gate(slug: str) -> tuple[bool, str]:
    """Chạy cảm biến 4 cổng kiểm định: cargo fmt, cargo clippy -D warnings, NFR grep, cargo test."""
    test_name = slug.replace("-", "_")
    proc = subprocess.run(
        ["./scripts/verify_pod.sh", test_name],
        cwd=WORKSPACE_DIR, capture_output=True, text=True
    )
    return (proc.returncode == 0, proc.stdout + "\n" + proc.stderr)


def build_pod_prompt(task: dict, compiler_error: str = "") -> str:
    """Tạo prompt cô lập ngữ cảnh cho đúng 1 công cụ (Bounded Pod Context)."""
    base_prompt = f"""
Hãy hiện thực hóa hoàn chỉnh công cụ sau theo đúng kiến trúc OfficeTool trait (@/workspace/.agents/skills/office-tool-pod/SKILL.md)
và giao diện Vùng 2 Leptos 0.8 (@/workspace/.agents/skills/leptos-3zone-ui/SKILL.md):

- Mã Yêu cầu: {task['fr_id']}
- Tên Công cụ: {task['name']}
- Slug định danh: `{task['slug']}`
- Động cơ Crate Rust bắt buộc: {task['crates']}
- Đầu vào: {task['inputs']} | Đầu ra: {task['outputs']}
- Đặc tả kỹ thuật: {task['description']}

YÊU CẦU THỰC THI:
1. Viết Unit Test & Security Test (Magic Bytes `infer`, Graceful Cancellation <100ms) tại `crates/office-engine/tests/{task['slug'].replace('-', '_')}_test.rs`.
2. Viết module thực thi tại `crates/office-engine/src/modules/{task['domain']}/{task['slug'].replace('-', '_')}.rs`.
3. Viết Component cấu hình Vùng 2 tại `crates/office-ui/src/tools/{task['slug'].replace('-', '_')}_view.rs` và đăng ký slug `{task['slug']}` vào Tool Registry.
"""
    if compiler_error:
        base_prompt += f"\n⚠️ LỖI BIÊN DỊCH / KIỂM THỬ Ở LƯỢT TRƯỚC (HÃY SỬA TRIỆT ĐỂ):\n```\n{compiler_error[-3500:]}\n```\n"
    return base_prompt


def process_single_tool(task: dict, agent: AgHarnessAgent) -> bool:
    slug = task["slug"]
    if is_tool_already_completed(slug):
        print(f"⏭️  Bỏ qua [{task['fr_id']}] `{slug}` (Đã hoàn thành trong Git history).")
        return True

    print(f"🚀 Đang sản xuất [{task['fr_id']}] `{slug}` ({task['name']})...")
    last_error = ""

    for attempt in range(1, MAX_RETRIES_PER_TOOL + 1):
        prompt = build_pod_prompt(task, last_error)
        agent.run_task(task_id=f"{slug}-try{attempt}", task_prompt=prompt)

        passed, gate_log = run_verify_gate(slug)
        if passed:
            # Tự động commit công cụ đạt chuẩn 100% vào Git
            subprocess.run(["git", "add", "."], cwd=WORKSPACE_DIR, check=True)
            subprocess.run(
                ["git", "commit", "-m", f"feat(pod): {slug} ({task['fr_id']}) - passed 4 quality gates"],
                cwd=WORKSPACE_DIR, check=True
            )
            print(f"✅ Hoàn tất & Đã Commit [{task['fr_id']}] `{slug}` ở lượt thử {attempt}/{MAX_RETRIES_PER_TOOL}!")
            return True

        print(f"⚠️  [{task['fr_id']}] `{slug}` thất bại cổng kiểm định ở lượt {attempt}. Đang gửi log lỗi để Agent tự sửa...")
        last_error = gate_log

    # Nếu quá 3 lượt vẫn lỗi: Hoàn tác các thay đổi dở dang và đẩy vào Dead-Letter Queue
    subprocess.run(["git", "checkout", "--", "."], cwd=WORKSPACE_DIR)
    subprocess.run(["git", "clean", "-fd", "crates/"], cwd=WORKSPACE_DIR)
    print(f"❌ Đã cô lập [{task['fr_id']}] `{slug}` vào Dead-Letter Queue sau {MAX_RETRIES_PER_TOOL} lượt thử.")
    return False


def main():
    tasks = extract_tasks_from_srs()
    agent = AgHarnessAgent(
        workspace_dir=WORKSPACE_DIR,
        harness_dir=WORKSPACE_DIR / ".harness",
        trajectory_save_dir=Path("../harness2/runs/autopilot_trajectories"),
        model_id="gemini-3.8-flash",
    )

    failed_tools = []
    for task in tasks:
        ok = process_single_tool(task, agent)
        if not ok:
            failed_tools.append({
                "fr_id": task["fr_id"],
                "slug": task["slug"],
                "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ"),
            })
            DLQ_FILE.write_text(json.dumps(failed_tools, indent=2, ensure_ascii=False), encoding="utf-8")

    print(f"\n🏁 KẾT THÚC DÂY CHUYỀN: Thành công {len(tasks) - len(failed_tools)}/{len(tasks)} công cụ.")
    if failed_tools:
        print(f"📋 Xem danh sách {len(failed_tools)} công cụ cần tinh chỉnh thêm tại: {DLQ_FILE}")


if __name__ == "__main__":
    main()