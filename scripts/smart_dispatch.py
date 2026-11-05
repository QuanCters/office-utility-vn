#!/usr/bin/env python3
"""
Smart Triage Router & Deterministic Gatekeeper cho Office Utility VN.
Cách dùng: python3 scripts/smart_dispatch.py "<Yêu cầu bằng tiếng Việt tự nhiên>"
"""

import json
import subprocess
import sys
from pathlib import Path

sys.path.append(str(Path("../harness2").resolve()))
from ag_substrate import AgHarnessAgent

WORKSPACE_DIR = Path(".").resolve()

# Định nghĩa ranh giới bảo vệ cứng (Deterministic Rules) cho từng loại tác vụ
TASK_POLICIES = {
    "add-tools": {
        "zone": "GREEN",
        "protected_paths": ["crates/office-shared", "src-tauri/src/db"],
        "require_bench": False,
    },
    "modify": {
        "zone": "GREEN",
        "protected_paths": ["crates/office-shared", "src-tauri/src/db"],
        "require_bench": False,
    },
    "legal-rules": {
        "zone": "GREEN",
        "protected_paths": ["crates/office-shared", "src-tauri/src/db"],
        "require_bench": False,
    },
    "fix-bug": {
        "zone": "GREEN",
        "protected_paths": ["crates/office-shared", "src-tauri/src/db"],
        "require_bench": False,
    },
    "refactor": {
        "zone": "GREEN",
        # Khóa cứng thư mục tests/ để cấm Agent tự sửa Unit Test khi refactor!
        "protected_paths": ["crates/office-shared", "src-tauri/src/db", "crates/office-engine/tests"],
        "require_bench": True,
    },
    "perf-tuning": {
        "zone": "GREEN",
        "protected_paths": ["crates/office-shared", "src-tauri/src/db", "crates/office-engine/tests"],
        "require_bench": True,
    },
    "sync-docs": {
        "zone": "GREEN",
        "protected_paths": ["crates/", "src-tauri/"],
        "require_bench": False,
    },
    "spine-upgrade": {"zone": "YELLOW", "protected_paths": [], "require_bench": False},
    "deps-upgrade":  {"zone": "YELLOW", "protected_paths": [], "require_bench": False},
    "os-compat":     {"zone": "YELLOW", "protected_paths": [], "require_bench": False},
    "deprecate":     {"zone": "YELLOW", "protected_paths": [], "require_bench": False},
    "sec-fuzzing":   {"zone": "YELLOW", "protected_paths": [], "require_bench": False},
    "db-migration":  {"zone": "RED",    "protected_paths": [], "require_bench": False},
    "addon-pack":    {"zone": "RED",    "protected_paths": [], "require_bench": False},
    "release-hotfix":{"zone": "RED",    "protected_paths": [], "require_bench": False},
}


def classify_user_intent(user_request: str, agent: AgHarnessAgent) -> dict:
    """Bước A: Dùng Gemini 3.8 Flash High phân loại yêu cầu thành JSON Manifest chuẩn."""
    triage_prompt = f"""
Bạn là Bộ định tuyến Triage Router của dự án Office Utility VN.
KHÔNG sửa bất kỳ tệp nào. Hãy phân tích yêu cầu sau của người dùng và trả về DUY NHẤT một khối JSON:
Yêu cầu: "{user_request}"

Danh sách `task_type` hợp lệ:
{list(TASK_POLICIES.keys())}

Định dạng JSON trả về bắt buộc:
{{
  "task_type": "<1 trong 15 loại trên>",
  "target_slug": "<slug công cụ bị ảnh hưởng, ví dụ: vietqr-napas247-generator hoặc global>",
  "fr_id": "<Mã FR nếu có, ví dụ: FR-VNO-03>",
  "summary": "<Tóm tắt ngắn gọn hành động kỹ thuật>",
  "execution_steps": ["Bước 1...", "Bước 2..."]
}}
"""
    res = agent.run_task(task_id="triage-router", task_prompt=triage_prompt)
    raw_text = res["final_output"]
    start_idx = raw_text.find("{")
    end_idx = raw_text.rfind("}") + 1
    return json.loads(raw_text[start_idx:end_idx])


def verify_protected_paths_untouched(protected_paths: list[str]) -> tuple[bool, str]:
    """Kiểm tra bằng Git xem Agent có lén sửa vào các thư mục bị cấm (như sửa file test khi refactor) không."""
    if not protected_paths:
        return True, ""
    proc = subprocess.run(
        ["git", "diff", "--name-only", "--"] + protected_paths,
        cwd=WORKSPACE_DIR, capture_output=True, text=True
    )
    violated_files = proc.stdout.strip()
    if violated_files:
        # Lập tức khôi phục các tệp bị sửa trái phép
        subprocess.run(["git", "checkout", "--"] + protected_paths, cwd=WORKSPACE_DIR)
        return False, f"VI PHẠM RANH GIỚI HARNESS: Agent đã cố sửa các tệp bị khóa:\n{violated_files}"
    return True, ""


def execute_pipeline(user_request: str):
    agent = AgHarnessAgent(
        workspace_dir=WORKSPACE_DIR,
        harness_dir=WORKSPACE_DIR / ".harness",
        trajectory_save_dir=Path("../harness2/runs/smart_dispatch"),
        model_id="gemini-3.8-flash",
    )

    print("🧠 [Bước 1/4] Đang phân loại tác vụ và lập kế hoạch thực thi...")
    manifest = classify_user_intent(user_request, agent)
    task_type = manifest["task_type"]
    slug = manifest["target_slug"]
    policy = TASK_POLICIES.get(task_type, TASK_POLICIES["modify"])
    zone = policy["zone"]

    print(f"📌 Phân loại: [{task_type.upper()}] | Vùng tự trị: [{zone}] | Đích: `{slug}`")
    print(f"📝 Kế hoạch: {manifest['summary']}")

    # Nếu thuộc Vùng Đỏ (RED), yêu cầu xác nhận kế hoạch trước khi chạy
    if zone == "RED":
        confirm = input("⚠️  Tác vụ thuộc VÙNG ĐỎ (Đụng tới DB Migration / Ký số Release). Bạn có duyệt chạy không? [y/N]: ")
        if confirm.lower() != "y":
            print("🛑 Đã hủy tác vụ theo yêu cầu.")
            return

    # Nếu cần đo Benchmark đối chứng (refactor / perf-tuning), lưu mốc trước khi sửa
    if policy["require_bench"]:
        print("📊 Đang lưu mốc hiệu năng Criterion trước khi thay đổi (--save-baseline before_change)...")
        subprocess.run(["cargo", "bench", "--", "--save-baseline", "before_change"], cwd=WORKSPACE_DIR)

    # Bước B: Vòng lặp thực thi có kiểm soát
    last_error = ""
    for attempt in range(1, 4):
        print(f"⚙️  [Bước 2/4] Đang thực thi qua Antigravity Subagent (Lượt {attempt}/3)...")
        exec_prompt = f"""
Thực hiện tác vụ thuộc kịch bản SOP `{task_type}`:
- Yêu cầu gốc: {user_request}
- Công cụ đích: `{slug}` ({manifest.get('fr_id', '')})
- CÁC ĐƯỜNG DẪN BỊ KHÓA CỨNG (TUYỆT ĐỐI KHÔNG SỬA): {policy['protected_paths']}
- Quy trình bắt buộc: Tuân thủ tài liệu `08-maintenance-lifecycle-sop.md`, nạp đúng SKILL.md và cập nhật bài học vào `.harness/memory/EXPERIENCE.md` nếu là sửa lỗi.
"""
        if last_error:
            exec_prompt += f"\n⚠️ LỖI Ở LƯỢT TRƯỚC (HÃY KHẮC PHỤC):\n```\n{last_error[-3000:]}\n```"

        agent.run_task(task_id=f"{task_type}-{slug}-try{attempt}", task_prompt=exec_prompt)

        # Kiểm tra xem Agent có vi phạm các tệp bị khóa không
        boundary_ok, boundary_err = verify_protected_paths_untouched(policy["protected_paths"])
        if not boundary_ok:
            print(f"❌ {boundary_err}")
            last_error = boundary_err
            continue

        # Chạy 4 cổng kiểm định chất lượng
        print("🛡️  [Bước 3/4] Đang kiểm tra 4 cổng chất lượng (verify_pod.sh)...")
        test_filter = "" if slug == "global" else slug.replace("-", "_")
        proc = subprocess.run(
            ["./scripts/verify_pod.sh", test_filter],
            cwd=WORKSPACE_DIR, capture_output=True, text=True
        )

        if proc.returncode == 0:
            print("✅ Đã vượt qua 100% cổng kiểm định!")
            # Bước 4: Xử lý Commit theo Vùng Tự trị
            if zone == "GREEN":
                subprocess.run(["git", "add", "."], cwd=WORKSPACE_DIR, check=True)
                subprocess.run(
                    ["git", "commit", "-m", f"{task_type}({slug}): {manifest['summary']}"],
                    cwd=WORKSPACE_DIR, check=True
                )
                print(f"🎉 [Bước 4/4] VÙNG XANH: Đã tự động Git Commit hoàn tất!")
            else:
                subprocess.run(["git", "status", "-s"], cwd=WORKSPACE_DIR)
                approve = input("🟡 [Bước 4/4] VÙNG VÀNG/ĐỎ: Kiểm định đã Pass. Bạn có đồng ý Commit thay đổi này không? [y/N]: ")
                if approve.lower() == "y":
                    subprocess.run(["git", "add", "."], cwd=WORKSPACE_DIR, check=True)
                    subprocess.run(
                        ["git", "commit", "-m", f"{task_type}({slug}): {manifest['summary']}"],
                        cwd=WORKSPACE_DIR, check=True
                    )
                    print("🎉 Đã Commit thành công!")
            return

        last_error = proc.stdout + "\n" + proc.stderr
        print(f"⚠️  Chưa vượt qua cổng kiểm định ở lượt {attempt}. Đang tự động sửa lỗi...")

    print("❌ Thất bại sau 3 lượt thử. Đang khôi phục trạng thái Git sạch ban đầu...")
    subprocess.run(["git", "checkout", "--", "."], cwd=WORKSPACE_DIR)


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Cách dùng: python3 scripts/smart_dispatch.py \"<Yêu cầu bằng tiếng Việt>\"")
        sys.exit(1)
    execute_pipeline(sys.argv[1])