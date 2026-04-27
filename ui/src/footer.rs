use ratatui::widgets::Paragraph;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Span, Line};
use ratatui::style::{Style, Modifier};
use ratatui::prelude::Stylize;
use contracts::Tab;
use crate::shortcuts;

pub fn render(
    f: &mut Frame<'_>,
    area: Rect,
    mode: &str,
    tab: Tab,
    _prompts_len: usize,
    _selected_index: usize,
    has_suggestions: bool,
    settings: &contracts::Settings,
) {
    let palette = crate::utils::get_palette(settings.theme_name.as_deref());
    let tab_name = match tab {
        Tab::Prompts => "Prompts",
        Tab::Canned => "Canned",
        Tab::Notes => "Notes",
        Tab::Snippets => "Snippets",
        Tab::Archive => "Archive",
        Tab::Settings => "Settings",
    };

    let all_shortcuts = shortcuts::get_shortcuts(mode, tab_name, has_suggestions);
    
    let mut lines = Vec::new();
    let mut current_line = Vec::new();
    let mut current_width = 0;
    let max_width = area.width as usize;

    for (i, shortcut) in all_shortcuts.iter().enumerate() {
        let key_span = Span::styled(
            shortcut.key,
            Style::default().fg(palette.accent).add_modifier(Modifier::BOLD),
        );
        let desc_span = Span::styled(
            format!(": {}", shortcut.desc),
            Style::default().fg(palette.fg),
        );
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
