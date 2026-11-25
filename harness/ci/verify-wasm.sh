#!/usr/bin/env bash
# ==============================================================================
# harness/ci/verify-wasm.sh
# Verification Harness: L4 Frontend WebAssembly (Leptos 0.8 CSR) Compile Runner
# ==============================================================================
set -euo pipefail

COLOR_RESET="\033[0m"
COLOR_BOLD="\033[1m"
COLOR_GREEN="\033[32m"
COLOR_YELLOW="\033[33m"
COLOR_RED="\033[31m"
COLOR_CYAN="\033[36m"

echo -e "${COLOR_BOLD}${COLOR_CYAN}=== [Harness L4] Frontend WASM Compilation Runner ===${COLOR_RESET}"

# 1. Kiểm tra target wasm32-unknown-unknown
echo -e "${COLOR_YELLOW}🌐 Kiểm tra rust target wasm32-unknown-unknown...${COLOR_RESET}"
if ! rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
  echo "Cài đặt target wasm32-unknown-unknown..."
  rustup target add wasm32-unknown-unknown
fi

# 2. Kiểm tra Trunk
if ! command -v trunk &> /dev/null; then
  echo -e "${COLOR_RED}⚠️ Trunk chưa được cài đặt. Thử cài đặt nhanh...${COLOR_RESET}"
  cargo install trunk --locked || {
    echo -e "${COLOR_RED}❌ Không thể cài đặt trunk!${COLOR_RESET}"
    exit 1
  }
fi

# 3. Biên dịch bản build WebAssembly Release
echo -e "${COLOR_YELLOW}📦 Đang đóng gói WASM frontend qua Trunk (trunk build --release)...${COLOR_RESET}"
trunk build --release

# 4. Kiểm tra sự tồn tại của file WASM và HTML trong thư mục dist/
if [[ -d "dist" ]] && ls dist/*.wasm 1> /dev/null 2>&1; then
  WASM_SIZE=$(du -h dist/*.wasm | cut -f1)
  echo -e "${COLOR_GREEN}✅ WASM bundle được tạo thành công! (Kích thước: ${WASM_SIZE})${COLOR_RESET}"
else
  echo -e "${COLOR_RED}❌ Thất bại: Không tìm thấy file .wasm trong thư mục dist/!${COLOR_RESET}"
  exit 1
fi

echo -e "\n${COLOR_BOLD}${COLOR_GREEN}🎉 [L4] Frontend WASM build đạt chuẩn!${COLOR_RESET}"
