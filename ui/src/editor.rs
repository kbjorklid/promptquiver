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
        let mut popup_area = area;
        popup_area.x = popup_area.x.saturating_add(2).min(area.right());
        popup_area.y = popup_area.y.saturating_add(2).min(area.bottom());
        popup_area.width = 30.min(area.right().saturating_sub(popup_area.x));
        popup_area.height = ((suggestions.len() as u16 + 2).min(10)).min(area.bottom().saturating_sub(popup_area.y));
        
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
        
        let title = match suggestions[0].r#type {
            contracts::PromptType::Snippet => " Snippets ",
            contracts::PromptType::Note => " Files ",
            contracts::PromptType::Prompt => " Commands ",
        };

        let list = List::new(items)
            .block(Block::default().title(title).borders(Borders::ALL));
        f.render_widget(list, popup_area);
    }
}
