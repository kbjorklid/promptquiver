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
            Constraint::Length(8), // Tab Visibility
            Constraint::Min(5),    // Slash Commands
            Constraint::Length(3),  // Advanced
        ])
        .split(area);

    // Tab Visibility
    let tabs = Tab::all();
    let items: Vec<ListItem<'_>> = tabs.iter().enumerate().map(|(i, t)| {
        let is_visible = settings.tab_visibility.get(t).copied().unwrap_or(true);
        let prefix = if i == selected_index { "> " } else { "  " };
        let status = if is_visible { "[x]" } else { "[ ]" };
        let style = if i == selected_index {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        ListItem::new(format!("{prefix} {status} {t:?}")).style(style)
    }).collect();

    let tab_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Tab Visibility (Space to toggle) "));
    f.render_widget(tab_list, chunks[0]);

    // Slash Commands
    let tabs_len = tabs.len();
    let slash_len = settings.slash_commands.len();
    
    let is_slash_focused = selected_index >= tabs_len && selected_index < tabs_len + slash_len + 1;
    let slash_block = Block::default()
        .borders(Borders::ALL)
        .title(" Slash Commands (Enter to edit, d to delete) ")
        .border_style(if is_slash_focused { Style::default().fg(Color::Yellow) } else { Style::default() });

    let mut slash_items: Vec<ListItem<'_>> = settings.slash_commands.iter().enumerate().map(|(i, cmd)| {
        let idx = tabs_len + i;
        let prefix = if idx == selected_index { "> " } else { "  " };
        let style = if idx == selected_index {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        
        if idx == selected_index && textarea.is_some() {
            ListItem::new(format!("{prefix} /")).style(style)
        } else {
            ListItem::new(format!("{prefix} /{cmd}")).style(style)
        }
    }).collect();

    // Add New item
    let add_idx = tabs_len + slash_len;
    let add_prefix = if add_idx == selected_index { "> " } else { "  " };
    let add_style = if add_idx == selected_index {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    if add_idx == selected_index && textarea.is_some() {
        slash_items.push(ListItem::new(format!("{add_prefix} ")).style(add_style));
    } else {
        slash_items.push(ListItem::new(format!("{add_prefix} + Add New Slash Command")).style(add_style));
    }

    let slash_list = List::new(slash_items).block(slash_block);
    f.render_widget(slash_list, chunks[1]);

    // Render TextArea in-line
    if let Some(ta) = textarea {
        if selected_index >= tabs_len && selected_index <= tabs_len + slash_len {
            let offset = (selected_index - tabs_len) as u16;
            let area = Rect {
                x: chunks[1].x + 5,
                y: chunks[1].y + 1 + offset,
                width: chunks[1].width.saturating_sub(7),
                height: 1,
            };
            // Clear the area first to avoid overlap if textarea is smaller than item
            f.render_widget(Clear, area);
            f.render_widget(ta, area);
        }
    }

    // Advanced
    let advanced_idx = tabs_len + slash_len + 1;
    let advanced_style = if selected_index == advanced_idx {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let claude_status = if settings.enable_claude_commands { "[ON]" } else { "[OFF]" };
    let claude_p = Paragraph::new(format!(" Enable Claude Commands: {claude_status}"))
        .block(Block::default().borders(Borders::ALL).title(" Advanced (Space to toggle) ")
        .border_style(advanced_style));
    f.render_widget(claude_p, chunks[2]);
}
