#!/usr/bin/env bash
# ==============================================================================
# verify_spine_lock.sh
# Kiểm tra tính toàn vẹn và khóa bảo vệ khung xương sống (Phase 0 Spine Layer)
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== [SPINE GUARD] KIỂM TRA KHÓA BẢO VỆ KHUNG XƯƠNG SỐNG ==="

SHARED_FILE="${PROJECT_ROOT}/crates/office-shared/src/lib.rs"
DB_FILE="${PROJECT_ROOT}/src-tauri/src/db/mod.rs"
CMD_FILE="${PROJECT_ROOT}/src-tauri/src/commands/mod.rs"

EXPECTED_SHARED="26a8aed81fceb200951f4581f80635b42f3344186d3cb0a7c84d1016629e32c6"
EXPECTED_DB="38bcd07c7358503b46664e407ed6d84e641661d82a95a428299d15f61df0c5d1"
# Stage 4B-2A Security Closure Authorized Amendment:
# Permanently fail-closed save_file_binary, removed register_authorized_document,
# added native_dialog_open_file and native_dialog_save_as.
# Stage 4C-0D Security Closure Authorized Amendment:
# Permanently fail-closed read_file_binary, added load_dev_fixture under #[cfg(debug_assertions)].
# Audit history:
# Stage 2D hash: 8f4524125899479ffebbc74b3b14be6a96fa6dc63bbce311a4278f46b920e1da
# Stage 4B-2A initial hash: aaf5784b547e3d93836f22cfa1acf2024a0c3735d82e56df7620783282a09237
# Stage 4B-2A revised hash: 191d8a12b60ec83217ac275c59a961a262bbaf014236444f44281f8182dd63a9
# PR-01 Safety Gate Hardening Authorized Amendment (RFC-SPINE-2026-001-PR01):
# Replaced wildcard fake SUCCESS with fail-closed deny-by-default (ENABLED_TOOL_ALLOWLIST),
# routed fake engines & unsafe lossy PDF tools to fail-closed, added emergency kill-switch,
# and enforced single-tool SQLite audit persistence (dispatch_tool_with_persistence).
EXPECTED_CMD="a40efb22329bc176fda7e47b0f74c31234f1af35061a65eb339506b48c2955e3"

ACTUAL_SHARED="$(sha256sum "${SHARED_FILE}" | awk '{print $1}')"
ACTUAL_DB="$(sha256sum "${DB_FILE}" | awk '{print $1}')"
ACTUAL_CMD="$(sha256sum "${CMD_FILE}" | awk '{print $1}')"

if [ "${ACTUAL_SHARED}" != "${EXPECTED_SHARED}" ]; then
    echo "❌ LỖI BẢO MẬT: Phát hiện crates/office-shared đã bị sửa đổi trái phép!"
    echo "   Kỳ vọng: ${EXPECTED_SHARED}"
    echo "   Thực tế: ${ACTUAL_SHARED}"
    exit 1
fi
echo "✓ crates/office-shared: Hợp lệ (SHA-256 khớp chuẩn)"

if [ "${ACTUAL_DB}" != "${EXPECTED_DB}" ]; then
    echo "❌ LỖI BẢO MẬT: Phát hiện src-tauri/src/db đã bị sửa đổi trái phép!"
    echo "   Kỳ vọng: ${EXPECTED_DB}"
    echo "   Thực tế: ${ACTUAL_DB}"
    exit 1
fi
echo "✓ src-tauri/src/db: Hợp lệ (SHA-256 khớp chuẩn)"

if [ "${ACTUAL_CMD}" != "${EXPECTED_CMD}" ]; then
    echo "❌ LỖI BẢO MẬT: Phát hiện src-tauri/src/commands đã bị sửa đổi trái phép!"
    echo "   Kỳ vọng: ${EXPECTED_CMD}"
    echo "   Thực tế: ${ACTUAL_CMD}"
    exit 1
fi
echo "✓ src-tauri/src/commands: Hợp lệ (SHA-256 khớp chuẩn)"

echo "=== [SPINE GUARD] KIỂM TRA UNIT TEST TOÀN WORKSPACE ==="
cargo test --workspace --manifest-path "${PROJECT_ROOT}/Cargo.toml"

echo "🎉 [SPINE GUARD] XÁC NHẬN TOÀN BỘ KHUNG XƯƠNG SỐNG AN TOÀN 100%!"
exit 0
