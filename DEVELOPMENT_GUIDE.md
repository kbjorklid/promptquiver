# Prompt Quiver: Development Guide

This guide provides practical implementation details to ensure a high-quality terminal experience and a productive development environment for the Rust/Ratatui re-implementation.

---

## 1. Terminal Lifecycle & Safety

To ensure the user's terminal isn't "broken" after the app exits or crashes, follow these lifecycle rules:

### 1.1 The "Alternate Screen"
- **Startup:** Always enter the "Alternate Screen" and enable "Raw Mode." This prevents the TUI from cluttering the user's terminal scrollback.
- **Shutdown:** Always exit the Alternate Screen and disable Raw Mode on exit.

### 1.2 Panic Hook
- **Implementation:** Implement a custom panic hook that restores the terminal state (disables raw mode, exits alternate screen, shows cursor) *before* printing the panic message.
- **Crate:** `std::panic::set_hook`.

---

## 2. Debugging & Logging

In a TUI application, `println!` or `dbg!` will corrupt the rendered UI. 

### 2.1 File-Based Logging
- **Strategy:** Use the `tracing` or `log` crate to write logs to a file.
- **Location:** Write logs to a local file (e.g., `debug.log`) in the application's data directory.
- **Workflow:** Developers should use `tail -f debug.log` in a separate terminal window to monitor the application state in real-time.

---

## 3. External Integrations

### 3.1 Git Operations
The app should execute these specific commands via `std::process::Command`:
- **Current Branch:** `git rev-parse --abbrev-ref HEAD` (Run every 10 seconds in a background thread).
- **Project Root:** `git rev-parse --show-toplevel` (Used to calculate the project hash).
- **Ignore Check:** For file autocomplete, use the `ignore` crate (which respects `.gitignore`) rather than calling Git directly for every file.

### 3.2 System Clipboard
- **Requirement:** Support cross-platform clipboard operations without external dependencies where possible.
- **Crate:** `arboard` is the recommended standard for Rust.

---

## 4. Command Line Interface (CLI)

The application should have a minimal CLI for flexibility.

### 4.1 Basic Arguments
- **`--path <DIR>`**: Open Prompt Quiver for a specific directory instead of the current working directory.
- **`--log-level <LEVEL>`**: Set the verbosity of the `debug.log` (e.g., `info`, `debug`, `trace`).

---

## 5. Directory Management

- **Data Persistence:** Use the `directories` crate to find the correct OS-specific paths for data storage.
  - **Windows:** `%APPDATA%\promptquiver`
  - **Linux:** `~/.local/share/promptquiver`
  - **macOS:** `~/Library/Application Support/promptquiver`
- **Atomic Renames:** When saving TOML files, always write to `{filename}.tmp` first and then use `std::fs::rename` to overwrite the original.
