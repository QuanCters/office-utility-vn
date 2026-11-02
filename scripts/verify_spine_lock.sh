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
EXPECTED_CMD="008c025aaccdb43ccd8422956048e9d2bb52e55041e3ae546ed3b9e9793abad7"

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
CARGO_TARGET_DIR=/tmp/cargo-target cargo test --workspace --manifest-path "${PROJECT_ROOT}/Cargo.toml"

echo "🎉 [SPINE GUARD] XÁC NHẬN TOÀN BỘ KHUNG XƯƠNG SỐNG AN TOÀN 100%!"
exit 0
