# Prompt Quiver: E2E Test Scenarios

This document defines the primary user journeys that must be verified using the `TestBackend` in the Rust re-implementation. These scenarios serve as the "Source of Truth" for the application's integrated behavior.

---

## 1. Staging & Clipboard Flow

### Scenario: Staging an Item
- **Initial State:** Two prompts in the `main` list (A and B). None are staged.
- **Actions:**
  1. Select Prompt A.
  2. Press `s`.
- **Expected Result:**
  - Prompt A is marked with `🎯`.
  - Terminal status bar shows "Staged to clipboard".
  - Mock Clipboard contains the processed text of A.

### Scenario: Staging Displaces Existing Stage
- **Initial State:** Prompt A is staged (`🎯`). Prompt B is in the `main` list.
- **Actions:**
  1. Select Prompt B.
  2. Press `s`.
- **Expected Result:**
  - Prompt B is marked with `🎯`.
  - Prompt A is **moved to the top of the Archive** and is no longer in the `main` list.
  - Prompt A's `staged` flag is `false`.

---

## 2. Editor & Autocomplete

### Scenario: Snippet Expansion via Autocomplete
- **Initial State:** A snippet named `header` exists with text "--- LOG ---". User is in the editor.
- **Actions:**
  1. Type `$$`.
  2. Autocomplete window appears; select `header` and press `Enter`.
  3. Press `Ctrl+s` to save.
- **Expected Result:**
  - The prompt text contains `$$header`.
  - When copying this prompt in the list view (press `y`), the Mock Clipboard contains "--- LOG ---".

### Scenario: File Mention Insertion
- **Initial State:** A file named `src/main.rs` exists. User is in the editor.
- **Actions:**
  1. Type `@`.
  2. Autocomplete window appears; select `src/main.rs` and press `Enter`.
- **Expected Result:**
  - The editor text now contains `@src/main.rs `.

---

## 3. List Management & Undo

### Scenario: Delete and Restore
- **Initial State:** One prompt in the `main` list.
- **Actions:**
  1. Press `d` (Archive).
  2. Switch to `Archive` tab (`l` or `Tab`).
  3. Press `r` (Restore).
- **Expected Result:**
  - The item is removed from the `main` list after step 1.
  - The item is back in the `main` list after step 3.

### Scenario: Undo/Redo Chain
- **Initial State:** Empty `main` list.
- **Actions:**
  1. Press `a`, type "Test", `Ctrl+s`.
  2. Press `d` (Archive).
  3. Press `u` (Undo).
  4. Press `u` (Undo).
- **Expected Result:**
  - After step 2: List is empty.
  - After step 3: "Test" is back in the list.
  - After step 4: List is empty again.

---

## 4. Search & Filtering

### Scenario: Global Search Copy-on-Save
- **Initial State:** A prompt exists in a *different* project TOML file.
- **Actions:**
  1. Press `G` (Global Search).
  2. Type search query and select the result.
  3. Edit view opens; press `Ctrl+s`.
- **Expected Result:**
  - A new item with a new UUID is created in the **current** project's list.
  - The original prompt in the other project remains unchanged.

### Scenario: Branch Filtering
- **Initial State:** 
  - Prompt A (Branch: `main`)
  - Prompt B (Branch: `feature`)
  - Current Git Branch: `main`
- **Actions:**
  1. Press `b` to enable filtering.
- **Expected Result:**
  - Only Prompt A is visible in the list.
  - Status bar indicates "Branch: main (Filtering)".
