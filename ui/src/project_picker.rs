use ratatui::widgets::{List, ListItem, Block, Borders, Clear};
use ratatui::Frame;
use ratatui::layout::{Rect, Layout, Constraint, Direction};
use ratatui::style::{Style, Modifier};
use ratatui::prelude::Stylize;
use crate::utils::get_palette;

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
    let area = centered_rect(60, 40, f.area());
    
    f.render_widget(Clear, area);

    // Split area into list and footer for toggle
    let constraints = if hide_filter {
        vec![Constraint::Min(3)]
    } else {
        vec![Constraint::Min(3), Constraint::Length(1)]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);
    
    let mut items = vec![ListItem::new("  Default  ")];
    for p in projects {
        items.push(ListItem::new(format!("  {}  ", p.title)));
    }
    
    if let Some(name) = adding_name {
        items.push(ListItem::new(format!("  [ New: {} ]  ", name))
            .fg(palette.accent)
            .add_modifier(Modifier::BOLD));
    } else {
        items.push(ListItem::new("  [ Add New Project... ]  ").fg(palette.secondary));
    }

    let list = List::new(items)
        .block(Block::default()
            .title(" Select Project ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(palette.accent))
            .bg(palette.bg))
        .highlight_style(Style::default()
            .bg(palette.accent)
            .fg(palette.bg)
            .add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, chunks[0], state);

    if !hide_filter {
        let filter_status = if project_filter { "[x]" } else { "[ ]" };
        let footer = ratatui::widgets::Paragraph::new(format!("  {filter_status} Filter by Project (Tab to toggle)  "))
            .style(Style::default().fg(if project_filter { palette.accent } else { palette.muted }))
            .block(Block::default().bg(palette.bg));
        f.render_widget(footer, chunks[1]);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
