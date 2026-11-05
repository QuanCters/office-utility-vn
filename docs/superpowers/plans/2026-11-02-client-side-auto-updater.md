# Client-Side Auto & Manual Updater Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement an in-app automatic and manual update notification and upgrade system (Update Now, Update Later, Skip this Version) with SQLite persistence and manual check UI.

**Architecture:** 
- Shared DTOs in `crates/office-shared` for type-safe IPC communication between Rust and Leptos.
- Preference state persisted in SQLite `app_preferences` table via `src-tauri/src/db`.
- Tauri v2 IPC commands (`check_app_update`, `install_app_update`, `set_update_action`) with network/dev graceful fallbacks.
- Leptos 0.8 frontend: non-blocking delayed background check (3s post-startup), global Update Dialog modal, and Titlebar manual check button.

**Tech Stack:** Rust (Tauri v2, Rusqlite, Serde), Leptos 0.8 (WASM), CSS3.

**Spec:** [docs/specs/07-deployment-devops.md](file:///home/quan/workspace/tauri-v2/office-utility-vn/docs/specs/07-deployment-devops.md#L313-L339) and Design Document from brainstorming session.

## Global Constraints

- Tauri v2 architecture; Rust 2024 edition.
- `NFR-PERF-01`: Cold start $< 300\text{ ms}$; background update check must be non-blocking and deferred $\ge 3\text{ seconds}$ after frontend mount.
- `NFR-SEC-01`: No unauthorized network calls outside updater endpoints (`latest.json` on GitHub Releases).
- `NFR-SEC-03`: No unchecked `.unwrap()` or `.expect()` in production application logic.
- Zero Data Loss: Persist user choices in SQLite `app_preferences` without destructive DDL changes.

## Review Focus

1. **Offline / Network failure:** App must handle connection drop or timeout gracefully without crashing, freezing, or showing technical stack traces to the user.
2. **Skipped version suppression:** If user clicked "Skip this version" on `v0.2.0`, background check must not prompt for `v0.2.0`, but manual check must still display the update option.
3. **Remind later window:** If user clicked "Update later", background check must suppress modal until $24\text{ hours}$ have elapsed.
4. **Debounced manual trigger:** Multiple rapid clicks on "Kiểm tra bản cập nhật" must not trigger concurrent network requests or race conditions.
5. **Zero regression in existing test suites:** `cargo test --workspace` and `./scripts/verify_pod.sh` must remain 100% green.

---

### Task 1: Shared DTOs & SQLite Persistence Layer

**Files:**
- Modify: `crates/office-shared/src/lib.rs:160-167`
- Modify: `src-tauri/src/db/mod.rs:555-560`
- Test: `src-tauri/src/db/mod.rs` (inline test)

**Interfaces:**
- Produces:
  - `UpdateCheckResponse`: `{ has_update: bool, current_version: String, latest_version: String, release_notes: Option<String>, release_date: Option<String>, should_prompt: bool }`
  - `UpdaterPreference`: `{ skipped_version: Option<String>, remind_later_until: Option<String>, last_checked_at: Option<String> }`
  - `get_updater_preference(conn: &Connection) -> Result<UpdaterPreference, CommandError>`
  - `save_updater_preference(conn: &Connection, pref: &UpdaterPreference) -> Result<(), CommandError>`

- [ ] **Step 1: Write the failing test for UpdaterPreference persistence in `src-tauri/src/db/mod.rs`**

```rust
#[test]
fn test_updater_preference_persistence() {
    let conn = init_in_memory().unwrap();
    let pref = get_updater_preference(&conn).unwrap();
    assert_eq!(pref.skipped_version, None);

    let updated = UpdaterPreference {
        skipped_version: Some("v0.2.0".to_string()),
        remind_later_until: Some("2026-11-03T15:00:00Z".to_string()),
        last_checked_at: Some("2026-11-02T15:00:00Z".to_string()),
    };
    save_updater_preference(&conn, &updated).unwrap();

    let loaded = get_updater_preference(&conn).unwrap();
    assert_eq!(loaded.skipped_version.as_deref(), Some("v0.2.0"));
    assert_eq!(loaded.remind_later_until.as_deref(), Some("2026-11-03T15:00:00Z"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p office-utility-vn test_updater_preference_persistence`
Expected: FAIL with "cannot find function `get_updater_preference`"

- [ ] **Step 3: Define DTOs in `crates/office-shared/src/lib.rs` and implement helper functions in `src-tauri/src/db/mod.rs`**

Add `UpdateCheckResponse` and `UpdaterPreference` to `crates/office-shared/src/lib.rs`.
Implement `get_updater_preference` and `save_updater_preference` using `get_app_preference(&conn, "updater_preference")` and `set_app_preference(&conn, "updater_preference", &json)`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p office-utility-vn test_updater_preference_persistence`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/office-shared/src/lib.rs src-tauri/src/db/mod.rs
git commit -m "feat(db): add UpdaterPreference DTOs and SQLite persistence helpers"
```

---

### Task 2: Tauri IPC Commands for Update Lifecycle

**Files:**
- Create: `src-tauri/src/commands/updater.rs`
- Modify: `src-tauri/src/commands/mod.rs:1-20`
- Modify: `src-tauri/src/lib.rs:44-64`
- Test: `src-tauri/src/commands/updater.rs` (unit tests)

**Interfaces:**
- Consumes: `UpdaterPreference`, `UpdateCheckResponse`, `get_updater_preference`, `save_updater_preference` from Task 1.
- Produces:
  - `#[tauri::command] pub async fn check_app_update(app_handle: AppHandle, state: State<'_, AppState>, manual: bool) -> Result<UpdateCheckResponse, CommandError>`
  - `#[tauri::command] pub async fn install_app_update(app_handle: AppHandle) -> Result<(), CommandError>`
  - `#[tauri::command] pub async fn set_update_action(state: State<'_, AppState>, action: String, target_version: Option<String>) -> Result<(), CommandError>`

- [ ] **Step 1: Write unit tests for update decision logic (skip and remind_later filtering)**

```rust
#[test]
fn test_should_prompt_decision_logic() {
    let current_version = "0.1.0";
    let latest_version = "0.2.0";
    
    // Case 1: Brand new update, no preference -> should prompt
    let pref = UpdaterPreference::default();
    assert!(evaluate_should_prompt(&pref, current_version, latest_version, false));

    // Case 2: Skipped this version -> should not prompt on background check
    let pref_skipped = UpdaterPreference {
        skipped_version: Some("0.2.0".to_string()),
        remind_later_until: None,
        last_checked_at: None,
    };
    assert!(!evaluate_should_prompt(&pref_skipped, current_version, latest_version, false));
    // But manual check overrides skip!
    assert!(evaluate_should_prompt(&pref_skipped, current_version, latest_version, true));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p office-utility-vn test_should_prompt_decision_logic`
Expected: FAIL with "cannot find function `evaluate_should_prompt`"

- [ ] **Step 3: Implement `src-tauri/src/commands/updater.rs` and register commands in `src-tauri/src/lib.rs`**

Implement `evaluate_should_prompt`, `check_app_update`, `install_app_update`, and `set_update_action`.
Include network error handling and graceful fallback if updater endpoint is unavailable in development.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p office-utility-vn test_should_prompt_decision_logic`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/updater.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat(updater): implement Tauri IPC commands for update check and actions"
```

---

### Task 3: Tauri Configuration & Plugin Integration

**Files:**
- Modify: `src-tauri/Cargo.toml:17-28`
- Modify: `src-tauri/tauri.conf.json:20-30`

**Interfaces:**
- Configures: `plugins.updater` with GitHub Releases `latest.json` endpoint and public key placeholder.

- [ ] **Step 1: Check existing `tauri.conf.json` and `Cargo.toml` dependencies**

Run: `cargo check -p office-utility-vn` to confirm baseline clean build.

- [ ] **Step 2: Configure `plugins.updater` in `src-tauri/tauri.conf.json`**

Add:
```json
"plugins": {
  "updater": {
    "endpoints": [
      "https://github.com/quan/office-utility-vn/releases/latest/download/latest.json"
    ],
    "pubkey": "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IHRlc3QtcHVia2V5Cg=="
  }
}
```

- [ ] **Step 3: Verify compilation across workspace**

Run: `cargo check --workspace`
Expected: SUCCESS with code 0.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "chore(updater): configure updater plugin endpoints in tauri.conf.json"
```

---

### Task 4: Leptos Frontend Global Update Modal & Manual Check UI

**Files:**
- Modify: `src/app.rs:550-600` (Titlebar manual check button)
- Modify: `src/app.rs:770-830` (Global Update Modal component & delayed background check trigger)
- Modify: `styles.css` (Modal styling for dark/light mode consistency)

**Interfaces:**
- Consumes: Tauri IPC `invoke("check_app_update", ...)`, `invoke("install_app_update")`, `invoke("set_update_action", ...)`
- Signals:
  - `show_update_modal: RwSignal<bool>`
  - `update_info: RwSignal<Option<UpdateCheckResponse>>`
  - `is_checking_update: RwSignal<bool>`
  - `is_installing_update: RwSignal<bool>`

- [ ] **Step 1: Add update modal and button CSS in `styles.css`**

Add modern styling for `.update-modal`, `.update-btn-group`, `.btn-update-now`, `.btn-update-later`, `.btn-skip-version`, and `.titlebar-update-btn`.

- [ ] **Step 2: Add delayed background check and manual trigger in `src/app.rs`**

- After mount: `spawn_local` with 3-second delay, invoking `check_app_update(manual: false)`.
- If `res.has_update && res.should_prompt`, set `update_info` and open `show_update_modal`.
- Add "Kiểm tra cập nhật" badge/button next to `v0.1.0` in the titlebar.
- Add handler for "Update Now" -> invokes `install_app_update`.
- Add handler for "Update Later" -> invokes `set_update_action("LATER", ...)` and closes modal.
- Add handler for "Skip this version" -> invokes `set_update_action("SKIP", ...)` and closes modal.

- [ ] **Step 3: Verify Trunk / WASM build succeeds**

Run: `trunk build`
Expected: SUCCESS, generates `dist/` artifacts without errors.

- [ ] **Step 4: Commit**

```bash
git add src/app.rs styles.css
git commit -m "feat(ui): add global update modal and titlebar update button in Leptos"
```

---

### Task 5: Quality Gate & Full Verification

**Files:**
- Modify: `scripts/verify_spine_lock.sh` (update expected sha256 hashes for modified spine files)
- Test: Run `./scripts/verify_pod.sh` and `cargo test --workspace`

- [ ] **Step 1: Run full cargo test suite**

Run: `cargo test --workspace`
Expected: 100% tests PASS across all crates (`office-shared`, `office-engine`, `office-utility-vn`).

- [ ] **Step 2: Run pod verification script**

Run: `./scripts/verify_pod.sh vietnamese-number-to-words`
Expected: PASS (fmt, clippy, NFR audit, unit tests).

- [ ] **Step 3: Update and run `scripts/verify_spine_lock.sh`**

Compute new SHA256 hashes for `crates/office-shared/src/lib.rs`, `src-tauri/src/db/mod.rs`, and `src-tauri/src/commands/mod.rs`, update `verify_spine_lock.sh`, and run:
`./scripts/verify_spine_lock.sh`
Expected: All checks PASS with code 0.

- [ ] **Step 4: Final commit for plan completion**

```bash
git add scripts/verify_spine_lock.sh
git commit -m "chore(spine): update spine lock hashes after updater feature integration"
```

