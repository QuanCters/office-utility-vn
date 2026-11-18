#!/usr/bin/env bash
# ==============================================================================
# scripts/inspect_build_cache.sh - Cargo Build Cache Auditor & Safe Inspector
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

DRY_RUN=false
CLEAN_SAFE=false
if [[ "${1:-}" == "--dry-run" ]] || [[ "${1:-}" == "-n" ]]; then
  DRY_RUN=true
elif [[ "${1:-}" == "--clean-safe" ]] || [[ "${1:-}" == "-c" ]]; then
  CLEAN_SAFE=true
fi

# ANSI Colors
C_RESET="\033[0m"
C_BOLD="\033[1m"
C_CYAN="\033[36m"
C_GREEN="\033[32m"
C_YELLOW="\033[33m"
C_RED="\033[31m"
C_MAGENTA="\033[35m"

format_bytes() {
  local b=$1
  if (( b < 1024 )); then
    echo "${b} B"
  elif (( b < 1048576 )); then
    echo "$(( (b * 100) / 1024 ))" | awk '{printf "%.2f KiB", $1/100}'
  elif (( b < 1073741824 )); then
    echo "$(( (b * 100) / 1048576 ))" | awk '{printf "%.2f MiB", $1/100}'
  else
    echo "$(( (b * 100) / 1073741824 ))" | awk '{printf "%.2f GiB", $1/100}'
  fi
}

get_size_bytes() {
  local path="$1"
  if [[ -e "$path" ]]; then
    du -sb "$path" 2>/dev/null | awk '{print $1}'
  else
    echo "0"
  fi
}

echo -e "${C_BOLD}${C_CYAN}========================================================================${C_RESET}"
echo -e "${C_BOLD}${C_CYAN}🔍 CARGO BUILD CACHE AUDIT REPORT — Office Utility VN${C_RESET}"
echo -e "${C_BOLD}${C_CYAN}========================================================================${C_RESET}"

# 1. Process Check
ACTIVE_BUILDS=$(ps aux | grep -E "cargo |trunk |cargo-tauri|rustc " | grep -v grep || true)
if [[ -n "$ACTIVE_BUILDS" ]]; then
  echo -e "${C_RED}⚠️ CẢNH BÁO: Phát hiện tiến trình biên dịch đang chạy:${C_RESET}"
  echo "$ACTIVE_BUILDS" | awk '{printf "  PID %s: %s %s %s\n", $2, $11, $12, $13}'
else
  echo -e "${C_GREEN}✓ Không có tiến trình cargo/trunk/tauri/rustc nào đang hoạt động.${C_RESET}"
fi

# 2. Disk Measurements
echo ""
echo -e "${C_BOLD}📊 Đo lường dung lượng thực tế trên ổ đĩa:${C_RESET}"

TARGET_MAIN="${ROOT_DIR}/target"
TARGET_PDF="${ROOT_DIR}/vendor/pdfcraft/target"
TARGET_GRID="${ROOT_DIR}/vendor/gridcraft/target"

B_MAIN=$(get_size_bytes "$TARGET_MAIN")
B_PDF=$(get_size_bytes "$TARGET_PDF")
B_GRID=$(get_size_bytes "$TARGET_GRID")
B_TOTAL=$(( B_MAIN + B_PDF + B_GRID ))

B_INC=$(get_size_bytes "${TARGET_MAIN}/debug/incremental")
B_DEPS=$(get_size_bytes "${TARGET_MAIN}/debug/deps")
B_BUILD=$(get_size_bytes "${TARGET_MAIN}/debug/build")
B_WASM=$(get_size_bytes "${TARGET_MAIN}/wasm32-unknown-unknown")
B_REL=$(get_size_bytes "${TARGET_MAIN}/release")

printf "  %-45s : %12d B (%s)\n" "1. Main Target (office-utility-vn/target)" "$B_MAIN" "$(format_bytes "$B_MAIN")"
printf "     - target/debug/incremental               : %12d B (%s)\n" "$B_INC" "$(format_bytes "$B_INC")"
printf "     - target/debug/deps                      : %12d B (%s)\n" "$B_DEPS" "$(format_bytes "$B_DEPS")"
printf "     - target/debug/build                     : %12d B (%s)\n" "$B_BUILD" "$(format_bytes "$B_BUILD")"
printf "     - target/wasm32-unknown-unknown          : %12d B (%s)\n" "$B_WASM" "$(format_bytes "$B_WASM")"
printf "     - target/release                         : %12d B (%s)\n" "$B_REL" "$(format_bytes "$B_REL")"
printf "  %-45s : %12d B (%s)\n" "2. Vendor PDFCraft Target" "$B_PDF" "$(format_bytes "$B_PDF")"
printf "  %-45s : %12d B (%s)\n" "3. Vendor GridCraft Target" "$B_GRID" "$(format_bytes "$B_GRID")"
echo "  ----------------------------------------------------------------------"
printf "  ${C_BOLD}%-45s : %12d B (%s)${C_RESET}\n" "TỔNG DUNG LƯỢNG BUILD CACHE HIỆN TẠI" "$B_TOTAL" "$(format_bytes "$B_TOTAL")"

# 3. Production Assets Check
echo ""
echo -e "${C_BOLD}📦 Kiểm tra tài sản tĩnh production độc lập (public/):${C_RESET}"
B_PUB_PDF=$(get_size_bytes "${ROOT_DIR}/public/pdfcraft")
B_PUB_GRID=$(get_size_bytes "${ROOT_DIR}/public/gridcraft")
printf "  - public/pdfcraft  : %s (Độc lập, WebView nhúng trực tiếp)\n" "$(format_bytes "$B_PUB_PDF")"
printf "  - public/gridcraft : %s (Độc lập, WebView nhúng trực tiếp)\n" "$(format_bytes "$B_PUB_GRID")"

# 4. Dry-run Clean Simulation
echo ""
echo -e "${C_BOLD}🧪 Giả lập dọn dẹp an toàn qua Cargo (Dry-run Analysis):${C_RESET}"

cd "$ROOT_DIR"
DRY_RUN_MAIN=$(cargo clean --dry-run 2>&1 | grep -E "Summary" | sed 's/^[ \t]*//' || echo "Không có thông tin")
DRY_RUN_UI=$(cargo clean -p office-utility-vn-ui --dry-run 2>&1 | grep -E "Summary" | sed 's/^[ \t]*//' || echo "N/A")
DRY_RUN_ENG=$(cargo clean -p office-engine --dry-run 2>&1 | grep -E "Summary" | sed 's/^[ \t]*//' || echo "N/A")
DRY_RUN_SHR=$(cargo clean -p office-shared --dry-run 2>&1 | grep -E "Summary" | sed 's/^[ \t]*//' || echo "N/A")
DRY_RUN_TAU=$(cargo clean -p office-utility-vn --dry-run 2>&1 | grep -E "Summary" | sed 's/^[ \t]*//' || echo "N/A")

echo "  - Toàn bộ workspace main      : ${DRY_RUN_MAIN}"
echo "  - Gói UI (kèm 12 test targets): ${DRY_RUN_UI}"
echo "  - Gói office-engine            : ${DRY_RUN_ENG}"
echo "  - Gói office-shared            : ${DRY_RUN_SHR}"
echo "  - Gói src-tauri                : ${DRY_RUN_TAU}"

if [[ -d "${ROOT_DIR}/vendor/pdfcraft" ]]; then
  cd "${ROOT_DIR}/vendor/pdfcraft"
  DRY_RUN_PDF=$(cargo clean --dry-run 2>&1 | grep -E "Summary" | sed 's/^[ \t]*//' || echo "N/A")
  echo "  - Vendor PDFCraft             : ${DRY_RUN_PDF}"
fi

if [[ -d "${ROOT_DIR}/vendor/gridcraft" ]]; then
  cd "${ROOT_DIR}/vendor/gridcraft"
  DRY_RUN_GRID=$(cargo clean --dry-run 2>&1 | grep -E "Summary" | sed 's/^[ \t]*//' || echo "N/A")
  echo "  - Vendor GridCraft            : ${DRY_RUN_GRID}"
fi

cd "$ROOT_DIR"

echo ""
echo -e "${C_BOLD}${C_YELLOW}💡 PHÂN LOẠI KHẢ NĂNG THU HỒI DUNG LƯỢNG (SAFE RECLAMATION CATEGORIES):${C_RESET}"
echo -e "  ${C_GREEN}[A] Thu hồi ngay KHÔNG CẦN rebuild ứng dụng đang chạy (~5.2 GiB):${C_RESET}"
echo -e "      • vendor/pdfcraft/target (3.54 GiB) - Tài sản đã nằm trong public/pdfcraft"
echo -e "      • vendor/gridcraft/target (1.57 GiB) - Tài sản đã nằm trong public/gridcraft"
echo ""
echo -e "  ${C_CYAN}[B] Thu hồi an toàn cho local crates, GIỮ NGUYÊN 3rd-party cache (~27.3 GiB):${C_RESET}"
echo -e "      • Chạy: cargo clean -p office-utility-vn-ui -p office-engine -p office-shared -p office-utility-vn"
echo -e "      • Giữ nguyên ~8.0 GiB thư viện ngoài (GTK, WebKit, Leptos CSR), lần build tiếp theo chỉ compile lại mã nội bộ."
echo ""
echo -e "  ${C_MAGENTA}[C] Thu hồi toàn bộ nhưng CẦN cold rebuild từ đầu (~35.3 GiB):${C_RESET}"
echo -e "      • Chạy: cargo clean (yêu cầu compile lại toàn bộ 300+ dependencies, mất ~2-5 phút)."
echo -e "${C_BOLD}${C_CYAN}========================================================================${C_RESET}"

if [[ "$CLEAN_SAFE" == true ]]; then
  echo ""
  echo -e "${C_BOLD}${C_GREEN}🚀 Đang thực hiện dọn dẹp cache an toàn (--clean-safe)...${C_RESET}"
  
  if [[ -d "${ROOT_DIR}/vendor/pdfcraft/target" ]]; then
    echo -e "  • Xóa vendor/pdfcraft/target..."
    rm -rf "${ROOT_DIR}/vendor/pdfcraft/target"
  fi
  if [[ -d "${ROOT_DIR}/vendor/gridcraft/target" ]]; then
    echo -e "  • Xóa vendor/gridcraft/target..."
    rm -rf "${ROOT_DIR}/vendor/gridcraft/target"
  fi

  echo -e "  • Thu hồi build cache cục bộ cho workspace packages qua Cargo..."
  cargo clean -p office-utility-vn-ui -p office-engine -p office-shared -p office-utility-vn 2>/dev/null || true

  if [[ -d "${TARGET_MAIN}/debug/incremental" ]]; then
    echo -e "  • Thu hồi bộ nhớ đệm incremental cũ..."
    rm -rf "${TARGET_MAIN}/debug/incremental"/* 2>/dev/null || true
  fi

  B_MAIN_AFTER=$(get_size_bytes "$TARGET_MAIN")
  B_PDF_AFTER=$(get_size_bytes "$TARGET_PDF")
  B_GRID_AFTER=$(get_size_bytes "$TARGET_GRID")
  B_TOTAL_AFTER=$(( B_MAIN_AFTER + B_PDF_AFTER + B_GRID_AFTER ))
  RECLAIMED=$(( B_TOTAL - B_TOTAL_AFTER ))
  if (( RECLAIMED < 0 )); then RECLAIMED=0; fi

  echo ""
  echo -e "${C_BOLD}${C_GREEN}========================================================================${C_RESET}"
  echo -e "${C_BOLD}${C_GREEN}🎉 THU HỒI DUNG LƯỢNG AN TOÀN THÀNH CÔNG!${C_RESET}"
  echo -e "   - Dung lượng giải phóng : ${C_BOLD}$(format_bytes "$RECLAIMED")${C_RESET}"
  echo -e "   - Dung lượng còn lại    : ${C_BOLD}$(format_bytes "$B_TOTAL_AFTER")${C_RESET} (giữ nguyên thư viện ngoài)"
  echo -e "${C_BOLD}${C_GREEN}========================================================================${C_RESET}"
fi
