use crate::types::RenderState;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
pub fn render(f: &mut Frame<'_>, area: Rect, state: &RenderState<'_, '_>) {
    let palette = crate::utils::get_palette(state.settings.theme_name.as_deref());

    let current_path = &state.nav.current_path;
    let formatted_path = crate::utils::format_path(current_path);
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
