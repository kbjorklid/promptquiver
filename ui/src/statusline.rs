use ratatui::widgets::Paragraph;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use std::path::Path;
use crate::types::RenderState;

pub fn render(
    f: &mut Frame<'_>,
    area: Rect,
    state: &RenderState<'_, '_>,
) {
    let palette = crate::utils::get_palette(state.settings.theme_name.as_deref());
    
    let current_path = &state.nav.current_path;
    let formatted_path = format_path(current_path);
    let branch_name = state.current_branch.unwrap_or("no branch");
    
    let active_project_title = state.nav.projects_manager.active_project_id.and_then(|id| {
        state.nav.projects_manager.projects.iter().find(|p| p.id == id).map(|p| p.title.as_str())
    });
    let project_name = active_project_title.unwrap_or("Default");
    let prompts_count = state.nav.prompts.len();
    
    let path_style = if state.nav.folder_filter {
        Style::default().fg(palette.bg).bg(palette.secondary)
    } else {
        Style::default().fg(palette.secondary)
    };

    let project_style = if state.nav.project_filter {
        Style::default().fg(palette.bg).bg(palette.accent)
    } else {
        Style::default().fg(palette.accent)
    };

    let branch_style = if state.nav.branch_filter {
        Style::default().fg(palette.bg).bg(palette.warning)
    } else {
        Style::default().fg(palette.warning)
    };

    let line = Line::from(vec![
        Span::styled(format!(" {formatted_path} "), path_style),
        Span::styled(format!("  {project_name} "), project_style),
        Span::styled(format!("  {branch_name} "), branch_style),
        Span::styled(format!(" [{prompts_count}] Items "), Style::default().fg(palette.muted)),
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
