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
  echo -e "\n${COLOR_YELLOW}🧹 [L1] Kiểm tra định dạng mã nguồn (cargo fmt --check)...${COLOR_RESET}"
  cargo fmt --all -- --check || {
    echo -e "${COLOR_RED}❌ Mã nguồn chưa định dạng đúng. Hãy chạy 'make fmt' trước khi commit.${COLOR_RESET}"
    exit 1
  }

  echo -e "\n${COLOR_YELLOW}🔍 [L1] Kiểm tra biên dịch cú pháp toàn workspace (cargo check)...${COLOR_RESET}"
  cargo check --workspace

  echo -e "\n${COLOR_YELLOW}🛡️ [L1] Kiểm tra linter khắt khe (cargo clippy)...${COLOR_RESET}"
  cargo clippy --workspace --all-targets -- -D warnings || {
    echo -e "${COLOR_RED}❌ Clippy phát hiện cảnh báo hoặc vi phạm quy chuẩn!${COLOR_RESET}"
    exit 1
  }
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
