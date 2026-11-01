# GitHub Actions Windows & macOS CI/CD Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish an automated GitHub Actions CI/CD pipeline to compile, bundle, and release the Tauri v2 + Leptos application for Windows (`.msi`, `.exe`) and macOS (`.dmg`, `.app`), with full artifact uploading and release drafting.

**Architecture:** A GitHub Actions workflow running a multi-platform matrix (`windows-latest`, `macos-latest`). Each runner configures the Rust toolchain with the `wasm32-unknown-unknown` target, sets up Trunk via pre-built binaries, caches cargo dependencies, executes `tauri-apps/tauri-action` to build bundles, and archives the resulting packages via `actions/upload-artifact` and GitHub Releases.

**Tech Stack:** GitHub Actions, Tauri v2 CLI / `tauri-apps/tauri-action@v0`, Leptos CSR (WASM), Trunk, Rust 2024 edition.

**Spec:** User request: "thiết lập github action đi" (set up GitHub Actions for Windows/macOS).

## Global Constraints

- Workflow must support Windows (`windows-latest`) and macOS (`macos-latest`).
- Frontend must build with Trunk using the `wasm32-unknown-unknown` target.
- Workflow must trigger on:
  - Push to `main`/`master`
  - Version tags (`v*`)
  - Pull requests (build verification)
  - Manual trigger (`workflow_dispatch`)
- Artifacts must be accessible even when runs do not create a tagged release.

## Review Focus

- **Workflow dispatch / PR without git tag**: When triggered manually or by PR, `tauri-apps/tauri-action` must not crash if no tag is present; build artifacts must be uploaded via `actions/upload-artifact@v4`.
- **Trunk wasm target availability**: Rust toolchain setup must explicitly include `wasm32-unknown-unknown` so `trunk build` does not fail on Windows/macOS runners.
- **Cargo.lock consistency**: Ensure `Cargo.lock` is tracked or preserved so runner environments do not unexpectedly resolve conflicting crate versions.
- **Tauri v2 bundle configuration**: Ensure `src-tauri/tauri.conf.json` has bundle targets and metadata compatible with both Windows (NSIS, MSI) and macOS (DMG, APP).
- **GitHub token permissions**: Workflow must declare `permissions: contents: write` so release creation succeeds on tag pushes.

---

### Task 1: Create GitHub Actions Workflow File

**Files:**
- Create: `.github/workflows/release.yml`

**Interfaces:**
- Consumes: `src-tauri/tauri.conf.json`, `Trunk.toml`, `Cargo.toml`
- Produces: GitHub Actions workflow `.github/workflows/release.yml` capable of running on `windows-latest` and `macos-latest`.

- [ ] **Step 1: Write `.github/workflows/release.yml`**
Create `.github/workflows/release.yml` with:
- `name: Release & Build Matrix`
- Triggers:
  ```yaml
  on:
    push:
      branches: [main, master]
      tags:
        - 'v*'
    pull_request:
      branches: [main, master]
    workflow_dispatch:
  ```
- Permissions: `contents: write`
- Matrix strategy:
  ```yaml
  strategy:
    fail-fast: false
    matrix:
      include:
        - platform: 'windows-latest'
          args: ''
        - platform: 'macos-latest'
          args: '--target universal-apple-darwin'
  ```
- Steps:
  - Checkout repository (`actions/checkout@v4`)
  - Rust toolchain (`dtolnay/rust-toolchain@stable`) with targets `wasm32-unknown-unknown`, and for macOS also `aarch64-apple-darwin`, `x86_64-apple-darwin`
  - Cache Rust (`swatinem/rust-cache@v2`)
  - Install Trunk via `taiki-e/install-action@v2` with tool: `trunk`
  - Tauri action (`tauri-apps/tauri-action@v0`) with `tagName: app-v__VERSION__` (or triggered tag) and `includeRelease: ${{ startsWith(github.ref, 'refs/tags/v') }}`
  - Upload build artifacts using `actions/upload-artifact@v4`

- [ ] **Step 2: Verify YAML syntax and validity**
Run a local YAML validation / lint check or schema verification command.
Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"`
Expected: YAML parses cleanly with exit code 0.

---

### Task 2: Refine Project & Tauri Configuration for Multi-Platform Build

**Files:**
- Modify: `.gitignore`
- Modify: `src-tauri/tauri.conf.json`

**Interfaces:**
- Consumes: existing `.gitignore` and `src-tauri/tauri.conf.json`
- Produces: updated `.gitignore` (un-ignoring `Cargo.lock` for deterministic builds) and refined `src-tauri/tauri.conf.json` (specifying NSIS / DMG bundle settings).

- [ ] **Step 1: Update `.gitignore`**
Remove `/Cargo.lock` from `.gitignore` so dependencies stay pinned for deterministic CI runs across Windows and macOS, and ensure `dist` and `target` remain ignored.

- [ ] **Step 2: Update `src-tauri/tauri.conf.json`**
Add bundle-specific options under `"bundle"`:
- Windows NSIS installer configuration (`"nsis": { "installMode": "currentUser" }` or similar sensible defaults).
- macOS configuration (`"macOS": { "minimumSystemVersion": "10.13" }`).

- [ ] **Step 3: Verify JSON validity**
Run: `python3 -c "import json; json.load(open('src-tauri/tauri.conf.json'))"`
Expected: Valid JSON with exit code 0.

---

### Task 3: Add Documentation for CI/CD & Local Builds

**Files:**
- Modify: `README.md`

**Interfaces:**
- Consumes: workflow parameters from Task 1 and configurations from Task 2.
- Produces: Clear setup instructions in `README.md` for both GitHub Actions and local Windows/macOS developers.

- [ ] **Step 1: Update `README.md` with CI/CD and platform guides**
Document:
1. **GitHub Actions Workflow**:
   - How to trigger builds automatically on push / PR.
   - How to trigger manual builds via `workflow_dispatch` (Actions tab -> Run workflow).
   - How to release an installer: creating and pushing a Git tag (e.g. `git tag v0.1.0 && git push origin v0.1.0`).
   - Repository settings: enabling `Read and write permissions` under **Settings > Actions > General > Workflow permissions**.
2. **Local Environment Requirements**:
   - Windows: Visual Studio C++ Build Tools, Rust `x86_64-pc-windows-msvc`, Trunk, WebView2.
   - macOS: Xcode Command Line Tools, Rust (`stable`), Trunk.

- [ ] **Step 2: Review documentation clarity and verify links**
Ensure instructions are complete and accurate for Tauri v2 + Leptos.
