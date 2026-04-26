# Project Progress: Prompt Quiver (Rust/Ratatui)

## 🎯 Current Milestone: Full Spec Compliance Audit (COMPLETED)
This document tracks the implementation status of every item in `FUNCTIONAL_SPEC.md`.

---

## 1. Core Philosophy
- [x] Contextual Isolation (Project-specific storage - Prompts, Notes, Archive).
- [x] Global Utility (Global storage - Canned prompts, Snippets, Settings).
- [x] High-Speed Workflow (Vim-inspired keybindings).
- [x] Performance (Async I/O, Clean Architecture).

---

## 2. Storage & Persistence (`infra`)
- [x] Project Storage (`.toml` in data dir, hashed by path).
- [x] Project Storage: `[info]` section with `path`.
- [x] Project Storage: `main` (Prompts), `notes`, `archive`.
- [x] Global Storage (`common.toml`).
- [x] Global Storage: `canned`, `snippets`.
- [x] Global Storage: `[settings]` section.
- [x] Settings Schema: `tab_visibility`, `slash_commands`, `enable_claude_commands`.
- [x] Atomic Writes (Temp file + Rename).

---

## 3. Core Logic (`contracts` & `app`)
- [x] Processor: Snippet expansion (`$$snippet`).
- [x] Processor: Title extraction (`-- Title`).
- [x] Processor: Comment stripping (lines starting with `--`).
- [x] Staging State Machine: Only one item staged globally.
- [x] Transition: Move currently staged item to Archive (Top).
- [x] Transition: Un-stage others (Canned).
- [x] Undo/Redo: Session-based history stack.
- [x] Mandatory `name` for Snippets.

---

## 4. User Interface (`ui`)
- [x] 6-Tab Layout (Prompts, Canned, Notes, Snippets, Archive, Settings).
- [x] Header: App Title ("PROMPT QUIVER"), Tabs, Current Branch.
- [x] Prompt List: Display Title, Staged Icon (🎯).
- [x] Use Display Title in list rendering.
- [x] Prompt Preview: TOML-like view (with syntax highlighting).
- [x] Editor: Full-screen `ratatui-textarea`.
- [x] Editor Mentions (Autocomplete): `$` Trigger snippet insertion.
- [x] Editor Mentions (Autocomplete): `$$` Trigger snippet name insertion.
- [x] Editor Mentions (Autocomplete): `@` Fuzzy file search (Basic recursive walk).
- [x] Editor Mentions (Autocomplete): `/` Slash commands (From settings).
- [x] Toasts: Transient notifications for Copy/Save/Error.
- [x] Footer: Keybinding hints based on mode.

---

## 5. Navigation & Interaction
- [x] Tab Switching (`Tab / Right / Left / 1-6`).
- [x] List Navigation (`j / k / Up / Down / G / gg`).
- [x] Branch Filter mode (`b`).
- [x] Local Search (filter current view) (`/`).
- [x] Global Search (across all project files) (`G`).
- [x] Mode Toggle: Move mode (reorder with `j / k`) (`m`).

---

## 6. Interaction Map (Keybindings)
- [x] `q`: Quit.
- [x] `s`: Stage selected.
- [x] `d`: Archive selected (or permanent delete if in Archive).
- [x] `a / i`: Add after / before.
- [x] `e / Enter`: Edit selected.
- [x] `y / c`: Copy processed text.
- [x] `u / Ctrl+y`: Undo / Redo.
- [x] `r`: Restore from Archive.
- [x] `m`: Move mode (reorder).
- [x] `/`: Local search.
- [x] `G`: Global search.
- [x] `b`: Toggle branch filter.
- [x] `Ctrl+s`: Save and exit (Editor).
- [x] `Ctrl+g`: Save, Stage, and exit (Editor).
- [x] `Esc`: Close (confirm if modified).

---

### ✅ Status (2026-04-25)
Project is now fully spec-compliant. All missing features identified in the audit have been implemented and verified with a successful build.

### Bug Fixes (2026-04-25):
- [x] Fixed issue where exiting editor with modifications would navigate back to list view before confirmation. Now correctly shows a modal over the editor.
- [x] Updated E2E tests to match latest `ui::render` signature and `Mode` enum.
- [x] Fixed navigation in Settings view (j/k and up/down arrows).
- [x] Removed `Tab` from global tab switching; it now moves focus between widgets within a view (implemented for Settings sections).
- [x] Implemented `gg` and `G` navigation (Go to Top/Bottom) in all list views.
- [x] Remapped Global Search to `Ctrl+f` to resolve conflict with `G` navigation.
- [x] Fixed issue where editing slash commands in Settings opened the multi-line Prompt Editor. It now opens an inline edit view specifically for the Slash Commands field.
- [x] Fixed issue where deleting the trigger character (e.g., '/') for autocomplete would not dismiss the suggestion box. Autocomplete now also correctly dismisses when typing a space.
- [x] Fixed autocomplete box title rendering. It now displays " Commands " for Slash commands, " Files " for `@` mentions, and " Snippets " for standard snippets instead of hardcoding " Snippets ".

### Remaining Polish (Low Priority) [COMPLETED]:
1.  [x] **Syntax Highlighting:** Refined TOML-like preview in `ui/src/list.rs` and `ui/src/utils.rs`.
2.  [x] **Fuzzy Search:** Integrated `fuzzy-matcher` crate for better fuzzy matching in autocomplete.
3.  [x] **Error Handling:** Enhanced error reporting in UI with `handle_error!` macro in `app/src/main.rs`.

### Settings Refactor (2026-04-26):
- [x] **Slash Command List:** Refactored slash commands in Settings from a single text area to an interactive list.
- [x] **Individual Editing:** Enabled Enter key to edit specific slash commands in-line.
- [x] **Validation:** Implemented regex validation `[a-zA-Z0-9_-]+` for slash command names.
- [x] **One-Liners:** Enforced one-line editing for slash commands in Settings.
- [x] **Case-Insensitive Suggestions:** Updated autocomplete to ignore case for slash commands.
- [x] **Deletion:** Mapped 'd' key to delete the selected slash command.
- [x] **Navigation:** Improved navigation within the settings view using j/k and arrow keys.
- [x] **Add New:** Added a dedicated "Add New Slash Command" item at the end of the list.

### Test Architecture Refactor (2026-04-26):
- [x] **Modular E2E Tests:** Split the monolithic `app/tests/e2e.rs` into logical files: `navigation.rs`, `editing.rs`, `autocomplete.rs`, `settings.rs`, and `workflow.rs`.
- [x] **Shared Test Utilities:** Created `app/tests/common/mod.rs` to deduplicate setup logic and constants.
- [x] **Improved Scope Management:** Properly scoped trait imports across all new test crates.
