# Streamline Build Targets & Disk Cache Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Streamline build targets across the project by consolidating fragmented test compilation units, scoping Tauri packaging bundle targets, and establishing automated safe cache reclamation to shrink the ~33 GiB target footprint.

**Architecture:** 
1. Consolidate 15 individual test suites into a single integration test runner (`tests/integration_tests.rs`) while preserving the dedicated E2E runner (`tests/e2e/main.rs`), eliminating redundant linking of 12 monolithic binaries and reclaiming ~14 GiB of duplicated build artifacts.
2. Scope Tauri bundle targets in `src-tauri/tauri.conf.json` from generic `"all"` to explicit production targets (`["nsis", "msi", "dmg", "deb", "appimage"]`) and eliminate redundant resource bundling.
3. Enhance `scripts/inspect_build_cache.sh` with automated safe reclamation (`--clean-safe`) and wire up `make clean-cache` in `Makefile` to quickly recover disk space without blowing away 3rd-party dependency caches.

**Tech Stack:** Rust 2024 edition, Cargo Workspaces, Tauri v2, Leptos CSR, Make, Bash.

**Spec:** User request: "chủ động rà soát và tinh gọn đồng bộ cả hai phần (giải phóng dung lượng cache + gộp test targets + tối ưu bundle targets)".

## Global Constraints

- Must preserve all 162 existing unit/integration tests and 517 E2E tests with 100% pass rate.
- Must not break `cargo test --workspace`, `cargo test --test e2e`, or `scripts/run_e2e_tests.sh`.
- Must maintain `verify_spine_lock.sh` architectural constraints.
- Must preserve 3rd-party compiled crates (GTK, WebKit, Leptos) during regular safe cache cleaning to avoid slow cold recompilation.
- Bundle configuration must remain fully compatible with GitHub Actions CI matrix (`windows-latest`, `macos-latest`).

## Review Focus

- **Test discovery and module isolation**: Ensure consolidation into `tests/integration_tests.rs` does not cause symbol collision or hide tests from `cargo test`.
- **Tauri bundle target syntax**: Ensure `targets: ["nsis", "msi", "dmg", "deb", "appimage"]` is valid for Tauri v2 and does not cause schema errors or packaging failures.
- **Resource duplication**: Verify that removing `resources: ["../public/pdfcraft"]` does not break PDF Studio or GridCraft (as both are already served via `Trunk` inside `frontendDist`).
- **Safe cache cleaning vs cold rebuild**: Verify `--clean-safe` clears internal package targets and vendor target directories without touching external dependencies.
- **Makefile workflow consistency**: Ensure `make test`, `make test-fast`, and `make clean-cache` operate seamlessly.

---

### Task 1: Consolidate Cargo Test Targets

**Files:**
- Create: `tests/integration_tests.rs`
- Modify: `Cargo.toml:36-84`
- Test: `tests/integration_tests.rs`, `tests/e2e/main.rs`

**Interfaces:**
- Consumes: All 15 test files in `tests/` (`app_lifecycle_test.rs`, `app_routing_test.rs`, `audio_soundpack_test.rs`, `data_lab_test.rs`, `gridcraft_workspace_test.rs`, `launcher_test.rs`, `media_studio_test.rs`, `micro_app_coordinator_test.rs`, `micro_app_platform_test.rs`, `micro_app_registry_test.rs`, `micro_app_routing_test.rs`, `pdf_studio_test.rs`, `shared_ui_test.rs`, `task_executor_contract_test.rs`, `vn_office_test.rs`).
- Produces: Single unified integration test binary `integration_tests` that executes all 162 unit/integration tests in a single process.

- [ ] **Step 1: Verify `tests/integration_tests.rs` harnesses all test modules**
Ensure `tests/integration_tests.rs` maps all 15 test modules using `#[path = "..."] mod ...;`.

- [ ] **Step 2: Streamline `Cargo.toml` `[[test]]` definitions**
Replace lines 40-83 in `Cargo.toml` (the 12 separate `[[test]]` blocks) with:
```toml
[[test]]
name = "e2e"
path = "tests/e2e/main.rs"

[[test]]
name = "integration_tests"
path = "tests/integration_tests.rs"
```

- [ ] **Step 3: Run unified integration tests**
Run: `cargo test -p office-utility-vn-ui --test integration_tests`
Expected: PASS with 162 tests passing, 0 failed.

- [ ] **Step 4: Run E2E test suite to verify no regressions**
Run: `cargo test -p office-utility-vn-ui --test e2e`
Expected: PASS with 517 tests passing, 0 failed.

---

### Task 2: Optimize Tauri Bundle Targets & Clean Redundancies

**Files:**
- Modify: `src-tauri/tauri.conf.json:33-55`

**Interfaces:**
- Consumes: Tauri v2 configuration schema (`https://schema.tauri.app/config/2`).
- Produces: Scoped bundle configuration targeting `["nsis", "msi", "dmg", "deb", "appimage"]` without redundant resources.

- [ ] **Step 1: Update bundle targets in `src-tauri/tauri.conf.json`**
Change `"targets": "all"` to explicit array:
```json
"targets": [
  "nsis",
  "msi",
  "dmg",
  "deb",
  "appimage"
]
```
Remove `"resources": ["../public/pdfcraft"]` because Trunk already compiles all `public/` assets (including `pdfcraft`, `gridcraft`, and `audio`) directly into `frontendDist` (`../dist`).

- [ ] **Step 2: Validate `tauri.conf.json` syntax and schema**
Run: `cargo check -p office-utility-vn`
Expected: PASS with zero configuration warnings or schema errors.

---

### Task 3: Enhance Cache Audit & Safe Reclamation Tooling

**Files:**
- Modify: `scripts/inspect_build_cache.sh`
- Modify: `Makefile:17-21, 137-148`

**Interfaces:**
- Consumes: `cargo clean -p <crate>`, `du -sb`.
- Produces: Automated `--clean-safe` CLI flag and `make clean-cache` target.

- [ ] **Step 1: Add `--clean-safe` implementation to `scripts/inspect_build_cache.sh`**
Implement `--clean-safe` flag in `scripts/inspect_build_cache.sh`:
1. Cleans vendor targets: `rm -rf vendor/pdfcraft/target vendor/gridcraft/target` (~5.8 GiB reclaimed).
2. Cleans internal workspace packages: `cargo clean -p office-utility-vn-ui -p office-engine -p office-shared -p office-utility-vn` (~20+ GiB reclaimed).
3. Cleans stale test and incremental artifacts from old hashes.
4. Preserves 3rd-party library cache so developers don't suffer cold recompiles.

- [ ] **Step 2: Add `clean-cache` and `test-integration` targets to `Makefile`**
Update `Makefile` to include:
```makefile
clean-cache: ## Dọn dẹp an toàn target nội bộ và vendor cache, giữ nguyên thư viện ngoài (~27 GiB)
	@./scripts/inspect_build_cache.sh --clean-safe

test-integration: ## Chạy bộ kiểm thử tích hợp hợp nhất (162 tests)
	@echo -e "$(COLOR_MAGENTA)🧪 Chạy bộ test tích hợp hợp nhất...$(COLOR_RESET)"
	cargo test -p office-utility-vn-ui --test integration_tests
```

- [ ] **Step 3: Execute safe cache cleanup and measure reclaimed disk space**
Run: `./scripts/inspect_build_cache.sh --clean-safe`
Expected: Reclaims > 25 GiB of disk space while keeping external dependency cache intact.

---

### Task 4: Full Workspace Verification & Quality Gates

**Files:**
- Verification only

- [ ] **Step 1: Verify Spine Lock integrity**
Run: `./scripts/verify_spine_lock.sh`
Expected: PASS with all architecture locks intact.

- [ ] **Step 2: Verify workspace compilation**
Run: `cargo check --workspace`
Expected: PASS with 0 errors.

- [ ] **Step 3: Run full workspace test suite**
Run: `cargo test --workspace`
Expected: PASS with all unit, integration, and E2E tests passing.

- [ ] **Step 4: Audit final cache state**
Run: `./scripts/inspect_build_cache.sh`
Expected: Disk footprint substantially reduced and report reflecting lean target status.
