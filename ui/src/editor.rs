use contracts::Prompt;
use ratatui::widgets::{Block, Borders, List, ListItem, Clear};
use ratatui::style::{Style, Color};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui_textarea::TextArea;

pub fn render(
    f: &mut Frame<'_>,
    area: Rect,
    textarea: &TextArea<'_>,
    suggestions: &[Prompt],
    suggestion_index: usize,
) {
    let mut textarea = textarea.clone();
    textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .title(" Edit Prompt (Ctrl+S to save, Esc to cancel) ")
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(Clear, area);
    f.render_widget(&textarea, area);

    // Autocomplete popup
    if !suggestions.is_empty() {
        let popup_area = Rect::new(area.x + 2, area.y + 2, 30, (suggestions.len() as u16 + 2).min(10));
        f.render_widget(Clear, popup_area);
        
        let items: Vec<ListItem<'_>> = suggestions
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let name = s.name.as_deref().unwrap_or(&s.text);
                let style = if i == suggestion_index {
                    Style::default().bg(Color::Yellow).fg(Color::Black)
                } else {
                    Style::default()
                };
                ListItem::new(name).style(style)
            })
            .collect();
        
        let list = List::new(items)
            .block(Block::default().title(" Snippets ").borders(Borders::ALL));
        f.render_widget(list, popup_area);
    }
}
