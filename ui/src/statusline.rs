use ratatui::widgets::Paragraph;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use std::path::Path;

pub fn render(
    f: &mut Frame<'_>,
    area: Rect,
    current_path: &str,
    current_branch: Option<&str>,
    prompts_count: usize,
    folder_filter_enabled: bool,
    settings: &contracts::Settings,
) {
    let palette = crate::utils::get_palette(settings.theme_name.as_deref());
    
    let formatted_path = format_path(current_path);
    let branch_name = current_branch.unwrap_or("no branch");
    
    let filter_mode = if folder_filter_enabled { "GLOBAL" } else { "LOCAL" };
    let filter_style = if folder_filter_enabled {
        Style::default().fg(palette.bg).bg(palette.accent)
    } else {
        Style::default().fg(palette.secondary)
    };

    let line = Line::from(vec![
        Span::styled(format!(" {} ", filter_mode), filter_style),
        Span::styled(format!(" {} ", formatted_path), Style::default().fg(palette.secondary)),
        Span::styled(format!("  {} ", branch_name), Style::default().fg(palette.warning)),
        Span::styled(format!(" [{}] Items ", prompts_count), Style::default().fg(palette.muted)),
    ]);

    let paragraph = Paragraph::new(line).style(Style::default().bg(palette.bg));
    f.render_widget(paragraph, area);
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
