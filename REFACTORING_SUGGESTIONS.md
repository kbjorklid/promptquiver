# Refactoring Suggestions - Prompt Quiver

This document outlines identified architectural and code-quality improvements for the Prompt Quiver Rust/Ratatui re-implementation.

## 1. Modularize Infrastructure Layer
**Target:** `infra/src/lib.rs` (God File)
**Issue:** The file contains multiple unrelated implementations (Sqlite, InMemory, Git, Clipboard) and has grown to over 800 lines.
**Action:**
- Split into `infra/src/storage/mod.rs` (with `memory.rs` and `sqlite.rs`).
- Create `infra/src/git/mod.rs` (with `real.rs` and `mock.rs`).
- Create `infra/src/clipboard/mod.rs`.
- Use `pub use` in `infra/src/lib.rs` to maintain public API compatibility.

## 2. Decompose `ListModule` Responsibilities
**Target:** `ui/src/list_module.rs`
**Issue:** Handles prompt listing, Undo/Redo stacks, project management, and search filtering.
**Action:**
- Extract `HistoryManager`: Move `undo_stack`, `redo_stack`, and associated methods to a dedicated struct.
- Extract `ProjectManager`: Move `projects`, `new_project_name`, and `selecting_startup_project` logic.
- Simplify `ListModule` to focus purely on the "List View" state and filtering.

## 3. Decouple Event Handling from Hardcoded Keys
**Target:** `promptquiver/src/handlers.rs`
**Issue:** Key bindings like `KeyCode::Char('q')` are hardcoded in match arms, making them difficult to customize and potentially inconsistent with `ui/src/shortcuts.rs`.
**Action:**
- Map `KeyEvent` to `Shortcut` types defined in `ui`.
- Use the `Shortcut` enum or registry to drive the match logic in `handlers.rs`.
- This ensures the UI hints (footer) and actual logic are always in sync.

## 4. Decouple `App::handle_message` Orchestration
**Target:** `promptquiver/src/app.rs`
**Issue:** The main message loop mixes mode transition logic, state updates, and global side effects (like saving to disk).
**Action:**
- Implement a "Mode Transition Map" or state machine to handle `Mode` changes based on messages.
- Separate "Command Generation" (deciding what to do) from "State Mutation" (actually updating `ListModule` or `EditorModule`).

## 5. Standardize Async Traits
**Target:** `contracts/src/lib.rs`
**Issue:** Some traits use `#[async_trait]` while others might not, leading to inconsistency.
**Action:**
- Audit all traits in `contracts` and ensure consistent use of `async_trait` where I/O is involved.
- Consider using "Static Dispatch" where performance is critical, but prioritize the current "Contracts" pattern for testability.

## 6. Improve `EditorModule` Content Management
**Target:** `ui/src/editor_module.rs`
**Issue:** Complex logic for handling autocomplete and editor state synchronization.
**Action:**
- Encapsulate `Autocomplete` logic further to reduce the footprint in the main `update` loop.
- Ensure clear separation between the "Title" buffer and the "Body" buffer management.
