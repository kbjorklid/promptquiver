# Project Progress: Prompt Quiver (Rust/Ratatui)

## 🎯 Current Milestone
- [x] Phase 3: The Main Loop (The "App" Crate)
- [x] Phase 4: Feature Modules (Iterative)
    - [x] Prompt Management (List, Add, Edit, Delete, Stage)
    - [x] Snippets & Autocomplete
    - [x] Git Branch Poller
- [ ] Phase 5: Persistence
    - [x] FileSystemStorage implementation
    - [ ] Real Git/Clipboard implementation

---

## 🗺️ Roadmap

### 1. Workspace Setup
- [x] Root `Cargo.toml` (Workspace definition)
- [x] `contracts` crate (Entities & Traits)
- [x] `infra` crate (Crate created)
- [x] `ui` crate (Crate created)
- [x] `app` crate (Crate created)
- [x] Project-wide lint/CI config (Clippy, Rustfmt, Workspace Lints)

### 2. Infrastructure Mocks (E2E Readiness)
- [x] `InMemoryStorage` implementation
- [x] `MockClipboard` implementation
- [x] Basic E2E test harness using `TestBackend`

### 3. The Main Loop (The "App" Crate)
- [x] Terminal initialization (Crossterm / Alternate Screen)
- [x] Basic Event Loop (Input handling)
- [x] Atomic UI Rendering loop
- [x] Automated Test Coverage Setup (cargo-llvm-cov)
- [x] Tab navigation

### 4. Feature Modules (Iterative)
- [x] Prompt Management
    - [x] List rendering
    - [x] Selection/Navigation (j/k)
    - [x] Add prompt (a)
    - [x] Edit prompt (e/Enter)
    - [x] Delete/Archive prompt (d)
    - [x] Staging (s) - includes un-staging and archiving logic
- [x] Editor (Staging area, Snippets)
    - [x] Snippet discovery
    - [x] Textarea integration
    - [x] Autocomplete popup UI
- [x] Comments & Metadata (Comment stripping and Snippet expansion implemented)

### 5. Persistence
- [x] TOML Parser/Serializer (via serde/toml)
- [x] Atomic File I/O (Temp file + Rename)
- [x] `FileSystemStorage` implementation (Project hash-based filenames, OS data directory)

### 6. Polishing
- [ ] Atlas Branding & UI Styling (Basic UI modularized)
- [ ] Toast Notifications
- [x] Git Branch Poller (Tokio background task)

---

## 📓 Technical Notes
- **Shell:** Windows PowerShell (use `;` for chaining).
- **Architecture:** Clean Architecture (Contracts -> Implementation).
- **Testing:** E2E-First via `ratatui::backend::TestBackend`.
- **Status (2026-04-25):** Major milestone reached. All core features (Prompts, Snippets, Autocomplete, Git Polling, TOML Storage) are implemented and verified with 9 E2E tests.
- **Refactoring (Latest):** 
  - Modularized `ui` crate into `header`, `list`, `footer`, `editor`, and `utils`.
  - Implemented `contracts::Processor` for comment stripping (`--`) and snippet expansion (`$$name`).
  - Aligned autocomplete triggers with spec: `@` (files), `$` (insert text), `$$` (insert variable), `/` (commands).
  - Updated `FileSystemStorage` to use OS data directory and `projects/{hash}.toml` layout.
  - All E2E and unit tests passing.
