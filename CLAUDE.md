# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Approach

Use **red-green refactoring** whenever possible: write a failing test first, make it pass with minimal code, then clean up.

## Project Overview

**Prompt Quiver** is a TUI (Terminal User Interface) application written in Rust that serves as a staging area for AI prompts. It allows developers and AI users to organize, draft, search, and quickly copy prompts to their clipboard with automatic processing (snippet expansion and comment stripping).

The application supports multi-project workspaces, Git branch awareness, file-based filtering, customizable themes, and snippet management. It stores data in an SQLite database and automatically detects Claude commands from `.gemini/` directories.

## Architecture

Prompt Quiver uses a **modular, layered architecture** with clear separation of concerns:

### Crate Structure (Rust Workspace)

1. **contracts** (data layer & traits)
   - Pure data types: `Prompt`, `Project`, `Settings`, `Tab`, `PromptFilter`
   - Abstract traits: `Storage`, `Clipboard`, `Git`, `AppService`, `Processor`
   - Domain errors and result types
   - No external I/O or dependencies (except serialization)
   - Location: `contracts/src/`

2. **infra** (infrastructure & implementations)
   - Concrete implementations of traits: `SqliteStorage`, `RealClipboard`, `RealGit`, `RealAppService`
   - SQLite database layer with schema management
   - File system integration for prompt discovery and search
   - Clipboard I/O using `arboard`
   - Git integration using system `git` command
   - Mocks for testing: `InMemoryStorage`, `MockClipboard`, `MockGit`
   - Location: `infra/src/`

3. **ui** (rendering & state management)
   - Ratatui TUI components: list, editor, settings, project picker, modals
   - Application state machines: `Mode` (List, Editor, Move, Search, etc.), `AppMessage` (user events)
   - Update logic: `EditorModule` and `ListModule` handle state transitions per mode
   - Render functions that transform state into terminal frames
   - Location: `ui/src/`

4. **promptquiver** (application orchestration)
   - Main entry point: `main.rs` wires up infra and spawns async tasks (git polling, file searching, DB sync)
   - App struct: holds all layers and routing logic
   - Event handlers: translate user input into messages
   - Main loop: coordinates rendering, event handling, and async updates
   - Location: `promptquiver/src/`

### Data Flow

```
User Input (Crossterm Events)
  ↓
handlers::handle_events() → Vec<AppMessage>
  ↓
App::handle_message() → routes to EditorModule or ListModule.update()
  ↓
Update logic modifies editor/list state and may call infra layer (Storage, Clipboard, Git)
  ↓
Main loop calls ui::render() with updated state
  ↓
Frame rendered to terminal via Ratatui
```

### Key Design Patterns

- **Trait-based abstraction**: All I/O operations (storage, clipboard, git) are behind traits, enabling mocking for tests
- **Async-first**: Uses Tokio for background tasks (git branch polling, file search, DB synchronization)
- **Message-driven updates**: User actions converted to `AppMessage` enums, processed by state machines
- **Modular UI**: Rendering is split into independent functions per component (header, list, editor, settings, etc.)
- **Layered testing**: Unit tests in `infra` verify storage and service logic; integration tests in `promptquiver/tests/` verify workflows

## Building & Running

### Build (Release)
```bash
cargo build --release
```
Output: `target/release/promptquiver` (or `.exe` on Windows)

### Run (Debug)
```bash
cargo run --release
```

### Install
- **macOS/Linux**: `./install.sh` → `~/.local/bin/quiver`
- **Windows**: `./install.ps1` → `%USERPROFILE%\.local\bin\quiver.exe`

## Testing

### Run All Tests
```bash
cargo test --all-features
```

### Run Integration Tests Only
```bash
cargo test --test '*' --release
```
Located in `promptquiver/tests/` with names like `autocomplete.rs`, `editing.rs`, `navigation.rs`, etc.

### Run Tests for a Specific Crate
```bash
cargo test -p contracts
cargo test -p infra
cargo test -p promptquiver
```

### Run a Single Test
```bash
cargo test --test editing -- --exact test_name --nocapture
```

### Generate Coverage Report
```bash
cargo tarpaulin --all --timeout 300 --out Html
```
(Requires `cargo install cargo-tarpaulin`)

## Code Quality

### Formatting
```bash
cargo fmt --all
```
Config: `.rustfmt.toml` (Windows line endings, max heuristics)

### Linting
```bash
cargo clippy --all --all-targets --all-features -- -D warnings
```
Workspace lints are configured in root `Cargo.toml`:
- Deny unsafe code
- Warn on missing debug implementations and Rust idioms
- Warn on Clippy pedantic and nursery rules

## Project Structure

### Key Files & Modules

- `main.rs`: Application entry point, CLI args parsing, async task setup
- `app.rs`: `App` struct (state container), `handle_message()` message router
- `handlers.rs`: Event-to-message conversion, input processing, character batching optimization
- `ui/lib.rs`: Render function dispatchers and layout splitting
- `ui/list_module.rs`: List view state machine (filtering, selection, movement, staging)
- `ui/editor_module.rs`: Editor state machine (text input, autocomplete, title parsing)
- `ui/settings.rs`: Settings UI and toggle logic
- `infra/storage/sqlite.rs`: Database schema and SQL queries
- `infra/service.rs`: Business logic (staging, archiving, file search)
- `contracts/processor.rs`: Snippet expansion and comment stripping

### Database

SQLite database at `~/.local/share/promptquiver/promptquiver.db` (or OS-specific data dir).

**Tables:**
- `prompts` (id, text, type, folder, project_id, branch, name, staged, last_copied, is_archived, created_at, updated_at, order_index)
- `projects` (id, title, created_at)
- `project_info` (folder, path)
- `settings` (key, value JSON)
- `_migrations` (version)

**Migration pattern:** `infra/src/storage/sqlite.rs` checks migration version on startup.

## Async & Concurrency

Main loop spawns three background tasks:

1. **Git poller** (`setup_git_poller`): Polls current branch every 5s, sends via channel
2. **File searcher** (`setup_file_searcher`): Listens for autocomplete `@` queries, searches filesystem in background
3. **DB sync poller** (`setup_db_poller`): Checks data version every 500ms to detect external changes

All three communicate back to main loop via `tokio::sync::mpsc` channels. Main loop checks channels with `try_recv()` to avoid blocking.

## Important Implementation Details

### Prompt Processing
- **Snippet expansion** (`$variable` → snippet text): Happens only during copy/stage, not on display
- **Comment stripping** (lines starting with `--`): Removed during copy/stage
- **Title extraction** (first line `-- My Title` with blank second line): Removed from display but used as UI label
- **Draft marking** (title contains `[Draft]`): Prevents staging until marker is removed

### Filtering & Sorting
- **Branch filter**: Only shows prompts from current Git branch
- **Folder filter**: Only shows prompts created in current working directory
- **Project filter**: Only shows prompts associated with selected project
- **Filters don't apply to**: Snippets and Canned tabs (globally visible)
- **Ordering**: Persistent `order_index` field, editable in Move mode

### Autocomplete Triggers
- `@` → File path suggestions (fuzzy-matched, directories preferred)
- `$` → Full snippet text insertion
- `$$` → Snippet variable placeholder (`$${name}`)
- `/` → Slash commands (customizable in Settings)

### Claude Commands Discovery
If `enable_claude_commands` is set, the app scans `.gemini/commands/` in the project root for YAML files and imports them as prompts in the Canned tab.

## Git Workflow

The codebase uses conventional commits. Recent major features:
- v0.8.0: Claude commands discovery, export/import, help modal
- v0.7.0: SQLite migration, multi-instance support, file watching
- v0.6.0: Theme picker, draft prompts, settings persistence

Tag releases with `v*` to trigger CI builds for all platforms (Linux, macOS x86/ARM, Windows).

## CI/CD

GitHub Actions workflow (`.github/workflows/ci.yml`):
1. **Test**: Runs `cargo test --all-features` on Ubuntu
2. **Build**: Cross-platform builds (Linux, Windows, macOS x86/ARM)
3. **Release**: Uploads artifacts on tag push (v*)

Artifacts are published as GitHub Releases.

## Common Development Tasks

### Add a New Tab
1. Add variant to `contracts::Tab` enum
2. Create rendering function in `ui/` (e.g., `ui/my_tab.rs`)
3. Update `ui::render()` to dispatch to your function
4. Add to `ListModule::update()` for any list-based filtering
5. Store data in database via `Storage` trait

### Add a New Filter
1. Add field to `contracts::PromptFilter`
2. Update `SqliteStorage::get_prompts()` to apply the filter
3. Add toggle message to `AppMessage`
4. Update `ListModule::update()` to handle toggle
5. Update UI to display filter status in statusline

### Add a New Autocomplete Trigger
1. Update `EditorModule::handle_autocomplete_query()` to recognize trigger character
2. Send message to spawn search task (file search uses channel in `main.rs`)
3. Receive results and update `editor.autocomplete.suggestions`
4. Render suggestions in `ui/editor.rs`

### Modify Storage Schema
1. Create new migration in `infra/src/storage/sqlite.rs` (increment version)
2. Update `Prompt` struct if needed in `contracts`
3. Update SQL queries in `SqliteStorage` methods
4. Add tests in `infra/tests/` or integration tests

### Add Tests
- **Unit tests**: Add `#[cfg(test)] mod tests` blocks in the same file
- **Integration tests**: Create new file in `promptquiver/tests/`
- Use `infra::InMemoryStorage` and `infra::MockClipboard` for isolation
- See existing tests for patterns (e.g., `tests/editing.rs`, `tests/autocomplete.rs`)

## Debugging

### Logging
No structured logging; use `eprintln!()` for debugging (visible in terminal if not capturing stdout).

### Common Issues
- **"Database is locked"**: Multiple instances writing simultaneously. Mitigated by polling version rather than watching.
- **Autocomplete hangs**: File search is blocking; large directories can be slow. Cap results at 1000 in `service.rs`.
- **Paste batching**: `handlers.rs` batches rapid character inputs to detect simulated pastes; if pasting behaves oddly, check the batching logic.

## Dependencies & Ecosystem

- **TUI framework**: Ratatui 0.30 with ecosystem widgets (textarea, toaster, themes, popups, markdown)
- **Async runtime**: Tokio 1.52
- **Database**: Rusqlite with SQLite bundled
- **Serialization**: Serde + TOML/JSON
- **Text processing**: Fuzzy-matcher for autocomplete, regex for parsing
- **System integration**: Arboard (clipboard), Notify (file watching), Crossterm (terminal backend)

Workspace dependencies are managed in root `Cargo.toml` for consistency.
