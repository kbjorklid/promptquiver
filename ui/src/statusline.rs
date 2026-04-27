use ratatui::widgets::Paragraph;
use ratatui::Frame;
use ratatui::layout::{Rect, Layout, Direction, Constraint};
use ratatui::style::{Style, Modifier};
use ratatui::text::{Line, Span};
use std::path::Path;
use throbber_widgets_tui::{Throbber, ThrobberState};

pub fn render(
    f: &mut Frame<'_>,
    area: Rect,
    current_path: &str,
    current_branch: Option<&str>,
    prompts_count: usize,
    throbber_state: &mut ThrobberState,
    settings: &contracts::Settings,
) {
    let palette = crate::utils::get_palette(settings.theme_name.as_deref());
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(10),
            Constraint::Length(1),
        ])
        .split(area);

    let formatted_path = format_path(current_path);
    let branch_name = current_branch.unwrap_or("no branch");
    
    let line = Line::from(vec![
        Span::styled(format!(" {} ", formatted_path), Style::default().fg(palette.secondary)),
        Span::styled(format!("  {} ", branch_name), Style::default().fg(palette.warning)),
        Span::styled(format!(" [{}] Items ", prompts_count), Style::default().fg(palette.muted)),
    ]);

    let paragraph = Paragraph::new(line).style(Style::default().bg(palette.bg));
    f.render_widget(paragraph, chunks[0]);

    let throbber = Throbber::default()
        .throbber_set(throbber_widgets_tui::BRAILLE_SIX)
        .style(Style::default().fg(palette.info).add_modifier(Modifier::BOLD));
    f.render_stateful_widget(throbber, chunks[1], throbber_state);
}

fn format_path(path_str: &str) -> String {
    let path = Path::new(path_str);
    let normal_components: Vec<_> = path
        .components()
        .filter(|c| matches!(c, std::path::Component::Normal(_)))
        .collect();

    if normal_components.len() <= 2 {
        return path_str.replace('\\', "/");
    }

    let last_two = &normal_components[normal_components.len() - 2..];
    let mut result = String::from(".../");
    for (i, comp) in last_two.iter().enumerate() {
        result.push_str(&comp.as_os_str().to_string_lossy().replace('\\', "/"));
        if i < 1 {
            result.push('/');
        }
    }

    result
}
