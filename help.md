# Prompt Quiver Help

## Overview
Prompt Quiver is a TUI-based staging area for AI prompts. It allows you to organize, draft, and quickly copy prompts to your clipboard with automatic processing (snippet expansion and comment stripping).

## Global Controls
- ?: Open/Close this help modal.
- Tab / Shift+Tab: Cycle through tabs.
- 1-6: Jump directly to a tab (Prompts, Canned, Notes, Snippets, Archive, Settings).
- u / Ctrl+y: Undo / Redo list mutations.
- Ctrl+p: Open Project Picker to switch contexts.
- q: Quit application.

## List View Commands
- j / k: Move selection up/down.
- g / G: Jump to top/bottom of the list.
- s: Stage (Process and copy to clipboard).
- y / c: Copy (Process and copy to clipboard).
- e / Enter: Edit selected item.
- a / i: Add new item after/before selection.
- D: Duplicate selected item.
- d: Move to Archive (or delete permanently if in Archive).
- m: Move Mode (Use j/k to reorder, Esc/Enter to finish).
- /: Filter current list.

### Filters
Filters help you manage which prompts are visible based on your current working context.
- b: Branch Filter - Only shows items associated with your current Git branch.
- f: Folder Filter - Only shows items created in your current working directory.
- p: Project Filter - Only shows items associated with the currently active project context (switchable via Ctrl+p).
- Note: Filters only apply to the Prompts, Notes, and Archive tabs. Global tabs like Snippets and Canned are always visible.

### Wide View
- w: Toggle Wide View - Shows a second line below each prompt in the Prompts tab displaying its folder, project, and branch (using the same colors as the statusline). Fields that are currently active as filters are hidden since they are the same for all visible prompts. Can also be toggled in Settings → Advanced.

## Smart Editor
The editor uses conventions and triggers to automate prompt preparation.

### Special Conventions (Inside the Editor)
- Define a Title:
  - Action: Start the first line with -- My Title and leave the second line empty.
  - Outcome: "My Title" appears in the list view but is removed when staging/copying.
- Mark as Draft:
  - Action: Include Draft or [Draft] in your title (e.g., -- My Prompt [Draft]).
  - Outcome: Item is marked as a draft in the list and cannot be staged until the marker is removed.
- Add Comments:
  - Action: Start any line with -- (no leading space).
  - Outcome: These lines are visible in the editor but stripped when staging/copying.

### Autocomplete & Triggers
Type these characters to open suggestions:
- @: Files - Fuzzy search and insert relative file paths.
- $: Snippet Content - Insert the full text of a snippet immediately.
- $$: Snippet Variable - Insert $${name} (expanded only when staging/copying).
- /: Slash Commands - Insert custom commands defined in Settings.
- Controls: Up/Down to select, Enter to confirm, Esc to close suggestions.

### Saving & Navigation
- Ctrl+s: Save and exit.
- Ctrl+g: Save, Stage (copy), and exit in one action.
- Esc: Exit editor (prompts to confirm if changes are unsaved).
- Snippets Tab Only:
  - Tab: Switch focus between the Snippet Name and Content fields.

## Settings
- Use Space or Enter to toggle options or enter sub-menus.
- Edit Slash Commands directly in the list to customize your / autocomplete.
