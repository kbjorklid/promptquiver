use ratatui::layout::{Layout, Constraint, Direction, Rect};
use ratatui::text::{Line, Span};
use ratatui::style::{Style, Color};
use ratatui_themes::{Theme, ThemeName, ThemePalette};

pub fn get_palette(theme_name: Option<&str>) -> ThemePalette {
    let name = theme_name.and_then(|n| {
        ThemeName::all().iter().find(|t| format!("{:?}", t) == n)
    }).copied().unwrap_or(ThemeName::Dracula);
    
    Theme::new(name).palette()
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn highlight_text(text: &str) -> Vec<Line<'_>> {
    text.lines()
        .map(|line| {
            if line.starts_with("--") {
                // Title or Comment
                Line::from(Span::styled(line, Style::default().fg(Color::DarkGray)))
            } else {
                let mut spans = Vec::new();
                let mut last_pos = 0;
                
                // Find snippets $$name
                let mut current_pos = 0;
                while let Some(pos) = line[current_pos..].find("$$") {
                    let absolute_pos = current_pos + pos;
                    
                    // Push text before snippet
                    if absolute_pos > last_pos {
                        spans.push(Span::raw(&line[last_pos..absolute_pos]));
                    }
                    
                    // Find end of snippet name (alphanumeric, _, -)
                    let start_name = absolute_pos + 2;
                    let mut end_name = start_name;
                    if start_name <= line.len() {
                        for c in line[start_name..].chars() {
                            if c.is_alphanumeric() || c == '_' || c == '-' {
                                end_name += c.len_utf8();
                            } else {
                                break;
                            }
                        }
                    }
                    
                    // Highlight snippet
                    spans.push(Span::styled(
                        &line[absolute_pos..end_name],
                        Style::default().fg(Color::Yellow),
                    ));
                    
                    last_pos = end_name;
                    current_pos = end_name;
                }
                
                // Push remaining text
                if last_pos < line.len() {
                    spans.push(Span::raw(&line[last_pos..]));
                }
                
                Line::from(spans)
            }
        })
        .collect()
}
