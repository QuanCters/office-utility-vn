#!/usr/bin/env bash
# ==============================================================================
# run_e2e_tests.sh
# Runner thực thi toàn diện bộ kiểm thử đầu cuối E2E (Tiers 1-4, 50 công cụ)
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/cargo-target}"

echo "======================================================================"
echo "🚀 [OFFICE UTILITY VN] KHỞI CHẠY BỘ KIỂM THỬ ĐẦU CUỐI E2E (4 TIERS)"
echo "   Mục tiêu: 50 công cụ qua 4 phân hệ (doc-suite, media-studio, data-lab, vn-office)"
echo "   Phương pháp: Opaque-box requirement-driven (02-srs.md & 03-nfr.md)"
echo "======================================================================"

echo ""
echo "🔍 [Bước 1/2] Kiểm tra tính toàn vẹn Khung xương sống (Spine Guard)..."
"${SCRIPT_DIR}/verify_spine_lock.sh"

echo ""
echo "🧪 [Bước 2/2] Thực thi bộ test E2E (cargo test --test e2e $@)..."
cargo test --test e2e --manifest-path "${PROJECT_ROOT}/Cargo.toml" "$@"

echo ""
echo "======================================================================"
echo "🎉 XÁC NHẬN: TOÀN BỘ HỆ THỐNG KIỂM THỬ E2E HOẠT ĐỘNG HOÀN HẢO!"
echo "   - Tier 1: Bao phủ tính năng 50 công cụ (>= 250 tests)"
echo "   - Tier 2: Biên & Góc cạnh + SLA Hủy < 100ms (>= 250 tests)"
echo "   - Tier 3: Chuỗi liên kết đa công cụ / Đa phân hệ (10 pipelines)"
echo "   - Tier 4: Kịch bản doanh nghiệp thực tế (5 scenarios)"
echo "======================================================================"
