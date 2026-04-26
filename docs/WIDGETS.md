# Ratatui Widget Reference

This document provides a catalog of core and community-contributed widgets available for the Ratatui ecosystem. Use this as a reference when designing and implementing TUI components for Prompt Quiver.

---

## 1. Core Widgets (Built-in)

Maintained by the Ratatui team, these are the fundamental building blocks for most interfaces.

| Widget | Purpose | Best Use Case |
| :--- | :--- | :--- |
| **Block** | Base container with optional borders and titles. | Foundation for almost every UI element; used to group and label content. |
| **Paragraph** | Styled text display with wrapping and alignment. | Static text, logs, or simple read-only descriptions. |
| **List** | Vertical list of items with selection state. | Sidebars, simple menus, or selecting a single item from a collection. |
| **Table** | Grid of rows and columns with scrolling/selection. | Data-heavy views (like process monitors or database viewers). |
| **Gauge / LineGauge** | Visual progress indicator (0–100%). | Progress bars for background tasks or resource levels. |
| **Chart / BarChart** | Data visualization (line, scatter, bar). | Dashboards and performance metrics. |
| **Tabs** | Horizontal navigation bar. | Switching between different views or "pages" in the application. |
| **Scrollbar** | Visual indicator for scroll position. | Complement to `List`, `Table`, or `Paragraph` for large datasets. |
| **Canvas** | Low-level drawing area for arbitrary shapes. | Custom graphics, maps, or specialized visualizations. |
| **Clear** | Clears the specified area. | Essential for rendering **Popups** or Modals over the existing UI. |

---

## 2. Community Widgets (Ecosystem)

These crates extend Ratatui's functionality for more complex interactions or specialized visual needs.

### Text Editing & Input
- **[ratatui-textarea](https://github.com/rhysd/ratatui-textarea)**: Simple multi-line text input. Best for notes or message composition.
- **[edtui](https://github.com/veeso/edtui)**: Vim-inspired modal editor widget.
- **[ratatui-code-editor](https://github.com/skanehira/ratatui-code-editor)**: Adds syntax highlighting (via tree-sitter) to editing.
- **[tui-prompts](https://github.com/veeso/tui-prompts)**: Interactive prompts for simple user queries (Yes/No, text input).

### Navigation & Structure
- **[tui-tree-widget](https://github.com/veeso/tui-tree-widget)**: Hierarchical data display. Ideal for file trees or nested categories.
- **[ratatui-explorer](https://github.com/veeso/ratatui-explorer)**: Ready-to-use file explorer component.
- **[tui-scrollview](https://github.com/veeso/tui-scrollview)**: A container for areas larger than the screen.
- **[tui-menu](https://github.com/veeso/tui-menu)**: Traditional dropdown or context menus.

### Visuals & Feedback
- **[ratatui-image](https://github.com/extrawurst/ratatui-image)**: Renders images using Sixels or Unicode half-blocks.
- **[ratatui-toaster](https://github.com/veeso/ratatui-toaster)**: Engine for temporary "toast" notifications.
- **[tui-skeleton](https://github.com/veeso/tui-skeleton)**: "Shimmering" placeholders for loading states.
- **[throbber-widgets-tui](https://github.com/veeso/throbber-widgets-tui)**: Simple spinners for background activity.
- **[tui-big-text](https://github.com/veeso/tui-big-text)**: Renders large, stylized text using 8x8 fonts.

### Advanced Data Visualization
- **[tui-piechart](https://github.com/veeso/tui-piechart)**: Configurable pie charts.
- **[ratatui-stacked-bar](https://github.com/veeso/ratatui-stacked-bar)**: Area charts for stacked data segments.
- **[tui-nodes](https://github.com/veeso/tui-nodes)**: Visualizing node-based graphs.

---

## 3. Implementation Strategy

When choosing a widget for Prompt Quiver:

1. **Standard First**: Always prefer Core Widgets for stability and performance.
2. **Interactive Editing**: For the snippet editor, `ratatui-textarea` is the recommended starting point unless complex modal behavior is required.
3. **Hierarchy**: Use `tui-tree-widget` for managing prompt collections and snippets if a nested structure is adopted.
4. **Feedback**: Utilize `ratatui-toaster` for non-intrusive save confirmations and errors.
5. **Modals**: Use the `Clear` widget combined with a `Block` and the desired content widget to implement popups for settings or confirmation dialogs.
