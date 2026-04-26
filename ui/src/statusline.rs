use ratatui::widgets::Paragraph;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Style, Color};
use ratatui::text::{Line, Span};
use std::path::Path;

pub fn render(
    f: &mut Frame<'_>,
    area: Rect,
    current_path: &str,
    current_branch: Option<&str>,
    prompts_count: usize,
) {
    let formatted_path = format_path(current_path);
    let branch_name = current_branch.unwrap_or("no branch");
    
    let line = Line::from(vec![
        Span::styled(format!(" {} ", formatted_path), Style::default().fg(Color::LightBlue)),
        Span::styled(format!("  {} ", branch_name), Style::default().fg(Color::Indexed(208))), // Orange
        Span::styled(format!(" [{}] Items ", prompts_count), Style::default().fg(Color::Gray)),
    ]);

    let paragraph = Paragraph::new(line).style(Style::default().bg(Color::Indexed(234))); // Dark gray background
    f.render_widget(paragraph, area);
}

fn format_path(path_str: &str) -> String {
    let path = Path::new(path_str);
    let components: Vec<_> = path.components().collect();
    
    if components.len() <= 2 {
        return path_str.replace('\\', "/");
    }

    let last_two: Vec<_> = components.iter().rev().take(2).rev().collect();
    let drive = components.first().and_then(|c| {
        if let std::path::Component::Prefix(p) = c {
            Some(p.as_os_str().to_string_lossy().replace('\\', "/"))
        } else {
            None
        }
    });

    let mut result = String::new();
    if let Some(d) = drive {
        result.push_str(&d);
        result.push_str("/.../");
    } else {
        result.push_str(".../");
    }

    for (i, comp) in last_two.iter().enumerate() {
        result.push_str(&comp.as_os_str().to_string_lossy().replace('\\', "/"));
        if i < last_two.len() - 1 {
            result.push('/');
        }
    }

    result
}
