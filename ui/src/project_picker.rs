use crate::utils::get_palette;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::prelude::Stylize;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem};
use ratatui::Frame;

pub fn render_picker(
    f: &mut Frame<'_>,
    projects: &[contracts::Project],
    state: &mut ratatui::widgets::ListState,
    settings: &contracts::Settings,
    adding_name: Option<&str>,
    project_filter: bool,
    hide_filter: bool,
) {
    let palette = get_palette(settings.theme_name.as_deref());
    let area = crate::utils::centered_rect(60, 40, f.area());

    f.render_widget(Clear, area);

    // Split area into list and footer for toggle
    let mut constraints = if hide_filter {
        vec![Constraint::Min(3)]
    } else {
        vec![Constraint::Min(3), Constraint::Length(1)]
    };
    constraints.push(Constraint::Length(1)); // Shortcut hints

    let chunks =
        Layout::default().direction(Direction::Vertical).constraints(constraints).split(area);

    let mut items = vec![ListItem::new("  Default  ")];
    for p in projects {
        items.push(ListItem::new(format!("  {}  ", p.title)));
    }

    if let Some(name) = adding_name {
        let label = if name.is_empty() { " " } else { name };
        items.push(
            ListItem::new(format!("  [ {label} ]  "))
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        );
    } else {
        items.push(ListItem::new("  [ Add New Project... ]  ").fg(palette.secondary));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Select Project ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(palette.accent))
                .bg(palette.bg),
        )
        .highlight_style(
            Style::default().bg(palette.accent).fg(palette.bg).add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, chunks[0], state);

    let mut current_chunk = 1;
    if !hide_filter {
        let filter_status = if project_filter { "[x]" } else { "[ ]" };
        let footer = ratatui::widgets::Paragraph::new(format!(
            "  {filter_status} Filter by Project (Tab to toggle)  "
        ))
        .style(Style::default().fg(if project_filter { palette.accent } else { palette.muted }))
        .block(Block::default().bg(palette.bg));
        f.render_widget(footer, chunks[current_chunk]);
        current_chunk += 1;
    }

    let hints = ratatui::widgets::Paragraph::new("  (r) Rename  (d/del) Delete  ")
        .style(Style::default().fg(palette.muted))
        .block(Block::default().bg(palette.bg));
    f.render_widget(hints, chunks[current_chunk]);
}
