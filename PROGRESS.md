# Project Progress: Prompt Quiver (Rust/Ratatui)

## 🎯 Current Milestone
- [x] Phase 3: The Main Loop (The "App" Crate)
- [x] Phase 4: Feature Modules (Iterative)
    - [x] Prompt Management (List, Add, Edit, Delete, Stage)
    - [x] Snippets & Autocomplete
    - [x] Git Branch Poller
    - [x] Tab navigation (Tab, Arrows, h/l)
    - [x] Tab jump shortcuts (1-6)
    - [ ] Restore from Archive (r) (not-done)
    - [ ] Move mode (m) (not-done)
    - [ ] Branch filter toggle (b) (not-done)
    - [ ] Local search (/) (not-done)
    - [ ] Global search (G) (not-done)
- [x] Editor (Staging area, Snippets)
    - [x] Snippet discovery
    - [x] Textarea integration
    - [x] Autocomplete popup UI
    - [ ] File search mentions (@) (not-done)
    - [ ] Slash command mentions (/) (not-done)
    - [ ] Save & Stage shortcut (Ctrl+g) (not-done)
    - [ ] Close confirmation if modified (Esc) (not-done)
- [x] Comments & Metadata
    - [x] Comment stripping implementation
    - [x] Snippet expansion implementation
    - [ ] Use Display Title in list view (not-done)
- [ ] Undo/Redo System (not-done)
    - [ ] Session-based history stack (u / Ctrl+y) (not-done)

### 5. Persistence
- [x] TOML Parser/Serializer (via serde/toml)
- [x] Atomic File I/O (Temp file + Rename)
- [x] `FileSystemStorage` implementation
    - [x] Project hash-based filenames
    - [x] OS data directory integration
    - [ ] `[info]` section in project files (not-done)
    - [ ] `[settings]` in `common.toml` (not-done)
- [x] `RealClipboard` implementation (via `arboard`)
- [x] `RealGit` implementation (via `git rev-parse`)

### 6. Polishing
- [x] Atlas Branding & UI Styling (Centered title, separate tab bar, rounded borders)
- [x] Toast Notifications (via `ratatui-toaster`)
- [x] Git Branch Poller (Tokio background task)
- [ ] New items branch assignment (not-done)

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
