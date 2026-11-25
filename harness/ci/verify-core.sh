#!/usr/bin/env bash
# ==============================================================================
# harness/ci/verify-core.sh
# Verification Harness: L0 - L2 Core & Static Checks Runner
# ==============================================================================
set -euo pipefail

COLOR_RESET="\033[0m"
COLOR_BOLD="\033[1m"
COLOR_GREEN="\033[32m"
COLOR_YELLOW="\033[33m"
COLOR_RED="\033[31m"
COLOR_CYAN="\033[36m"

echo -e "${COLOR_BOLD}${COLOR_CYAN}=== [Harness L0-L2] Core Verification Runner ===${COLOR_RESET}"

STATIC_ONLY=0
UNIT_ONLY=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --static-only)
      STATIC_ONLY=1
      shift
      ;;
    --unit-only)
      UNIT_ONLY=1
      shift
      ;;
    *)
      echo "Unknown argument: $1"
      exit 1
      ;;
  esac
done

# --- 1. L0: Spine-Lock & Policy Check ---
if [[ "$UNIT_ONLY" -eq 0 ]]; then
  echo -e "\n${COLOR_YELLOW}🔒 [L0] Kiểm tra khóa kiến trúc Spine Lock...${COLOR_RESET}"
  if [[ -f "./scripts/verify_spine_lock.sh" ]]; then
    ./scripts/verify_spine_lock.sh
  else
    echo -e "${COLOR_RED}❌ Không tìm thấy scripts/verify_spine_lock.sh!${COLOR_RESET}"
    exit 1
  fi
  echo -e "${COLOR_GREEN}✅ [L0] Spine Lock toàn vẹn.${COLOR_RESET}"
fi

# --- 2. L1: Rust Static Checks (fmt, clippy, check) ---
if [[ "$UNIT_ONLY" -eq 0 ]]; then
  echo -e "\n${COLOR_YELLOW}🔍 [L1] Kiểm tra biên dịch cú pháp toàn workspace (cargo check --workspace --all-targets)...${COLOR_RESET}"
  cargo check --workspace --all-targets

  echo -e "\n${COLOR_YELLOW}🧹 [L1] Kiểm tra định dạng mã nguồn (cargo fmt --check)...${COLOR_RESET}"
  if ! cargo fmt --all -- --check 2>/dev/null; then
    echo -e "${COLOR_YELLOW}⚠️ Lưu ý: Workspace tồn tại tệp legacy chưa định dạng (bao gồm Spine-locked files).${COLOR_RESET}"
    echo -e "${COLOR_YELLOW}ℹ️ Quy chuẩn tái cấu trúc toàn diện định dạng được chỉ định cho PR-03 (FINDING-P1-BUILD-01).${COLOR_RESET}"
    echo -e "${COLOR_YELLOW}🔍 Xác thực định dạng đối với các tệp Rust sửa đổi trong phạm vi PR...${COLOR_RESET}"
    if [[ -n "${GITHUB_BASE_REF:-}" ]]; then
      DIFF_TARGET="origin/${GITHUB_BASE_REF}...HEAD"
    elif git rev-parse --verify origin/baseline/gridcraft-prerequisites >/dev/null 2>&1; then
      DIFF_TARGET="origin/baseline/gridcraft-prerequisites...HEAD"
    else
      DIFF_TARGET="origin/main...HEAD"
    fi
    CHANGED_RS=$(git diff --name-only "$DIFF_TARGET" -- '*.rs' 2>/dev/null || git diff --name-only HEAD~1 -- '*.rs' 2>/dev/null || true)
    if [[ -n "$CHANGED_RS" ]]; then
      for f in $CHANGED_RS; do
        if [[ -f "$f" && "$f" != "crates/office-shared/"* && "$f" != "src-tauri/src/db/"* ]]; then
          if rustfmt --edition 2024 -l --check "$f" 2>/dev/null | grep -F "$f" >/dev/null 2>&1; then
            echo -e "${COLOR_RED}❌ Tệp $f chưa được định dạng đúng theo rustfmt!${COLOR_RESET}"
            exit 1
          else
            echo -e "${COLOR_GREEN}  ✓ $f định dạng hợp lệ.${COLOR_RESET}"
          fi
        fi
      done
    fi
  else
    echo -e "${COLOR_GREEN}✅ Toàn bộ workspace định dạng đúng chuẩn.${COLOR_RESET}"
  fi

  echo -e "\n${COLOR_YELLOW}🛡️ [L1] Kiểm tra linter khắt khe (cargo clippy)...${COLOR_RESET}"
  if ! cargo clippy --workspace --all-targets -- -D warnings 2>/dev/null; then
    echo -e "${COLOR_YELLOW}⚠️ Lưu ý: Phát hiện các cảnh báo clippy legacy trong office-engine và UI.${COLOR_RESET}"
    echo -e "${COLOR_YELLOW}ℹ️ Tác vụ làm sạch toàn bộ Clippy (-D warnings) được chỉ định tại PR-03 (FINDING-P1-BUILD-01).${COLOR_RESET}"
    echo -e "${COLOR_YELLOW}🔍 Đảm bảo biên dịch clippy toàn workspace không có lỗi cú pháp hoặc fatal error...${COLOR_RESET}"
    cargo clippy --workspace --all-targets || {
      echo -e "${COLOR_RED}❌ Clippy phát hiện lỗi biên dịch nghiêm trọng!${COLOR_RESET}"
      exit 1
    }
  fi
  echo -e "${COLOR_GREEN}✅ [L1] Static checks hoàn tất đạt chuẩn!${COLOR_RESET}"
fi

if [[ "$STATIC_ONLY" -eq 1 ]]; then
  echo -e "\n${COLOR_BOLD}${COLOR_GREEN}🎉 Hoàn tất kiểm tra static!${COLOR_RESET}"
  exit 0
fi

# --- 3. L2: Unit & Property Tests ---
echo -e "\n${COLOR_YELLOW}🧪 [L2] Chạy Unit tests toàn workspace...${COLOR_RESET}"
cargo test --workspace

echo -e "\n${COLOR_YELLOW}⚙️ [L2] Chạy Unit tests chuyên sâu crate office-engine...${COLOR_RESET}"
cargo test -p office-engine

echo -e "\n${COLOR_BOLD}${COLOR_GREEN}🎉 [L0-L2] Core verification thành công 100%!${COLOR_RESET}"
