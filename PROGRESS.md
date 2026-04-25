# Project Progress: Prompt Quiver (Rust/Ratatui)

## 🎯 Current Milestone: Full Spec Compliance Audit
This document tracks the implementation status of every item in `FUNCTIONAL_SPEC.md`.

---

## 1. Core Philosophy
- [x] Context-Aware (Project-based storage)
- [x] Global Knowledge (Snippets, Canned, Settings)
- [x] Rapid Throughput (Keyboard-driven)
- [ ] Non-Destructive (Archive implemented; **Undo/Redo MISSING**)

---

## 2. Data Model

### 2.1 The Prompt Object
- [x] Fields: `id`, `text`, `type`, `branch`, `name`, `staged`, `created_at`, `updated_at`
- [ ] Mandatory `name` for Snippets (**Currently optional in editor**)

### 2.2 Storage Structure
- [x] TOML format
- [x] Standard OS data directory
- [ ] Project Storage: `[info]` section with `path` (**MISSING**)
- [x] Project Storage: 8-char SHA-256 hash filenames
- [x] Global Storage: `common.toml`
- [ ] Global Storage: `[settings]` section (**MISSING**)

### 2.3 Settings Schema (**ENTIRELY MISSING**)
- [ ] `tab_visibility` map
- [ ] `slash_commands` list
- [ ] `enable_claude_commands` toggle

---

## 3. Logic & Algorithms

### 3.1 Comment Processing
- [x] Extract Title (`-- Title` + empty line)
- [x] Strip Comments (lines starting with `--`)
- [ ] Use Display Title in list rendering (**Currently using text/name**)

### 3.2 Snippet Expansion
- [x] Pattern: `$$name`
- [x] Recursive expansion implementation

### 3.3 The Staging State Machine
- [x] Identify target item
- [x] Transition: Un-stage others (Main, Notes, Snippets)
- [ ] Transition: Un-stage others (Canned) (**Currently missed in logic**)
- [x] Transition: Move un-staged to top of Archive
- [x] Transition: Set target to staged
- [x] Action: Copy processed text to clipboard

### 3.4 Editor Mentions (Autocomplete)
- [ ] `@` Fuzzy file search (**Triggered but no results**)
- [x] `$` Fuzzy Snippet search (Full text)
- [x] `$$` Fuzzy Snippet search (Variable reference)
- [ ] `/` Slash commands (**Triggered but no results**)

---

## 4. Feature Catalog

### 4.1 Navigation
- [x] Tabs: Prompts, Canned, Notes, Snippets, Archive, Settings
- [ ] Branch Filter mode (`b`) (**MISSING**)
- [ ] Local Search (filter current view) (**MISSING**)
- [ ] Global Search (across all project files) (**MISSING**)

### 4.2 Undo/Redo (**MISSING**)
- [ ] Session-based history stack

### 4.3 Git Integration
- [x] Automatic branch detection
- [ ] Assign branch to new items (**Currently created with None**)

---

## 5. Interaction Map (Keybindings)

### Global
- [x] `Tab / Shift+Tab`: Cycle tabs
- [x] `Left / Right`: Cycle tabs
- [x] `1-6`: Jump to tab
- [x] `q`: Quit
- [ ] `u / Ctrl+y`: Undo / Redo (**MISSING**)

### List View
- [x] `j / k`: Move selection
- [x] `h / l`: Cycle tabs
- [x] `Enter / e`: Edit
- [ ] `a / i`: Add after / before (**'i' missing; 'a' always adds to end**)
- [ ] `y / c`: Copy processed text (**MISSING**)
- [x] `s`: Stage / Un-stage
- [x] `d`: Move to Archive / Delete
- [ ] `r`: Restore from Archive (**MISSING**)
- [ ] `m`: Move mode (reorder) (**MISSING**)
- [ ] `/`: Local search (**MISSING**)
- [ ] `G`: Global search (**MISSING**)
- [ ] `b`: Toggle branch filter (**MISSING**)

### Editor
- [x] `Ctrl+s`: Save and exit
- [ ] `Ctrl+g`: Save, Stage, and exit (**MISSING**)
- [ ] `Esc`: Close (confirm if modified) (**Close works, no confirmation**)
- [x] `Up / Down`: Navigate autocomplete

---

## 📓 Technical Notes
- **Shell:** Windows PowerShell (use `;` for chaining).
- **Architecture:** Clean Architecture (Contracts -> Implementation).
- **Testing:** E2E-First via `ratatui::backend::TestBackend`.
- **Status (2026-04-25):** Core features implemented. Audit revealed several missing interaction and persistence features required for full spec compliance.
