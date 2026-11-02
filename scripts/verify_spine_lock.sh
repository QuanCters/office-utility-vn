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

EXPECTED_SHARED="30e34a51e3acc5b244b1f260d37f4052812189309b6b7d3194afaaaf9a5aa8cb"
EXPECTED_DB="28c86a6e4f2d9e1366188eb984640247e840844cc72bcd348272f7f6f71874ec"
EXPECTED_CMD="e42f6e5ad371ef9cdd4c05c0fd27f7ad6fcc3b632786916694a935fdd5d92416"

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
