#!/usr/bin/env bash
# ==============================================================================
# harness/review/run-ai-review.sh
# Quy trình Local 4-Pass AI Review chuyên sâu với Gemini 3.8 Flash HIGH
# ==============================================================================
set -euo pipefail

COLOR_RESET="\033[0m"
COLOR_BOLD="\033[1m"
COLOR_GREEN="\033[32m"
COLOR_YELLOW="\033[33m"
COLOR_RED="\033[31m"
COLOR_CYAN="\033[36m"

echo -e "${COLOR_BOLD}${COLOR_CYAN}=== [AI Review] 4-Pass Deep Review Workflow (Gemini 3.8 Flash High) ===${COLOR_RESET}"

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${PROJECT_ROOT}"

# 1. Kiểm tra Antigravity CLI và Model Policy
echo -e "\n${COLOR_YELLOW}🔍 [Step 1] Kiểm tra phiên bản Antigravity CLI & Model Policy...${COLOR_RESET}"
if ! command -v agy &> /dev/null; then
  echo -e "${COLOR_RED}❌ Không tìm thấy công cụ agy (Antigravity CLI)!${COLOR_RESET}"
  exit 1
fi

AGY_VERSION="$(agy --version || echo 'unknown')"
echo -e "   Phiên bản CLI: ${COLOR_GREEN}${AGY_VERSION}${COLOR_RESET}"

# 2. Xác định các commit SHA
HEAD_SHA="$(git rev-parse HEAD)"
BASE_SHA="$(git merge-base origin/main HEAD 2>/dev/null || git rev-parse HEAD~1)"
echo -e "   Base SHA: ${COLOR_CYAN}${BASE_SHA:0:8}${COLOR_RESET}"
echo -e "   HEAD SHA: ${COLOR_CYAN}${HEAD_SHA:0:8}${COLOR_RESET}"

# 3. Tạo isolated git worktree cho phiên review
REVIEW_WORKTREE="../office-review-${HEAD_SHA:0:8}"
echo -e "\n${COLOR_YELLOW}📂 [Step 2] Thiết lập worktree tách biệt tại: ${REVIEW_WORKTREE}...${COLOR_RESET}"
if [[ -d "${REVIEW_WORKTREE}" ]]; then
  echo "   Dọn dẹp worktree cũ..."
  git worktree remove --force "${REVIEW_WORKTREE}" 2>/dev/null || rm -rf "${REVIEW_WORKTREE}"
fi

git worktree add --detach "${REVIEW_WORKTREE}" "${HEAD_SHA}"

# Đảm bảo dọn dẹp worktree khi script thoát
cleanup() {
  if [[ "${KEEP_WORKTREE:-0}" -eq 0 && -d "${REVIEW_WORKTREE}" ]]; then
    echo -e "\n${COLOR_YELLOW}🧹 Dọn dẹp review worktree...${COLOR_RESET}"
    git worktree remove --force "${REVIEW_WORKTREE}" 2>/dev/null || true
  fi
}
trap cleanup EXIT

# 4. Thực thi 4 lượt review qua python engine
echo -e "\n${COLOR_YELLOW}🧠 [Step 3] Khởi chạy 4 lượt review (Correctness -> Security -> Integration -> Verification)...${COLOR_RESET}"
python3 harness/review/ai_review_engine.py \
  --worktree "${REVIEW_WORKTREE}" \
  --base-sha "${BASE_SHA}" \
  --head-sha "${HEAD_SHA}" \
  --model "gemini-3.8-flash-high" \
  --effort "high"

echo -e "\n${COLOR_BOLD}${COLOR_GREEN}🎉 Quy trình 4-Pass AI Review hoàn tất thành công!${COLOR_RESET}"
