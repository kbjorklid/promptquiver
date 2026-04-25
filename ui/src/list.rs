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
    mode: &str,
    search_query: &str,
) {
    let title = if search_query.is_empty() {
        format!(" {:?} ", active_tab)
    } else if mode == "Global Search" {
        format!(" {:?} (Global Search: {}) ", active_tab, search_query)
    } else {
        format!(" {:?} (Search: {}) ", active_tab, search_query)
    };

    let list_items: Vec<ListItem<'_>> = prompts
        .iter()
        .enumerate()

        .map(|(i, p)| {
            let prefix = if i == selected_index { "> " } else { "  " };
            let staged_icon = if p.staged { "🎯 " } else { "" };
            
            let display_name = if let Some(ref name) = p.name {
                name.clone()
            } else {
                let (title, _) = contracts::Processor::extract_title(&p.text);
                title.unwrap_or_else(|| p.text.lines().next().unwrap_or("").to_string())
            };
            
            let style = if i == selected_index {
                Style::default().bg(Color::Indexed(240)).fg(Color::White)
            } else {
                Style::default()
            };

            ListItem::new(format!("{}{}{}", prefix, staged_icon, display_name)).style(style)
        })
        .collect();

    let list = List::new(list_items)
        .block(Block::default().borders(Borders::ALL).title(title.clone()));
    
    if prompts.is_empty() {
        let empty_msg = Paragraph::new("No items found.")
            .block(Block::default().borders(Borders::ALL).title(title));
        f.render_widget(empty_msg, area);
    } else {
        f.render_widget(list, area);
    }
}
