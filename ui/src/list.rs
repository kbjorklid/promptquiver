use contracts::{Prompt, Tab};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::style::{Style, Color};
use ratatui::Frame;
use ratatui::layout::Rect;

pub fn render(
    f: &mut Frame<'_>,
    area: Rect,
    active_tab: Tab,
    prompts: &[Prompt],
    selected_index: usize,
) {
    let items: Vec<ListItem<'_>> = prompts
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let prefix = if i == selected_index { "> " } else { "  " };
            let staged_icon = if p.staged { "🎯 " } else { "" };
            let content = p.name.as_deref().unwrap_or(&p.text);
            let first_line = content.lines().next().unwrap_or("");
            
            let style = if i == selected_index {
                Style::default().bg(Color::Indexed(240)).fg(Color::White)
            } else {
                Style::default()
            };

            ListItem::new(format!("{}{}{}", prefix, staged_icon, first_line)).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(format!(" {:?} ", active_tab)));
    
    if prompts.is_empty() {
        let empty_msg = Paragraph::new("No items found.")
            .block(Block::default().borders(Borders::ALL).title(format!(" {:?} ", active_tab)));
        f.render_widget(empty_msg, area);
    } else {
        f.render_widget(list, area);
    }
}
