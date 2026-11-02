#!/usr/bin/env bash
set -e

echo "🔍 [1/5] Kiểm tra vị trí đặt repo harness2 và office-utility-vn..."
if [ -d "./harness2" ]; then
    echo "❌ LỖI: Thư mục 'harness2' đang nằm BÊN TRONG 'office-utility-vn'. Hãy di chuyển nó ra thư mục cha (../harness2)!"
    exit 1
fi
echo "✅ Cấu trúc thư mục tách biệt hợp lệ."

echo "🔍 [2/5] Kiểm tra tệp AGENTS.md và liên kết nội tuyến @[experience]..."
if [ ! -f "AGENTS.md" ]; then
    echo "❌ LỖI: Thiếu tệp AGENTS.md ở thư mục gốc!"
    exit 1
fi
if grep -q "^---" AGENTS.md; then
    echo "❌ LỖI: AGENTS.md KHÔNG ĐƯỢC có YAML frontmatter ở dòng đầu!"
    exit 1
fi
if ! grep -q "@\[experience\](.harness/memory/EXPERIENCE.md)" AGENTS.md; then
    echo "❌ LỖI: AGENTS.md chưa nhúng @[experience](.harness/memory/EXPERIENCE.md)!"
    exit 1
fi
echo "✅ AGENTS.md hợp lệ."

echo "🔍 [3/5] Kiểm tra YAML frontmatter của .agents/rules/ (Flat directory & snake_case)..."
for rule_file in .agents/rules/*.md; do
    if ! head -n 1 "$rule_file" | grep -q "^---"; then
        echo "❌ LỖI: $rule_file thiếu YAML frontmatter (bắt đầu bằng ---)!"
        exit 1
    fi
    if ! grep -q "trigger: always_on" "$rule_file"; then
        echo "❌ LỖI: $rule_file thiếu 'trigger: always_on' (lưu ý viết snake_case, không viết alwaysOn)!"
        exit 1
    fi
done
echo "✅ Toàn bộ Rules trong .agents/rules/ hợp lệ."

echo "🔍 [4/5] Kiểm tra cấu trúc .agents/skills/*/SKILL.md..."
for skill_name in "office-tool-pod" "leptos-3zone-ui"; do
    skill_path=".agents/skills/${skill_name}/SKILL.md"
    if [ ! -f "$skill_path" ]; then
        echo "❌ LỖI: Không tìm thấy $skill_path!"
        exit 1
    fi
    if ! grep -q "^name:" "$skill_path" || ! grep -q "^description:" "$skill_path"; then
        echo "❌ LỖI: $skill_path thiếu trường 'name:' hoặc 'description:' trong YAML frontmatter!"
        exit 1
    fi
done
echo "✅ Toàn bộ Skills hợp lệ."

echo "🔍 [5/5] Kiểm tra Subagent .agents/agents/pod_implementer.md..."
if ! grep -q "^subagent: true" .agents/agents/pod_implementer.md; then
    echo "❌ LỖI: .agents/agents/pod_implementer.md thiếu khai báo 'subagent: true'!"
    exit 1
fi
echo "✅ Subagent hợp lệ."

echo "🎉 CHÚC MỪNG! Toàn bộ cấu hình Harness2 + Antigravity đã sẵn sàng 100%."