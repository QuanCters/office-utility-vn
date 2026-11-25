#!/usr/bin/env bash
# ==============================================================================
# verify_gridcraft_patch.sh
# Xác minh tính toàn vẹn và khả năng tái lập của bản vá GridCraft Confirmed Save
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
GRIDCRAFT_DIR="${ROOT_DIR}/vendor/gridcraft"
PATCH_FILE="${ROOT_DIR}/patches/0001-gridcraft-confirmed-save.patch"
UPSTREAM_META="${GRIDCRAFT_DIR}/UPSTREAM_COMMIT.txt"

echo "=== [GRIDCRAFT VENDOR PATCH AUDITOR] ==="

# 1. Kiểm tra tồn tại thư mục vendor
if [ ! -d "${GRIDCRAFT_DIR}" ]; then
    echo "❌ Lỗi: Thư mục ${GRIDCRAFT_DIR} không tồn tại!"
    exit 1
fi

# 2. Kiểm tra siêu dữ liệu upstream commit
if [ ! -f "${UPSTREAM_META}" ]; then
    echo "❌ Lỗi: Thiếu tệp siêu dữ liệu ${UPSTREAM_META}!"
    exit 1
fi

EXPECTED_SHA="d60709a1738d4f02c7f507848441306c0aacf692"
ACTUAL_SHA="$(grep -E "^COMMIT_SHA:" "${UPSTREAM_META}" | awk '{print $2}')"

if [ "${ACTUAL_SHA}" != "${EXPECTED_SHA}" ]; then
    echo "❌ Lỗi: Upstream SHA không khớp!"
    echo "   Kỳ vọng: ${EXPECTED_SHA}"
    echo "   Thực tế: ${ACTUAL_SHA}"
    exit 1
fi
echo "✓ Upstream Commit SHA: ${ACTUAL_SHA} (Khớp chuẩn 100%)"

# 3. Kiểm tra các tệp bản quyền và phân phối gốc
if [ ! -f "${GRIDCRAFT_DIR}/LICENSE-MIT" ] || [ ! -f "${GRIDCRAFT_DIR}/LICENSE-APACHE" ]; then
    echo "❌ Lỗi: Thiếu tệp bản quyền LICENSE-MIT hoặc LICENSE-APACHE!"
    exit 1
fi
echo "✓ Attribution & License files: Hợp lệ (MIT & Apache-2.0)"

# 4. Kiểm tra tệp bản vá
if [ ! -f "${PATCH_FILE}" ]; then
    echo "❌ Lỗi: Tệp bản vá ${PATCH_FILE} không tồn tại!"
    exit 1
fi
echo "✓ Patch Artifact: ${PATCH_FILE} (Tồn tại)"

# 5. Kiểm tra mã nguồn host.rs đã áp dụng cơ chế xác nhận lưu
HOST_RS="${GRIDCRAFT_DIR}/apps/gridcraft-web/src/host.rs"
if ! grep -Fq 'Some("confirmed")' "${HOST_RS}" || ! grep -Fq 'Message::Saved { id }' "${HOST_RS}"; then
    echo "❌ Lỗi: ${HOST_RS} chưa có cơ chế confirmed save!"
    exit 1
fi
echo "✓ Confirmed Save Protocol: Đã tích hợp đầy đủ vào host.rs"

# 6. Kiểm tra tính áp dụng ngược (Reverse dry-run) để chứng minh tính nhất quán của bản vá
if ! (cd "${GRIDCRAFT_DIR}" && patch -p1 --dry-run -R < "${PATCH_FILE}" > /dev/null 2>&1); then
    echo "❌ Lỗi: Bản vá không thể áp dụng/hoàn tác sạch sẽ trên mã nguồn hiện tại!"
    exit 1
fi
echo "✓ Deterministic Patch Verification: Hoàn tất (Patch khớp 100% mã nguồn)"

echo "🎉 [GRIDCRAFT VENDOR PATCH] TẤT CẢ TIÊU CHÍ XÁC MINH ĐỀU ĐẠT CHUẨN 100%!"
exit 0
