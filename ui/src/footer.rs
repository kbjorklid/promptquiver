use crate::shortcuts;
use crate::types::RenderState;
use contracts::Tab;
use ratatui::layout::Rect;
use ratatui::prelude::Stylize;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn render(f: &mut Frame<'_>, area: Rect, state: &RenderState<'_, '_>) {
    let palette = crate::utils::get_palette(state.settings.theme_name.as_deref());
    let tab_name = match state.nav.active_tab {
        Tab::Prompts => "Prompts",
        Tab::Canned => "Canned",
        Tab::Notes => "Notes",
        Tab::Snippets => "Snippets",
        Tab::Archive => "Archive",
        Tab::Settings => "Settings",
    };

    let mode_str = match state.mode {
        crate::types::Mode::List => "List",
        crate::types::Mode::Editor => "Editor",
        crate::types::Mode::Move => "Move",
        crate::types::Mode::Search => "Search",
        crate::types::Mode::ConfirmDiscard => "Confirm Discard",
        crate::types::Mode::ThemePicker => "Theme Picker",
        crate::types::Mode::ProjectPicker => "Project Picker",
        crate::types::Mode::AddProject => "Add Project",
        crate::types::Mode::RenameProject => "Rename Project",
        crate::types::Mode::ExportDialog => "Export Data",
        crate::types::Mode::ImportDialog => "Import Data",
        crate::types::Mode::MetadataEditor => "Metadata Editor",
    };

    let has_suggestions = !state.editor.autocomplete.suggestions.is_empty();
    let all_shortcuts = shortcuts::get_shortcuts(mode_str, tab_name, has_suggestions);

    let mut lines = Vec::new();
    let mut current_line = Vec::new();
    let mut current_width = 0;
    let max_width = area.width as usize;

    for (i, shortcut) in all_shortcuts.iter().enumerate() {
        let key_span = Span::styled(
            shortcut.key,
            Style::default().fg(palette.accent).add_modifier(Modifier::BOLD),
        );
        let desc_span =
            Span::styled(format!(": {}", shortcut.desc), Style::default().fg(palette.fg));
        let separator = if i < all_shortcuts.len() - 1 { " | " } else { "" };
        let sep_span = Span::styled(separator, Style::default().fg(palette.muted));

        let shortcut_width = shortcut.key.len() + 2 + shortcut.desc.len() + separator.len();

        if current_width + shortcut_width > max_width && !current_line.is_empty() {
            lines.push(Line::from(current_line));
            current_line = Vec::new();
            current_width = 0;
        }

        current_line.push(key_span);
        current_line.push(desc_span);
        if !separator.is_empty() {
            current_line.push(sep_span);
        }
        current_width += shortcut_width;

        if lines.len() >= 2 {
            break; // Max 2 lines
        }
    }

    if !current_line.is_empty() && lines.len() < 2 {
        lines.push(Line::from(current_line));
    }

    let footer = Paragraph::new(lines).bg(palette.bg);
    f.render_widget(footer, area);
}
