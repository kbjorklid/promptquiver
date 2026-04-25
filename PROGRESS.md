# Project Progress: Prompt Quiver (Rust/Ratatui)

## 🎯 Current Milestone
- [ ] Phase 1: Workspace Setup & Core Contracts

---

## 🗺️ Roadmap

### 1. Workspace Setup
- [ ] Root `Cargo.toml` (Workspace definition)
- [ ] `contracts` crate (Entities & Traits)
- [ ] `infra` crate (Mock implementations)
- [ ] `ui` crate (Modular TEA setup)
- [ ] `app` crate (Main entry point & CLI setup)
- [ ] Project-wide lint/CI config

### 2. Infrastructure Mocks (E2E Readiness)
- [ ] `InMemoryStorage` implementation
- [ ] `MockClipboard` implementation
- [ ] Basic E2E test harness using `TestBackend`

### 3. The Main Loop (The "App" Crate)
- [ ] Terminal initialization (Crossterm / Alternate Screen)
- [ ] Basic Event Loop (Input handling)
- [ ] Atomic UI Rendering loop

### 4. Feature Modules (Iterative)
- [ ] Prompt Management (List, Add, Delete)
- [ ] Editor (Staging area, Snippets)
- [ ] Comments & Metadata

### 5. Persistence
- [ ] TOML Parser/Serializer
- [ ] Atomic File I/O
- [ ] `FileSystemStorage` implementation

### 6. Polishing
- [ ] Atlas Branding & UI Styling
- [ ] Toast Notifications
- [ ] Git Branch Poller (Tokio background task)

---

## 📓 Technical Notes
- **Shell:** Windows PowerShell (use `;` for chaining).
- **Architecture:** Clean Architecture (Contracts -> Implementation).
- **Testing:** E2E-First via `ratatui::backend::TestBackend`.
