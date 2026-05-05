# Claude Code Custom Commands Discovery

This document outlines how **Prompt Quiver** discovers custom slash commands from **Claude Code** to provide autocomplete and prefilling within the TUI.

## 1. Why Integrate?
Prompt Quiver serves as a staging area for AI prompts. Since Claude Code users often define custom, reusable commands (slash commands), Prompt Quiver should:
- **Command Prefilling**: Automatically suggest known Claude slash commands when the user starts typing `/` in the editor.
- **Project Awareness**: Respect the same project-level vs. global-level command scoping as Claude Code to ensure relevant suggestions.

## 2. Discovery Mechanism

### Command Locations
Claude Code looks for Markdown files (`*.md`) in three specific hierarchies:

| Scope | Path Template | Priority |
| :--- | :--- | :--- |
| **Project** | `<project_root>/.claude/commands/*.md` | **High** (Overrides Global) |
| **Global** | `~/.claude/commands/*.md` | **Medium** |
| **Marketplace** | `~/.claude/plugins/cache/*/commands/*.md` | **Low** (Plugin-provided) |

### Root Discovery Logic
To find the `<project_root>`, Prompt Quiver follows Claude's heuristic:
1. Start at the current working directory (`cwd`) of the active project in Prompt Quiver.
2. Search upwards for:
   - A `.git/` folder.
   - A `CLAUDE.md` file.
   - A `package.json` file.
3. If found, the directory containing these markers is the Project Root.

### Marketplace Plugin Cache
Plugin commands are located in the global Claude cache. On Windows, this typically maps to `%USERPROFILE%\.claude\plugins\cache\`. Prompt Quiver should scan all subdirectories within this cache for a `commands/` folder.

## 3. Command Metadata for UI

### File Format
Commands are Markdown files where the filename (slugified) is the command name.
Example: `.claude/commands/refactor.md` -> `/refactor`

### Frontmatter Extraction
Prompt Quiver parses the YAML frontmatter to enhance the autocomplete UI:
```markdown
---
description: Refactor the selected code for clarity and DRY principles.
---
```
- **Description**: Displayed alongside the command in the autocomplete suggestion list to help the user choose the right command.

## 4. Implementation Details

### Background Polling
The `infra` layer should implement a `ClaudeCommandScanner` that:
- Watches the global and local directories for changes using `notify` (cross-platform).
- Debounces filesystem events to prevent excessive re-parsing.
- Provides a list of available command names and descriptions to the `EditorModule`.

### Autocomplete Trigger
- When the user types `/` at the start of a line or after a space in the Prompt Quiver editor:
  1. The editor fetches the current list of Claude commands.
  2. A popup/suggestion list appears with matching command names and their descriptions.
  3. Selecting a command inserts the full command slug (e.g., `/refactor `) into the editor.

## 5. References
- [Claude Code Channels Documentation](https://code.claude.com/docs/en/channels)
