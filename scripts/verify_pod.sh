#!/usr/bin/env bash
set -e

POD_NAME=$1

if [ -z "$POD_NAME" ]; then
    echo "❌ Vui lòng truyền tên công cụ (tool pod). Ví dụ: ./scripts/verify_pod.sh vietnamese-number-to-words"
    exit 1
fi

echo "🔍 [1/3] Kiểm tra biên dịch cargo check -p office-engine..."
cargo check -p office-engine

echo "🔍 [2/3] Kiểm tra bảo mật Zero Network & No Unwrap/Expect trong engine..."
# Kiểm tra NFR-SEC-01: Cấm std::net, tokio::net, reqwest, hyper
if grep -rn --include="*.rs" -E "(std::net|tokio::net|reqwest|hyper)" crates/office-engine/src/ 2>/dev/null; then
    echo "❌ VI PHẠM BẢO MẬT (NFR-SEC-01): Phát hiện network crate hoặc std::net trong office-engine!"
    exit 1
fi

# Kiểm tra NFR-SEC-03: Cấm .unwrap() và .expect() trong mã logic (ngoại trừ tests)
FORBIDDEN_UNWRAP=$(find crates/office-engine/src -type f -name "*.rs" -exec awk '
    /#\[cfg\(test\)\]/ { in_test = 1 }
    !in_test && /(\.unwrap\(|\.expect\()/ { print FILENAME ":" FNR ":" $0 }
' {} + 2>/dev/null || true)

if [ -n "$FORBIDDEN_UNWRAP" ]; then
    echo "❌ VI PHẠM AN TOÀN BỘ NHỚ (NFR-SEC-03): Phát hiện .unwrap() hoặc .expect() trong mã nghiệp vụ:"
    echo "$FORBIDDEN_UNWRAP"
    exit 1
fi

TEST_FILTER=$(echo "$POD_NAME" | tr '-' '_')
echo "🔍 [3/3] Chạy bộ kiểm thử cargo test -p office-engine $TEST_FILTER..."
cargo test -p office-engine "$TEST_FILTER"

echo "✅ Kiểm định Tool Pod '$POD_NAME' hoàn tất!"
