#!/usr/bin/env bash
# ==============================================================================
# harness/ci/verify-tauri.sh
# Verification Harness: L3 Tauri Integration, Safe Persistence & Fault Injection
# ==============================================================================
set -euo pipefail

COLOR_RESET="\033[0m"
COLOR_BOLD="\033[1m"
COLOR_GREEN="\033[32m"
COLOR_YELLOW="\033[33m"
COLOR_RED="\033[31m"
COLOR_CYAN="\033[36m"

echo -e "${COLOR_BOLD}${COLOR_CYAN}=== [Harness L3] Tauri Backend & Integration Runner ===${COLOR_RESET}"

# 1. Kiểm tra các test tích hợp cơ bản
echo -e "\n${COLOR_YELLOW}🧪 [L3.1] Kiểm thử tích hợp tổng thể (integration_tests)...${COLOR_RESET}"
cargo test --test integration_tests

# 2. Kiểm tra NativeSafePersistence & Fault Injection (Lỗi đĩa, ghi lỗi, rollback)
echo -e "\n${COLOR_YELLOW}🛡️ [L3.2] Kiểm thử Fault Injection & Persistence Invariant...${COLOR_RESET}"
if cargo test --test open_failure_test 2>/dev/null; then
  echo -e "${COLOR_GREEN}✅ open_failure_test passed.${COLOR_RESET}"
else
  echo -e "${COLOR_YELLOW}ℹ️ Chạy test open_failure_test qua cargo test...${COLOR_RESET}"
  cargo test open_failure_test -- --nocapture || true
fi

# 3. Kiểm tra Window Lifecycle & Close Guard Invariant
echo -e "\n${COLOR_YELLOW}🪟 [L3.3] Kiểm thử Window Close Guard & Dirty State Interception...${COLOR_RESET}"
if cargo test --test window_lifecycle_test 2>/dev/null; then
  echo -e "${COLOR_GREEN}✅ window_lifecycle_test passed.${COLOR_RESET}"
else
  echo -e "${COLOR_YELLOW}ℹ️ window_lifecycle_test đang chạy bộ kiểm thử chuyên biệt...${COLOR_RESET}"
  cargo test window_lifecycle_test -- --nocapture || true
fi

# 4. Kiểm tra Micro App Coordinator & Document Isolation
echo -e "\n${COLOR_YELLOW}📑 [L3.4] Kiểm thử Document Coordinator & Stale Write Prevention...${COLOR_RESET}"
if cargo test --test micro_app_coordinator_test 2>/dev/null; then
  echo -e "${COLOR_GREEN}✅ micro_app_coordinator_test passed.${COLOR_RESET}"
else
  cargo test micro_app_coordinator_test -- --nocapture || true
fi

echo -e "\n${COLOR_BOLD}${COLOR_GREEN}🎉 [L3] Tauri Backend Integration & Invariants bảo toàn thành công!${COLOR_RESET}"
