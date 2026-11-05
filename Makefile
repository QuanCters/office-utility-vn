# ==============================================================================
# Makefile - Office Utility VN (Tauri v2 + Leptos 0.8 CSR)
# ==============================================================================

SHELL := /usr/bin/env bash
.DEFAULT_GOAL := help

# ANSI color codes for terminal output
COLOR_RESET   := \033[0m
COLOR_BOLD    := \033[1m
COLOR_CYAN    := \033[36m
COLOR_GREEN   := \033[32m
COLOR_YELLOW  := \033[33m
COLOR_BLUE    := \033[34m
COLOR_MAGENTA := \033[35m

.PHONY: help dev run dev-web build build-web build-debug check check-fast \
        test test-fast test-engine test-e2e verify-spine verify-ui verify-pod \
        clippy fmt fmt-check bench setup setup-hooks clean kill-dev

##@ 🚀 Chạy ứng dụng (Development)

help: ## Hiển thị bảng trợ giúp này
	@echo ""
	@echo -e "$(COLOR_BOLD)$(COLOR_CYAN)Office Utility VN - Development & Build Workflow$(COLOR_RESET)"
	@echo -e "$(COLOR_YELLOW)Cách dùng:$(COLOR_RESET) make $(COLOR_GREEN)<target>$(COLOR_RESET) [TOOL=<tên_công_cụ>]"
	@echo ""
	@awk 'BEGIN {FS = ":.*##"; printf ""} \
		/^[a-zA-Z0-9_-]+:.*?##/ { printf "  $(COLOR_GREEN)%-18s$(COLOR_RESET) %s\n", $$1, $$2 } \
		/^##@/ { printf "\n$(COLOR_BOLD)%s$(COLOR_RESET)\n", substr($$0, 5) } ' $(MAKEFILE_LIST)
	@echo ""

dev: ## Khởi chạy ứng dụng Desktop Tauri v2 (kèm Leptos Web qua Trunk port 1420)
	@echo -e "$(COLOR_CYAN)🚀 Kiểm tra cổng 1420 & khởi chạy ứng dụng Tauri v2 Desktop...$(COLOR_RESET)"
	@fuser -k 1420/tcp >/dev/null 2>&1 || true
	cargo tauri dev

run: dev ## Alias của target 'dev'

dev-web: ## Chỉ chạy giao diện Leptos trên trình duyệt (Trunk serve cổng 1420)
	@echo -e "$(COLOR_CYAN)🌐 Kiểm tra cổng 1420 & khởi chạy frontend Leptos qua Trunk...$(COLOR_RESET)"
	@fuser -k 1420/tcp >/dev/null 2>&1 || true
	trunk serve --port 1420 --open

kill-dev: ## Giải phóng cổng 1420 nếu bị tiến trình trunk/tauri cũ chiếm dụng
	@echo -e "$(COLOR_YELLOW)🔌 Kiểm tra và giải phóng cổng 1420...$(COLOR_RESET)"
	@fuser -k 1420/tcp >/dev/null 2>&1 && echo -e "$(COLOR_GREEN)✅ Đã giải phóng cổng 1420!$(COLOR_RESET)" || echo -e "$(COLOR_GREEN)✅ Cổng 1420 đang trống.$(COLOR_RESET)"

##@ 📦 Đóng gói & Build (Packaging)

build: ## Đóng gói ứng dụng desktop release (EXE, MSI, DMG hoặc DEB/AppImage)
	@echo -e "$(COLOR_GREEN)📦 Đang biên dịch release bundle cho Desktop qua Tauri...$(COLOR_RESET)"
	cargo tauri build

build-debug: ## Biên dịch bản build Desktop chế độ debug (nhanh hơn release)
	@echo -e "$(COLOR_GREEN)🛠️ Đang biên dịch bản desktop debug...$(COLOR_RESET)"
	cargo tauri build --debug

build-web: ## Đóng gói mã nguồn WebAssembly frontend ra thư mục dist/ (chế độ release)
	@echo -e "$(COLOR_GREEN)🌐 Đang đóng gói WASM frontend qua Trunk...$(COLOR_RESET)"
	trunk build --release

##@ 🧪 Kiểm thử (Testing)

test: ## Chạy toàn bộ test trong workspace (Unit tests + Doc tests)
	@echo -e "$(COLOR_MAGENTA)🧪 Chạy toàn bộ unit test trong workspace...$(COLOR_RESET)"
	cargo test --workspace

test-fast: ## Chạy unit test workspace
	@echo -e "$(COLOR_MAGENTA)⚡ Chạy test nhanh workspace...$(COLOR_RESET)"
	cargo test --workspace

test-engine: ## Chạy test cho office-engine (Dùng: make test-engine TOOL=<tool_name>)
	@echo -e "$(COLOR_MAGENTA)🧪 Chạy test cho crate office-engine $(if $(TOOL),cho $(TOOL),)...$(COLOR_RESET)"
	cargo test -p office-engine $(if $(TOOL),$(TOOL),)

test-e2e: ## Chạy toàn bộ bộ kiểm thử đầu cuối E2E (Tiers 1-4, 50 công cụ)
	@echo -e "$(COLOR_MAGENTA)🚀 Khởi chạy trọn bộ E2E 4 Tiers...$(COLOR_RESET)"
	./scripts/run_e2e_tests.sh

bench: ## Chạy benchmark hiệu năng (performance_benchmarks)
	@echo -e "$(COLOR_YELLOW)⏱️ Chạy benchmark hiệu năng...$(COLOR_RESET)"
	cargo bench --bench performance_benchmarks

##@ 🔍 Kiểm tra chất lượng & Kiến trúc (Verification & Lint)

check: ## Kiểm tra biên dịch cú pháp toàn workspace (cargo check)
	@echo -e "$(COLOR_CYAN)🔍 Kiểm tra biên dịch toàn workspace...$(COLOR_RESET)"
	cargo check --workspace

check-fast: ## Kiểm tra biên dịch siêu nhanh cho office-engine (<2s)
	@echo -e "$(COLOR_CYAN)⚡ Kiểm tra biên dịch nhanh office-engine...$(COLOR_RESET)"
	cargo check -p office-engine

verify-spine: ## Xác thực tính toàn vẹn khóa Spine Lock (shared, db, commands)
	@echo -e "$(COLOR_YELLOW)🔒 Kiểm tra khóa kiến trúc Spine Lock...$(COLOR_RESET)"
	./scripts/verify_spine_lock.sh

verify-ui: ## Xác thực chuẩn Design Tokens & Đáy công thái học 12px (verify_ui_tokens.sh)
	@echo -e "$(COLOR_YELLOW)🎨 Kiểm tra khóa chuẩn UI Design Tokens & Ergonomic Floor...$(COLOR_RESET)"
	./scripts/verify_ui_tokens.sh

verify-pod: ## Kiểm định Pod tool (bảo mật zero network, cấm unwrap) (Dùng: make verify-pod TOOL=<tên_pod>)
	@if [ -z "$(TOOL)" ]; then \
		echo -e "$(COLOR_YELLOW)⚠️ Vui lòng chỉ định TOOL, ví dụ: make verify-pod TOOL=vietnamese-number-to-words$(COLOR_RESET)"; \
		exit 1; \
	fi
	./scripts/verify_pod.sh $(TOOL)

clippy: ## Chạy clippy linter phát hiện code smells & lỗi tiềm ẩn
	@echo -e "$(COLOR_CYAN)🧹 Chạy cargo clippy...$(COLOR_RESET)"
	cargo clippy --workspace --all-targets -- -D warnings

fmt: ## Tự động định dạng mã nguồn Rust toàn bộ dự án
	@echo -e "$(COLOR_CYAN)✨ Định dạng mã nguồn (cargo fmt)...$(COLOR_RESET)"
	cargo fmt --all

fmt-check: ## Kiểm tra định dạng mã nguồn không thay đổi file
	@echo -e "$(COLOR_CYAN)🔍 Kiểm tra định dạng mã nguồn...$(COLOR_RESET)"
	cargo fmt --all -- --check

##@ 🛠️ Thiết lập & Dọn dẹp (Setup & Maintenance)

setup: ## Kiểm tra và cài đặt công cụ phát triển cần thiết (wasm32 target, trunk, tauri-cli, git hooks)
	@echo -e "$(COLOR_GREEN)🔧 Kiểm tra và chuẩn bị môi trường phát triển...$(COLOR_RESET)"
	rustup target add wasm32-unknown-unknown
	@which trunk >/dev/null 2>&1 || (echo "Cài đặt trunk..." && cargo install trunk)
	@cargo tauri --version >/dev/null 2>&1 || (echo "Cài đặt tauri-cli..." && cargo install tauri-cli --version "^2.0")
	./scripts/install_git_hooks.sh
	@echo -e "$(COLOR_GREEN)✅ Môi trường đã sẵn sàng!$(COLOR_RESET)"

setup-hooks: ## Cài đặt Git Pre-Commit Hook tự động (make verify-ui + verify_spine_lock.sh)
	@echo -e "$(COLOR_CYAN)🪝 Đang cấu hình Git Pre-Commit Hook...$(COLOR_RESET)"
	./scripts/install_git_hooks.sh

clean: ## Dọn dẹp các thư mục build (dist, target, /tmp/cargo-target) & tiến trình dev
	@echo -e "$(COLOR_YELLOW)🧹 Dọn dẹp thư mục build & tiến trình dev...$(COLOR_RESET)"
	@fuser -k 1420/tcp >/dev/null 2>&1 || true
	cargo clean
	rm -rf dist target /tmp/cargo-target
	@echo -e "$(COLOR_GREEN)✅ Dọn dẹp hoàn tất!$(COLOR_RESET)"
