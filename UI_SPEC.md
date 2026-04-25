# Prompt Quiver: UI Specification

This document details the visual representation and layout of the Prompt Quiver TUI application. It serves as a guide for recreating the interface in any TUI framework.

---

## 1. Global Layout

The application uses a three-section vertical layout:
1.  **Header (2-3 lines):** Application branding and navigation tabs.
2.  **Main Area (Flexible):** The active view (List, Editor, Search, or Settings).
3.  **Footer (2 lines):** Keyboard shortcuts and a status bar.

### 1.1 Visual Style
- **Borders:** Use rounded box-drawing characters for modals and the editor. Use simple horizontal lines for list dividers.
- **Selection:** The active item in any list must be clearly highlighted with a background color (e.g., Blue or Gray) and a selection indicator (e.g., `> `).
- **Typography:** Use Bold for headings and active tab names.

### 1.2 Color Palette
- **Standard Prompts:** Yellow.
- **Notes:** Cyan.
- **Named / Comment-Titled Items:** Magenta.
- **Indicators:**
  - `🎯` (Staged): Red.
  - `📋` (Copied): Green.
- **Secondary/Dimmed Text:** Gray.

---

## 2. Global Components

### 2.1 The Header
- **Branding:** Centered title "PROMPT QUIVER". Optional decorative elements (e.g., arrows or lines) are encouraged.
- **Tabs:** A horizontal bar showing all visible tabs.
  - Active tab: White/Bold with an underline or background.
  - Inactive: Gray.
  - Order: `Prompts | Canned | Notes | Snippets | Archive | Settings`.

### 2.2 The Footer
- **Shortcut Legend:** A compact, multi-column list of primary keys (e.g., `j/k Nav`, `e Edit`, `s Stage`, `q Quit`).
- **Status Bar:** A solid-background bar at the bottom containing:
  - **Count:** `[N] Items`
  - **Path:** Current working directory (truncated if necessary).
  - **Branch:** Current Git branch (highlighted if filtering is active).

### 2.3 Notifications (Toasts)
- Temporary messages that appear centered above the footer.
- Style: Rounded border, Yellow text, disappears after 2-3 seconds.

---

## 3. View: List View

- **Rows:** Each item occupies 1-2 lines.
- **Prefix:** Index number followed by the selection indicator and any status icons (`🎯`, `📋`).
- **Content Preview:**
  - If the item has a `name` or **Comment Title**, display it in Magenta.
  - Otherwise, display the first 2 lines of the `text` (truncated with `...`).
- **Dividers:** A subtle horizontal line between each row.

---

## 4. View: Smart Editor

- **Layout:**
  - **Name Field (Snippets Only):** A single-line input at the top.
  - **Content Area:** A large multi-line text area taking up the remaining space.
- **Border Focus:** The border of the active field should change color (e.g., Blue).
- **Autocomplete Window:**
  - Appears near the cursor or at the bottom of the editor when a mention (`@`, `$`, `$$`, `/`) is typed.
  - Displays up to 5 results with the current selection highlighted.

---

## 5. View: Global Search

- **Input:** A prominent search field at the top.
- **Toggle:** A visual badge showing the current filter (`[Prompts]` or `[Notes]`).
- **Results:** A list where each row is prefixed by the project name in brackets (e.g., `[my-project] My prompt text...`).

---

## 6. View: Settings

- **Structure:** A scrollable list of categorized options.
- **Checkboxes:** Use `[x]` (Green) and `[ ]` (Red) for Boolean toggles.
- **Inline Editing:** For Slash Commands, allow editing directly in the list row by replacing the text with an input field.
