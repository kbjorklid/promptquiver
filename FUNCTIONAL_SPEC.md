# Prompt Quiver: Functional Specification

This document provides a technology-agnostic blueprint of the Prompt Quiver application. It describes all functional behaviors, data relationships, and user interactions required to recreate the application.

---

## 1. Core Philosophy
Prompt Quiver is a "staging area" for AI interactions. It is designed to be a low-latency, keyboard-driven tool that lives in the terminal, allowing users to queue and organize prompts while an AI is busy.

### Key Goals
- **Context-Aware:** Prompts are stored per-project based on the directory they are accessed from.
- **Global Knowledge:** Snippets, Canned Prompts, and Settings are available across all projects.
- **Rapid Throughput:** Moving a prompt from "idea" to "clipboard" should take minimum keystrokes.
- **Non-Destructive:** Heavy use of undo/redo and an archive instead of immediate deletion.

---

## 2. Data Model

### 2.1 The Prompt Object
The core unit of data.
- `id`: UUID (v4).
- `text`: Multi-line string (the actual content).
- `type`: One of `prompt` or `note`.
- `branch`: (Optional) The Git branch name associated with the prompt.
- `name`: (Optional) A display name. **Mandatory for Snippets.**
- `staged`: Boolean (indicates if the item is "active" in the staging area).
- `created_at`: ISO-8601 timestamp.
- `updated_at`: ISO-8601 timestamp.

### 2.2 Storage Structure (Simplified)
Storage uses **TOML** format for all files.

- **Base Directory:** Standard OS data directory (e.g., `AppData/Roaming/promptquiver` on Windows, `~/.local/share/promptquiver` on Linux).
- **Project Storage:** 
  - **Filename:** `projects/{hash}.toml` where `{hash}` is the first 8 characters of the SHA-256 hash of the project's absolute path.
  - **Contents:**
    - `[info]`: Contains `path` (the absolute path this file belongs to).
    - `[[main]]`: Array of Prompt objects.
    - `[[notes]]`: Array of Prompt objects (type: note).
    - `[[archive]]`: Array of Prompt objects.
- **Global Storage:**
  - **Filename:** `common.toml`.
  - **Contents:**
    - `[[canned]]`: Reusable prompt templates.
    - `[[snippets]]`: Reusable text fragments.
    - `[settings]`: User preferences.

### 2.3 Settings Schema
- `tab_visibility`: Map of tab names to Booleans.
- `slash_commands`: List of custom strings (e.g., `["/fix", "/explain"]`).
- `enable_claude_commands`: Boolean. If true, includes pre-defined AI commands in autocomplete.

---

## 3. Logic & Algorithms

### 3.1 Comment Processing
Processing occurs when copying or staging an item.
1.  **Extract Title:** If the first line starts with `--` AND the second line is empty:
    - The text after `--` on the first line (trimmed) is the **Display Title**.
    - Both lines are removed during processing.
2.  **Strip Comments:** Remove all lines that start exactly with `--` (no leading whitespace).

### 3.2 Snippet Expansion
Occurs when copying or staging an item (except when copying from the Snippets tab).
- **Pattern:** Find all occurrences of `$$[a-zA-Z0-9_-]+`.
- **Replacement:** If a snippet with a matching `name` exists, replace the variable with the snippet's full `text`. Otherwise, leave as-is.

### 3.3 The Staging State Machine
The "Stage" action (`s`) is the central workflow:
1.  **Identify Target:** The selected item in any tab (except Archive/Settings).
2.  **Transition:**
    - If item is being **Staged** (staged: false -> true):
      - Find ALL other items in `main`, `notes`, `canned`, and `snippets` that have `staged: true`.
      - For each found item (except if it's the target):
        - Set `staged: false`.
        - If the item is in `main`, `notes`, or `snippets`, **move it to the top of the Archive**.
        - (Canned prompts never move to Archive).
      - Set target item `staged: true`.
    - If item is being **Un-staged** (staged: true -> false):
      - Set target item `staged: false`. (Do not move to Archive yet; that is a separate 'Delete/Archive' action `d`).
3.  **Action:** The processed text (comments stripped, snippets expanded) is copied to the system clipboard.

### 3.4 Editor Mentions (Autocomplete)
Triggered in the editor when a line starts with or a space is followed by:
- `@`: Fuzzy search of files in the current project (respecting `.gitignore`). Inserts the relative path.
- `$`: Fuzzy search of Snippet names. Inserts the **entire text** of the snippet immediately.
- `$$`: Fuzzy search of Snippet names. Inserts the **variable reference** (e.g., `$$name`) for later expansion.
- `/`: Fuzzy search of Slash Commands (custom + Claude commands). Inserts the command.

---

## 4. Feature Catalog

### 4.1 Navigation
- **Tabs:** `Prompts`, `Canned`, `Notes`, `Snippets`, `Archive`, `Settings`.
- **Branch Filter:** Toggleable mode (`b`) that hides items not matching the current Git branch. Items with no branch are always visible.
- **Search:** 
  - **Local:** Filter the current list view.
  - **Global:** Search all project TOML files for a substring. Selecting a result creates a copy in the current project.

### 4.2 Undo/Redo
- A session-based history stack (not persisted to disk) that tracks all list mutations (add, delete, reorder, update, stage).

### 4.3 Git Integration
- Automatically detects the current Git branch.
- Assigns the current branch to new items created via Add, Paste, or Global Search.

---

## 5. Interaction Map (Keybindings)

### Global
- `Tab / Shift+Tab`: Cycle tabs.
- `1-6`: Jump to tab.
- `q`: Quit.
- `u / Ctrl+y`: Undo / Redo.

### List View
- `j / k`: Move selection.
- `Enter / e`: Edit.
- `a / i`: Add after / before.
- `y / c`: Copy processed text.
- `s`: Stage / Un-stage.
- `d`: Move to Archive (or delete permanently if already in Archive).
- `r`: Restore from Archive to original list.
- `m`: Move mode (reorder with `j/k`).
- `/`: Local search.
- `G`: Global search.
- `b`: Toggle branch filter.

### Editor
- `Ctrl+s`: Save and exit.
- `Ctrl+g`: Save, Stage, and exit.
- `Esc`: Close (confirm if modified).
- `Up / Down`: Navigate autocomplete if open.
