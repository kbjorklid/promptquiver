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

pub fn get_zebra_color(color: Color) -> Color {
    match color {
        Color::Rgb(r, g, b) => {
            let luminance = 0.2126 * f32::from(r) + 0.7152 * f32::from(g) + 0.0722 * f32::from(b);
            if luminance < 40.0 {
                // Very dark - lighten more
                Color::Rgb(r.saturating_add(15), g.saturating_add(15), b.saturating_add(15))
            } else if luminance < 128.0 {
                // Dark - lighten slightly
                Color::Rgb(r.saturating_add(10), g.saturating_add(10), b.saturating_add(10))
            } else {
                // Light - darken slightly
                Color::Rgb(r.saturating_sub(10), g.saturating_sub(10), b.saturating_sub(10))
            }
        }
        _ => color,
    }
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

pub fn highlight_text<'a>(text: &'a str, palette: &ThemePalette) -> Vec<Line<'a>> {
    text.lines()
        .map(|line| {
            if line.starts_with("--") {
                // Title or Comment
                Line::from(Span::styled(line, Style::default().fg(palette.muted)))
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
                        Style::default().fg(palette.warning),
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn test_get_palette() {
        let p = get_palette(Some("Dracula"));
        assert_eq!(p.accent, Color::Rgb(189, 147, 249)); // Dracula accent

        let p_default = get_palette(None);
        assert_eq!(p_default.accent, Color::Rgb(189, 147, 249)); // Default is Dracula
    }

    #[test]
    fn test_get_zebra_color() {
        let dark = Color::Rgb(10, 10, 10);
        let lightened = get_zebra_color(dark);
        assert!(matches!(lightened, Color::Rgb(r, _g, _b) if r > 10));

        let mid = Color::Rgb(100, 100, 100);
        let mid_lightened = get_zebra_color(mid);
        assert!(matches!(mid_lightened, Color::Rgb(r, _g, _b) if r > 100));

        let bright = Color::Rgb(200, 200, 200);
        let darkened = get_zebra_color(bright);
        assert!(matches!(darkened, Color::Rgb(r, _g, _b) if r < 200));

        assert_eq!(get_zebra_color(Color::Red), Color::Red);
    }

    #[test]
    fn test_centered_rect() {
        let r = Rect::new(0, 0, 100, 100);
        let centered = centered_rect(50, 50, r);
        assert_eq!(centered.width, 50);
        assert_eq!(centered.height, 50);
        assert_eq!(centered.x, 25);
        assert_eq!(centered.y, 25);
    }

    #[test]
    fn test_highlight_text() {
        let palette = get_palette(None);
        let lines = highlight_text("-- comment\nHello $$snippet world", &palette);
        assert_eq!(lines.len(), 2);
        
        // Line 1: comment
        assert_eq!(lines[0].spans.len(), 1);
        
        // Line 2: snippet
        assert_eq!(lines[1].spans.len(), 3); // "Hello ", "$$snippet", " world"
        assert_eq!(lines[1].spans[1].content, "$$snippet");
    }
}
