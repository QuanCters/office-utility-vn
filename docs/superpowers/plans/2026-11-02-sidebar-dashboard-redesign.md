# Sidebar-First Dashboard Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the flat 3-tab layout (Tools/WorkflowDAG/History) with a persistent left sidebar containing favorites, recents, domain-grouped tool tree, and secondary nav — while preserving the existing zone-1/2/3 workspace layout in the main area, and adding a lightweight Home welcome view.

**Architecture:** The `app-window` container changes from `flex-direction: column` (titlebar → tab-bar → content) to a two-column layout: titlebar stays full-width on top, below it a `flex-direction: row` container holds sidebar (240px, collapsible to 48px) and main-area (flex: 1). The `WorkspaceView` enum is replaced by sidebar navigation state that drives which content renders in the main area. A new `favorite_tools` module handles user-customizable favorites persisted in LocalStorage alongside existing `recent_tools`.

**Tech Stack:** Leptos 0.8 (CSR), Tauri v2, Rust/WASM, existing CSS variables (`--bg-main`, `--bg-surface`, `--bg-card`, `--border-color`, `--primary`)

**Spec:** `docs/superpowers/specs/2026-11-02-dashboard-redesign-approaches.md` (artifact in brain)

## Global Constraints

- All UI text must be 100% Vietnamese (`vi-VN`), matching existing copy style
- No network calls from UI layer (Zero-Cloud)
- Cold start < 300ms — sidebar must not add perceptible load time
- Keyboard shortcuts must be preserved: `Ctrl+K` (search), `Ctrl+Enter` (execute), `Ctrl+P` (pin window), `Ctrl+O` (open output), `Ctrl+W` (close), `Esc` (cancel)
- WCAG 2.1 AA contrast ratios (≥ 4.5:1 text) using existing Slate/Indigo palette
- IME compatibility (UniKey/EVKey) — sidebar search input must handle `compositionstart`/`compositionend`
- `is_pinned` on `ToolItem` remains a static default; favorites are a separate user-configurable list

## Review Focus

1. **Sidebar width on small windows** (1024×700): collapsed state must trigger automatically or be easily accessible; main area must not compress below ~680px usable width
2. **Keyboard focus trap**: Tab navigation must flow logically from sidebar items into main workspace; sidebar search should not steal focus from active tool form inputs
3. **State persistence across restarts**: favorites and sidebar collapsed state must survive app restart (LocalStorage); corrupted JSON must degrade gracefully (empty defaults)
4. **Tool selection during active task**: switching tools via sidebar while `is_running == true` should warn or queue, not lose progress
5. **Domain group expand/collapse memory**: collapsed domains should persist per session (signal state) so user doesn't re-expand every time they switch tools

---

## File Structure

| Action | File | Responsibility |
|--------|------|----------------|
| Create | `src/favorite_tools.rs` | Favorites list CRUD + LocalStorage persistence (mirrors `recent_tools.rs`) |
| Create | `src/sidebar.rs` | `<Sidebar>` component: search, favorites, recents, domain groups, secondary nav |
| Create | `src/home_view.rs` | `<HomeView>` component: welcome, recent cards, stats summary, keyboard hints |
| Modify | `src/app.rs` | Replace `WorkspaceView` enum + tab bar with sidebar layout; extract tool workspace into own closure; integrate `<Sidebar>` and `<HomeView>` |
| Modify | `src/main.rs` | Add `mod favorite_tools; mod sidebar; mod home_view;` |
| Modify | `styles.css` | Add sidebar, home-view, domain-group CSS; remove/deprecate `sub-header-nav`, `nav-tabs`, `nav-tab-btn`, `dashboard-recent-bar` styles |
| Test | `src/favorite_tools.rs` (inline `#[cfg(test)]`) | Unit tests for favorites CRUD logic |

---

### Task 1: Favorite Tools Module

**Files:**
- Create: `src/favorite_tools.rs`
- Modify: `src/main.rs:1` (add `pub mod favorite_tools;`)

**Interfaces:**
- Consumes: nothing (standalone module)
- Produces:
  - `pub const FAVORITE_TOOLS_STORAGE_KEY: &str`
  - `pub const DEFAULT_MAX_FAVORITES: usize` (= 10)
  - `pub fn toggle_favorite(existing: &[String], slug: &str) -> Vec<String>` — adds if absent, removes if present
  - `pub fn is_favorite(favorites: &[String], slug: &str) -> bool`
  - `pub fn load_favorites_from_storage() -> Vec<String>`
  - `pub fn save_favorites_to_storage(slugs: &[String])`

- [ ] **Step 1: Write failing tests for `toggle_favorite`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toggle_add_new_favorite() {
        let existing = vec!["tool-a".to_string()];
        let result = toggle_favorite(&existing, "tool-b");
        assert_eq!(result, vec!["tool-a", "tool-b"]);
    }

    #[test]
    fn test_toggle_remove_existing_favorite() {
        let existing = vec!["tool-a".to_string(), "tool-b".to_string()];
        let result = toggle_favorite(&existing, "tool-a");
        assert_eq!(result, vec!["tool-b"]);
    }

    #[test]
    fn test_toggle_empty_slug_noop() {
        let existing = vec!["tool-a".to_string()];
        let result = toggle_favorite(&existing, "  ");
        assert_eq!(result, existing);
    }

    #[test]
    fn test_toggle_respects_max_limit() {
        let existing: Vec<String> = (0..10).map(|i| format!("tool-{}", i)).collect();
        let result = toggle_favorite(&existing, "tool-new");
        assert_eq!(result.len(), 10); // should not exceed DEFAULT_MAX_FAVORITES
    }

    #[test]
    fn test_is_favorite_true_and_false() {
        let favs = vec!["pdf-merge".to_string()];
        assert!(is_favorite(&favs, "pdf-merge"));
        assert!(!is_favorite(&favs, "pdf-split"));
    }

    #[test]
    fn test_deserialize_corrupt_favorites_fallback() {
        // Review Focus: corrupted JSON must degrade gracefully
        let corrupt = "{not valid json[";
        let result: Vec<String> = serde_json::from_str(corrupt).unwrap_or_default();
        assert!(result.is_empty());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /home/quan/workspace/tauri-v2/office-utility-vn && cargo test --lib favorite_tools -- --nocapture`
Expected: FAIL — module does not exist

- [ ] **Step 3: Implement `favorite_tools.rs`**

Mirror the structure of `recent_tools.rs`: same `#[cfg(target_arch = "wasm32")]` gating for `web_sys::window().local_storage()`, same `serde_json` serialization. `toggle_favorite` checks if slug exists → removes; else appends (up to `DEFAULT_MAX_FAVORITES`). Add `pub mod favorite_tools;` to `main.rs`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /home/quan/workspace/tauri-v2/office-utility-vn && cargo test --lib favorite_tools -- --nocapture`
Expected: all 5 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/favorite_tools.rs src/main.rs
git commit -m "feat: add favorite_tools module with toggle, persistence, and tests"
```

---

### Task 2: Sidebar Component

**Files:**
- Create: `src/sidebar.rs`
- Modify: `src/main.rs:1` (add `pub mod sidebar;`)
- Modify: `styles.css` (append sidebar CSS)

**Interfaces:**
- Consumes:
  - `crate::app::ToolItem` — the tool struct with `slug`, `name`, `domain`, `is_pinned`
  - `crate::app::ALL_TOOLS: &[ToolItem]` — static tool list
  - `crate::favorite_tools::{toggle_favorite, is_favorite, load_favorites_from_storage, save_favorites_to_storage}` from Task 1
  - `crate::recent_tools::load_recent_from_storage` (existing)
- Produces:
  - `pub fn Sidebar(cx: Scope) -> impl IntoView` component accepting these props via Leptos signals passed as props:
    - `current_slug: ReadSignal<String>` — which tool is active
    - `on_select_tool: Callback<String>` — called when user clicks a tool
    - `on_navigate: Callback<SidebarDestination>` — called for Workflow/History/Home clicks
    - `favorite_slugs: ReadSignal<Vec<String>>` — current favorites
    - `recent_slugs: ReadSignal<Vec<String>>` — current recents
    - `on_toggle_favorite: Callback<String>` — toggle a tool's favorite status
  - `pub enum SidebarDestination { Home, WorkflowDag, History }`
  - Domain groups derived from `ALL_TOOLS` grouped by `domain` field

- [ ] **Step 1: Define `SidebarDestination` enum and `Sidebar` component signature in `src/sidebar.rs`**

Create the file with the enum, the component function signature with all props, and a minimal placeholder body that renders a `<nav class="app-sidebar">` with static text. Add `pub mod sidebar;` to `main.rs`.

- [ ] **Step 2: Verify it compiles**

Run: `cd /home/quan/workspace/tauri-v2/office-utility-vn && cargo check --lib`
Expected: compiles with no errors (component not yet mounted)

- [ ] **Step 3: Implement sidebar sections — Search input**

Add an `<input>` with class `sidebar-search-input` inside the sidebar. Wire a local `(search_query, set_search_query)` signal. When non-empty, filter `ALL_TOOLS` and show results below; when empty, show the normal sections. Include `on:compositionstart` / `on:compositionend` for IME safety.

- [ ] **Step 4: Implement sidebar sections — Favorites**

Render a `<div class="sidebar-section">` with header "★ YÊU THÍCH". Map over `favorite_slugs.get()`, look up each slug in `ALL_TOOLS`, render as `<button class="sidebar-tool-item">` with tool name. On click → call `on_select_tool`. Add a ★ toggle button on hover that calls `on_toggle_favorite`.

- [ ] **Step 5: Implement sidebar sections — Recents**

Render "🕒 GẦN ĐÂY" section. Map over `recent_slugs.get().iter().take(5)`, same rendering as favorites but without toggle button.

- [ ] **Step 6: Implement sidebar sections — Domain groups**

Define a static list of domain groups:
```rust
const DOMAIN_GROUPS: &[(&str, &str, &str)] = &[
    ("doc-suite", "📄", "Document Suite"),
    ("media-studio", "🎨", "Media Studio"),
    ("data-lab", "📊", "Data Lab"),
    ("vn-office", "🇻🇳", "Nghiệp vụ VN"),
    ("sys-hub", "⚙️", "System Hub"),
];
```
For each group, create a collapsible section: header with count badge, expand/collapse toggled by a local `RwSignal<bool>`. Show tools belonging to that domain. Highlight active tool with `sidebar-tool-item active` class.

- [ ] **Step 7: Implement sidebar sections — Secondary nav (bottom)**

Below a separator, add three buttons: "⚡ Workflow DAG", "⏱ Lịch sử", styled as `sidebar-nav-item`. On click → call `on_navigate` with the corresponding `SidebarDestination`.

- [ ] **Step 8: Implement sidebar collapse toggle**

Add a `(sidebar_collapsed, set_sidebar_collapsed)` signal. When collapsed, sidebar renders with class `app-sidebar collapsed` (width 48px, show only icons). Add a toggle button at the bottom or top of the sidebar (`«` / `»`).

- [ ] **Step 9: Add sidebar CSS to `styles.css`**

Append CSS rules for: `.app-sidebar` (width 240px, `--bg-main` background, left border, flex column, overflow-y auto), `.app-sidebar.collapsed` (width 48px), `.sidebar-search-input`, `.sidebar-section`, `.sidebar-section-header`, `.sidebar-tool-item` (hover: `--bg-surface`, active: indigo left border + text), `.sidebar-tool-item .fav-toggle`, `.sidebar-nav-item`, `.sidebar-collapse-btn`. All using existing CSS variables.

- [ ] **Step 10: Verify it compiles**

Run: `cd /home/quan/workspace/tauri-v2/office-utility-vn && cargo check --lib`
Expected: compiles (component defined but not yet integrated into App)

- [ ] **Step 11: Commit**

```bash
git add src/sidebar.rs src/main.rs styles.css
git commit -m "feat: add Sidebar component with search, favorites, recents, domain groups, and collapse"
```

---

### Task 3: Home Welcome View

**Files:**
- Create: `src/home_view.rs`
- Modify: `src/main.rs:1` (add `pub mod home_view;`)
- Modify: `styles.css` (append home view CSS)

**Interfaces:**
- Consumes:
  - `crate::app::ALL_TOOLS: &[ToolItem]`
  - `recent_slugs: ReadSignal<Vec<String>>` — for "Tiếp tục" cards
  - `on_select_tool: Callback<String>` — navigate to a tool
  - `history_records: ReadSignal<Vec<TaskHistoryUi>>` — for stats summary (optional, can start with static)
- Produces:
  - `pub fn HomeView(cx: Scope) -> impl IntoView` component

- [ ] **Step 1: Create `src/home_view.rs` with `HomeView` component**

Render a centered `<div class="home-view">` containing:
1. Header: "Office Utility VN v0.1.0" + "🔒 54 công cụ · 100% On-Device"
2. "TIẾP TỤC" section: up to 3 recent tool cards as `<button class="home-recent-card">` showing tool name; on click → `on_select_tool`
3. Keyboard hints section: `Ctrl+K → Tìm nhanh`, `Ctrl+⏎ → Thực thi`, `Ctrl+P → Ghim cửa sổ`

Add `pub mod home_view;` to `main.rs`.

- [ ] **Step 2: Add home view CSS to `styles.css`**

Append: `.home-view` (max-width 600px, margin auto, padding 48px), `.home-recent-card` (background `--bg-surface`, border, border-radius 8px, hover: indigo border), `.home-kbd` (monospace badge for keyboard shortcuts).

- [ ] **Step 3: Verify it compiles**

Run: `cd /home/quan/workspace/tauri-v2/office-utility-vn && cargo check --lib`
Expected: compiles

- [ ] **Step 4: Commit**

```bash
git add src/home_view.rs src/main.rs styles.css
git commit -m "feat: add HomeView welcome component with recent cards and keyboard hints"
```

---

### Task 4: Integrate Sidebar Layout into App

This is the main refactor task — replaces the tab navigation with the sidebar layout.

**Files:**
- Modify: `src/app.rs:26-30` (replace `WorkspaceView` enum)
- Modify: `src/app.rs:518-1998` (refactor `App` component layout)
- Modify: `styles.css` (add `.app-body` layout, deprecate tab-nav styles)

**Interfaces:**
- Consumes:
  - `crate::sidebar::{Sidebar, SidebarDestination}` from Task 2
  - `crate::home_view::HomeView` from Task 3
  - `crate::favorite_tools::*` from Task 1
- Produces: Updated `App` component with sidebar-driven navigation

- [ ] **Step 1: Replace `WorkspaceView` enum with `AppView`**

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppView {
    Home,
    Tool,        // active tool determined by current_slug
    WorkflowDag,
    History,
}
```

Replace `(active_view, set_active_view) = signal(WorkspaceView::Tools)` with `signal(AppView::Home)`.

- [ ] **Step 2: Add favorites signals**

```rust
let (favorite_slugs, set_favorite_slugs) = signal(crate::favorite_tools::load_favorites_from_storage());

let toggle_fav = move |slug: String| {
    let updated = crate::favorite_tools::toggle_favorite(&favorite_slugs.get(), &slug);
    crate::favorite_tools::save_favorites_to_storage(&updated);
    set_favorite_slugs.set(updated);
};
```

- [ ] **Step 3: Update `select_tool` to set `AppView::Tool`**

In the existing `select_tool` closure, change `set_active_view.set(WorkspaceView::Tools)` to `set_active_view.set(AppView::Tool)`.

- [ ] **Step 4: Restructure the view! macro — titlebar stays, add app-body row**

Replace the current structure:
```
<div class="app-window">
  <header class="titlebar">...</header>
  <nav class="sub-header-nav">... tabs ...</nav>
  { match active_view ... }
</div>
```

With:
```
<div class="app-window">
  <header class="titlebar">...</header>
  <div class="app-body">
    <Sidebar
      current_slug=current_slug
      on_select_tool=Callback::new(move |slug| select_tool(slug))
      on_navigate=Callback::new(move |dest| { ... set_active_view based on dest })
      favorite_slugs=favorite_slugs.into()
      recent_slugs=recent_tool_slugs.into()
      on_toggle_favorite=Callback::new(move |slug| toggle_fav(slug))
    />
    <div class="main-area">
      { match active_view.get() {
          AppView::Home => view! { <HomeView ... /> },
          AppView::Tool => view! { <main class="workspace"> ... zones ... </main> },
          AppView::WorkflowDag => view! { ... existing workflow view ... },
          AppView::History => view! { ... existing history view ... },
      }}
    </div>
  </div>
</div>
```

- [ ] **Step 5: Remove the `<nav class="sub-header-nav">` block entirely**

Delete lines rendering the 3 tab buttons and the "🔒 Zero-Cloud Security" badge (this info moves to sidebar or home view).

- [ ] **Step 6: Remove the `dashboard-recent-bar` chip bar**

Delete the `recent_tool_items()` chip bar that was rendered at the top of the Tools workspace — sidebar recents section replaces it.

- [ ] **Step 7: Add `.app-body` and `.main-area` CSS**

```css
.app-body {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.main-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
```

- [ ] **Step 8: Update Workflow and History view heights**

Change `.workflow-workspace` and `.history-workspace` from `height: calc(100vh - 100px)` to `height: 100%` (they now fill `.main-area` instead of the full viewport minus titlebar+tabs).

- [ ] **Step 9: Verify full compilation**

Run: `cd /home/quan/workspace/tauri-v2/office-utility-vn && cargo check --lib`
Expected: compiles with no errors

- [ ] **Step 10: Build and verify WASM output**

Run: `cd /home/quan/workspace/tauri-v2/office-utility-vn && trunk build` (or project's build command)
Expected: successful WASM build

- [ ] **Step 11: Commit**

```bash
git add src/app.rs styles.css
git commit -m "feat: replace tab navigation with sidebar-first layout

- Remove WorkspaceView 3-tab enum, add AppView with Home/Tool/Workflow/History
- Integrate Sidebar component with favorites, recents, domain groups
- Add HomeView as default landing
- Preserve zone-1/2/3 workspace layout for tools
- Remove sub-header-nav tab bar and dashboard-recent-bar chip bar"
```

---

### Task 5: CSS Cleanup and Polish

**Files:**
- Modify: `styles.css`

**Interfaces:**
- Consumes: Final rendered layout from Task 4
- Produces: Polished, clean stylesheet without dead CSS

- [ ] **Step 1: Remove or comment out deprecated CSS rules**

Mark as deprecated (comment block): `.sub-header-nav`, `.nav-tabs`, `.nav-tab-btn`, `.nav-tab-btn:hover`, `.nav-tab-btn.active`, `.dashboard-recent-bar`, `.dashboard-recent-label`, `.dashboard-recent-chips`, `.recent-tool-chip`, `.recent-tool-chip:hover`, `.recent-tool-chip.active`.

- [ ] **Step 2: Add sidebar active-tool visual indicator**

Ensure `.sidebar-tool-item.active` has a visible left border accent (`border-left: 3px solid var(--primary)`) and subtle background tint.

- [ ] **Step 3: Add sidebar collapsed responsive behavior**

Add a media query or JS-driven class: when window width < 1100px, auto-collapse sidebar to 48px.

- [ ] **Step 4: Verify build**

Run: `cd /home/quan/workspace/tauri-v2/office-utility-vn && cargo check --lib`
Expected: compiles

- [ ] **Step 5: Commit**

```bash
git add styles.css
git commit -m "style: clean up deprecated tab-nav CSS, polish sidebar active states and responsive collapse"
```

---

### Task 6: Integration Test — Full Navigation Flow

**Files:**
- Modify: existing test infrastructure or manual verification

**Interfaces:**
- Consumes: All previous tasks
- Produces: Verified working navigation

- [ ] **Step 1: Verify `favorite_tools` unit tests pass**

Run: `cd /home/quan/workspace/tauri-v2/office-utility-vn && cargo test --lib favorite_tools`
Expected: all PASS

- [ ] **Step 2: Verify `recent_tools` existing tests still pass**

Run: `cd /home/quan/workspace/tauri-v2/office-utility-vn && cargo test --lib recent_tools`
Expected: all 6 existing tests PASS (no regression)

- [ ] **Step 3: Verify full project compiles**

Run: `cd /home/quan/workspace/tauri-v2/office-utility-vn && cargo check --lib`
Expected: 0 errors, 0 warnings related to our changes

- [ ] **Step 4: Run full test suite**

Run: `cd /home/quan/workspace/tauri-v2/office-utility-vn && cargo test`
Expected: all tests pass

- [ ] **Step 5: Commit final state**

```bash
git add -A
git commit -m "test: verify sidebar dashboard redesign — all tests pass"
```
