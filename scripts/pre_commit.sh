#!/usr/bin/env bash
# ==============================================================================
# Git Pre-Commit Hook - Office Utility VN
# Tự động xác thực Spine Lock & UI Design Tokens trước mỗi lần git commit
# ==============================================================================
set -euo pipefail

BOLD='\033[1m'
CYAN='\033[36m'
GREEN='\033[32m'
RED='\033[31m'
YELLOW='\033[33m'
RESET='\033[0m'

echo -e "\n${BOLD}${CYAN}🚀 [GIT PRE-COMMIT] Khởi chạy kiểm định tự động trước khi commit...${RESET}\n"

# 1. Kiểm định UI Tokens & 12px Ergonomic Floor
echo -e "${YELLOW}👉 Bước 1/2: Kiểm định UI Tokens & Đáy công thái học 12px (make verify-ui)...${RESET}"
if ! make verify-ui; then
    echo -e "\n${BOLD}${RED}❌ [PRE-COMMIT BỊ CHẶN] Phát hiện vi phạm Design Tokens hoặc Đáy 12px!${RESET}"
    echo -e "${RED}Vui lòng sửa các lỗi trên trước khi commit (hoặc dùng 'git commit --no-verify' nếu thực sự khẩn cấp).${RESET}\n"
    exit 1
fi

# 2. Kiểm định Khóa bảo vệ khung xương sống Spine Lock & Unit Tests
echo -e "\n${YELLOW}👉 Bước 2/2: Kiểm định tính toàn vẹn Spine Lock & Unit Tests (./scripts/verify_spine_lock.sh)...${RESET}"
if ! ./scripts/verify_spine_lock.sh; then
    echo -e "\n${BOLD}${RED}❌ [PRE-COMMIT BỊ CHẶN] Spine Lock bị vi phạm hoặc Unit Test thất bại!${RESET}"
    echo -e "${RED}Tầng khung xương sống (office-shared, db, commands) bị khóa chỉ đọc và mọi test phải PASS 100%.${RESET}\n"
    exit 1
fi

echo -e "\n${BOLD}${GREEN}🎉 [PRE-COMMIT HOÀN TẤT] Toàn bộ kiểm định hợp lệ! Tiến hành commit mã nguồn.${RESET}\n"
exit 0
