use contracts::{Settings, Tab};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Clear};
use ratatui::style::{Style, Color, Modifier};
use ratatui::Frame;
use ratatui::layout::{Rect, Layout, Constraint, Direction};
use ratatui_textarea::TextArea;

pub fn render(
    f: &mut Frame<'_>,
    area: Rect,
    settings: &Settings,
    selected_index: usize,
    textarea: Option<&TextArea<'_>>,
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
    let slash_style = if selected_index == tabs.len() {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Slash Commands ")
        .border_style(slash_style);

    if selected_index == tabs.len() && textarea.is_some() {
        let mut ta = textarea.unwrap().clone();
        ta.set_block(block.title(" Edit Slash Commands (Ctrl+S to save, Esc to cancel) "));
        f.render_widget(Clear, chunks[1]);
        f.render_widget(&ta, chunks[1]);
    } else {
        let slash_cmds = Paragraph::new(settings.slash_commands.join(", "))
            .block(block);
        f.render_widget(slash_cmds, chunks[1]);
    }

    // Claude Commands
    let advanced_style = if selected_index == tabs.len() + 1 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let claude_status = if settings.enable_claude_commands { "[ON]" } else { "[OFF]" };
    let claude_p = Paragraph::new(format!(" Enable Claude Commands: {}", claude_status))
        .block(Block::default().borders(Borders::ALL).title(" Advanced ")
        .border_style(advanced_style));
    f.render_widget(claude_p, chunks[2]);
}
