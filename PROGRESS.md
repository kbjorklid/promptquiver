# Project Progress: Prompt Quiver (Rust/Ratatui)

## 🎯 Current Milestone
- [x] Phase 3: The Main Loop (The "App" Crate)
- [x] Phase 4: Feature Modules (Iterative)
    - [x] Prompt Management (List, Add, Edit, Delete, Stage)
    - [x] Snippets & Autocomplete
    - [x] Git Branch Poller
- [x] Phase 5: Persistence
    - [x] FileSystemStorage implementation
    - [x] Real Git/Clipboard implementation

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
- [x] `RealClipboard` implementation (via `arboard`)
- [x] `RealGit` implementation (via `git rev-parse`)

### 6. Polishing
- [x] Atlas Branding & UI Styling (Centered title, separate tab bar, rounded borders)
- [x] Toast Notifications (via `ratatui-toaster`)
- [x] Git Branch Poller (Tokio background task)

---

## 📓 Technical Notes
- **Shell:** Windows PowerShell (use `;` for chaining).
- **Architecture:** Clean Architecture (Contracts -> Implementation).
- **Testing:** E2E-First via `ratatui::backend::TestBackend`.
- **Status (2026-04-25):** Project substantially complete. All core and infrastructure features implemented, including real-world Git and Clipboard integration, and a polished UI with branding and notifications.
- **Refactoring (Latest):** 
  - Implemented `RealClipboard` and `RealGit` in `infra`.
  - Added Toast notifications and Atlas branding in `ui`.
  - Improved Editor visualization: The editor now takes over the main area with a rounded border and clear title, providing better separation from the list view.
  - All E2E and unit tests passing.
