#!/usr/bin/env bash
# ==============================================================================
# Cài đặt Git Hooks - Office Utility VN
# ==============================================================================
set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
HOOKS_DIR="$ROOT_DIR/.git/hooks"

if [ ! -d "$HOOKS_DIR" ]; then
    echo "❌ Không tìm thấy thư mục .git/hooks! Vui lòng chạy script trong git repository."
    exit 1
fi

cat << 'EOF' > "$HOOKS_DIR/pre-commit"
#!/usr/bin/env bash
# Delegate to version-controlled scripts/pre_commit.sh
ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || dirname "$(dirname "$(dirname "$0")")")"
exec "$ROOT_DIR/scripts/pre_commit.sh" "$@"
EOF

chmod +x "$HOOKS_DIR/pre-commit"
chmod +x "$ROOT_DIR/scripts/pre_commit.sh"

echo "✅ [GIT HOOKS] Đã cài đặt thành công Git Pre-Commit Hook!"
echo "   Mỗi khi 'git commit', hệ thống sẽ tự động chạy:"
echo "   1. make verify-ui (UI Design Tokens & 12px Ergonomic Floor)"
echo "   2. ./scripts/verify_spine_lock.sh (Khóa Spine Lock & Unit Tests)"
