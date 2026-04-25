use contracts::{Settings, Tab};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::style::{Style, Color, Modifier};
use ratatui::Frame;
use ratatui::layout::{Rect, Layout, Constraint, Direction};

pub fn render(
    f: &mut Frame<'_>,
    area: Rect,
    settings: &Settings,
    selected_index: usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Tab Visibility
            Constraint::Min(5),    // Slash Commands
            Constraint::Length(3),  // Claude Commands
        ])
        .split(area);

    // Tab Visibility
    let tabs = Tab::all();
    let items: Vec<ListItem<'_>> = tabs.iter().enumerate().map(|(i, t)| {
        let is_visible = settings.tab_visibility.get(t).cloned().unwrap_or(true);
        let prefix = if i == selected_index { "> " } else { "  " };
        let status = if is_visible { "[x]" } else { "[ ]" };
        let style = if i == selected_index {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        ListItem::new(format!("{} {} {:?}", prefix, status, t)).style(style)
    }).collect();

    let tab_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Tab Visibility (Space to toggle) "));
    f.render_widget(tab_list, chunks[0]);

    // Slash Commands
    let slash_cmds = Paragraph::new(settings.slash_commands.join(", "))
        .block(Block::default().borders(Borders::ALL).title(" Slash Commands "));
    f.render_widget(slash_cmds, chunks[1]);

    // Claude Commands
    let claude_status = if settings.enable_claude_commands { "[ON]" } else { "[OFF]" };
    let claude_p = Paragraph::new(format!(" Enable Claude Commands: {}", claude_status))
        .block(Block::default().borders(Borders::ALL).title(" Advanced "));
    f.render_widget(claude_p, chunks[2]);
}
