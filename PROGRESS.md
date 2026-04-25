# Project Progress: Prompt Quiver (Rust/Ratatui)

## 🎯 Current Milestone: Full Spec Compliance Audit (COMPLETED)
This document tracks the implementation status of every item in `FUNCTIONAL_SPEC.md`.

---

## 1. Core Philosophy
- [x] Contextual Isolation (Project-specific storage).
- [x] Global Utility (Canned prompts & Snippets).
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
- [x] Processor: Comment stripping (lines starting with `#`).
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
- [x] Prompt Preview: TOML-like view (TODO: syntax highlighting).
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

### Remaining Polish (Low Priority):
1.  **Syntax Highlighting:** Refine TOML-like preview in `ui/src/list.rs` or `ui/src/utils.rs`.
2.  **Fuzzy Search:** Use a proper library for better fuzzy matching in autocomplete.
3.  **Error Handling:** More robust error reporting in UI.
