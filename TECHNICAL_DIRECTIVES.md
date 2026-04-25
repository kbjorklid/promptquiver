# Prompt Quiver: Technical Directives

This document outlines the architectural and implementation requirements for the Rust/Ratatui re-implementation. It follows the principles of **Clean Architecture** and a **Modular Monolith** using a **Cargo Workspace**.

---

## 1. Project Structure (The Workspace)

The project is organized as a Cargo Workspace to enforce hard boundaries between modules and prevent circular dependencies.

### 1.1 Crate Layout
- `contracts/`: **The Core Domain.** Shared entities (Prompt, Note), Error types, and Trait definitions (Storage, Clipboard).
- `infra/`: **The Infrastructure.** Concrete implementations of traits (File system, Git, Clipboard).
- `ui/`: **The Interface.** All Ratatui logic, organized as internal modules (e.g., `mod editor`, `mod search`).
- `app/`: **The Orchestrator.** Main entry point, event loop, and dependency injection.

---

## 2. Clean Architecture Implementation

Each module must adhere to internal layering:
1.  **Domain:** Pure structs and logic (e.g., the algorithm for stripping comments). No I/O.
2.  **Application:** Use cases and logic that coordinates domain entities.
3.  **Infrastructure:** Concrete implementations of traits defined in `contracts`.
4.  **UI (Interface):** Ratatui components and the `View` part of the Model-View-Update pattern.

### 2.1 Dependency Inversion (The "Contracts" Pattern)
Modules should not depend on each other. Instead:
- If `Search` needs to tell `PromptManagement` to create a new prompt, it defines a requirement in the `contracts` crate via a Trait.
- The `app` crate instantiates the concrete implementations and "plugs" them together at startup.

---

## 3. The TUI Pattern: Modular TEA

The application follows **The Elm Architecture (TEA)**:
- **Model:** Each module maintains its own local state.
- **Update:** Each module has an `update` function that receives `Events` and returns a `Command` (a message for the orchestrator to perform an action or talk to another module).
- **View:** Each module implements a `render` function taking a Ratatui `Frame`.

---

## 4. Technical Requirements

### 4.1 Concurrency & I/O
- **Non-Blocking UI:** The main loop must stay responsive. Use `tokio` for background tasks.
- **Communication:** Use `tokio::sync::mpsc` channels to send messages from background threads (Git branch polling, Global Search file walking) to the UI.
- **Atomic Writes:** Persistence must use the "Write-to-Temp-then-Rename" pattern to prevent data corruption.

### 4.2 Error Handling & Safety
- **Panic Recovery:** Implement a custom panic hook to ensure `disable_raw_mode()` and `show_cursor()` are called if the app crashes.
- **Result Type:** Use `thiserror` for domain-specific errors and `anyhow` for top-level application errors.

### 4.3 Recommended Crates
- **UI:** `ratatui`, `crossterm`.
- **Widgets:** `ratatui-textarea`, `ratatui-toaster`, `tui-popup`.
- **Data:** `serde`, `toml`, `uuid`.
- **System:** `arboard` (Clipboard), `directories` (Standard OS paths).
- **Async:** `tokio`.

---

## 5. Development & Testing Workflow

The project follows a strict **Test-Driven / Refactor-Always** development cycle.

1.  **E2E-Driven Implementation:** Every feature or bug fix must begin with a comprehensive integration test using the `TestBackend`. The implementation is incomplete until this test passes and covers all specified edge cases.
2.  **Infrastructure Mocking:** Prioritize implementing robust `InMemory` versions of the `contracts` traits to enable these tests.
3.  **Mandatory Refactoring Phase:** After a feature is implemented and tests pass, a dedicated refactoring round is REQUIRED. Focus on:
    - Aligning code with Clean Architecture layers.
    - Removing duplication and improving naming.
    - Ensuring the "Modular Monolith" boundaries are respected.
    - Optimizing for Rust idiomaticity (e.g., proper use of `Option`, `Result`, and Ownership).
4.  **Surgical Unit Testing:** Use unit tests for complex, state-independent algorithms where E2E tests are too coarse.
5.  **Validation:** DECLARE SUCCESS ONLY after the full E2E suite passes and the code has been refactored for long-term maintainability.

---

## 6. End-to-End (E2E) Testing Strategy

The application must be architected to support full integration tests that run without side effects.

### 6.1 Virtual Terminal Testing
- **Backend:** Use `ratatui::backend::TestBackend` for all E2E scenarios.
- **Input Simulation:** Drive the application by sending a stream of `crossterm::event::Event` (KeyEvents) to the main event handler.
- **State Verification:** Assert against the `TestBackend` buffer. Verify string content, text colors, and cursor positions.

### 6.2 Deterministic Mocking
- **Infrastructure:** All E2E tests must use "Mock" versions of the traits defined in `contracts` (e.g., `InMemoryStorage`, `MockClipboard`).
- **Isolation:** Tests must not touch the actual filesystem or system clipboard.
- **Snapshot Testing:** Use a tool like the `insta` crate to perform "Golden Master" testing on the terminal UI state to catch regressions in layout or styling.
