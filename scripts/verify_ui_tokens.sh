#!/usr/bin/env bash
set -euo pipefail

echo "========================================================="
echo "🎨 [UI TOKEN GUARD] KIỂM TRA CHUẨN DESIGN TOKENS & CÔNG THÁI HỌC"
echo "========================================================="

# 1. Kiểm tra mã màu hex cứng ngoài tệp định nghĩa token (base.css)
echo "🔍 1. Kiểm tra mã màu hex cứng ngoài Design Tokens (styles/ & src/)..."
HEX_VIOLATIONS=0

if grep -rnE '#[0-9a-fA-F]{3,8}' styles/ src/ --include="*.css" --exclude="base.css" --exclude="tokens.css" 2>/dev/null; then
  echo "❌ Phát hiện mã màu hex viết trực tiếp ngoài token definitions!"
  HEX_VIOLATIONS=1
fi

if [ $HEX_VIOLATIONS -ne 0 ]; then
  echo "❌ [THẤT BẠI] Toàn bộ mã màu trong các component CSS phải sử dụng biến token: var(--...)"
  exit 1
fi
echo "✓ Không phát hiện mã màu hex cứng ngoài base.css"

# 2. Kiểm tra quy tắc đáy công thái học 12px (The 12px Ergonomic Floor Rule)
echo "🔍 2. Kiểm tra quy tắc đáy 12px Ergonomic Floor (CSS & Leptos templates)..."
FLOOR_VIOLATIONS=0

# Quét trong toàn bộ tệp CSS
if grep -rnE 'font-size:\s*(9px|10px|11px|0\.5[0-9]*rem|0\.6[0-9]*rem|0\.7rem)' styles/ src/ --include="*.css" 2>/dev/null; then
  echo "❌ Phát hiện khai báo font-size < 12px trong CSS!"
  FLOOR_VIOLATIONS=1
fi

# Quét trong toàn bộ inline styles của Rust / Leptos templates
if grep -rnE 'font-size:\s*(9px|10px|11px|0\.5[0-9]*rem|0\.6[0-9]*rem|0\.7rem)' src/ --include="*.rs" 2>/dev/null; then
  echo "❌ Phát hiện khai báo font-size < 12px trong Rust/Leptos templates!"
  FLOOR_VIOLATIONS=1
fi

if [ $FLOOR_VIOLATIONS -ne 0 ]; then
  echo "❌ [THẤT BẠI] Mọi nhãn chữ, badge và thông tin phụ phải đạt tối thiểu >= 12px để bảo vệ thị lực người dùng!"
  exit 1
fi
echo "✓ 100% font-size đạt ngưỡng an toàn >= 12px"

# 3. Kiểm tra bẫy sự kiện chuột thanh tiêu đề Tauri (No-Drag Guard)
echo "🔍 3. Kiểm tra bẫy kéo cửa sổ trên thanh tiêu đề Tauri (app-region: no-drag)..."
if ! grep -q -- "-webkit-app-region: no-drag" styles/base.css || ! grep -q -- "app-region: no-drag" styles/base.css; then
  echo "❌ Thiếu khai báo no-drag cho các phần tử con tương tác trên thanh tiêu đề!"
  exit 1
fi
echo "✓ Khai báo app-region: no-drag đầy đủ cho toàn bộ nút điều khiển và badge"

# 4. Kiểm tra quy tắc chống rớt dòng cho Badge, Pill & Shortcut Chip
echo "🔍 4. Kiểm tra quy tắc chống vỡ dòng (Anti-Wrap Safeguard)..."
if ! grep -q -- "white-space: nowrap" styles/base.css; then
  echo "❌ Thiếu quy tắc white-space: nowrap cho các badge/pill!"
  exit 1
fi
echo "✓ Quy tắc chống vỡ dòng (white-space: nowrap, flex-shrink: 0) đã được kích hoạt"

echo "========================================================="
echo "🎉 [UI TOKEN GUARD] TOÀN BỘ CHUẨN DESIGN TOKENS HỢP LỆ 100%!"
echo "========================================================="
