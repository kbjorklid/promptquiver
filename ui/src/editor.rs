use contracts::{Prompt, Tab};
use ratatui::widgets::{Block, Borders, List, ListItem, Clear};
use ratatui::style::{Style, Color};
use ratatui::Frame;
use ratatui::layout::{Rect, Layout, Constraint, Direction};
use ratatui_textarea::TextArea;

pub fn render(
    f: &mut Frame<'_>,
    area: Rect,
    textarea: &TextArea<'_>,
    title_textarea: &TextArea<'_>,
    title_focused: bool,
    active_tab: Tab,
    suggestions: &[Prompt],
    suggestion_index: usize,
) {
    let is_snippet = active_tab == Tab::Snippets;
    
    let main_title = if is_snippet {
        " Edit Snippet (Tab to switch, Ctrl+S to save, Esc to cancel) "
    } else {
        " Edit Prompt (Ctrl+S to save, Esc to cancel) "
    };

    let mut textarea = textarea.clone();
    textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .title(main_title)
            .border_style(if !title_focused || !is_snippet { Style::default().fg(Color::Cyan) } else { Style::default() }),
    );

    f.render_widget(Clear, area);

    if is_snippet {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title field
                Constraint::Min(3),    // Content field
            ])
            .split(area);

        let mut title_textarea = title_textarea.clone();
        title_textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .title(" Snippet Name ([a-zA-Z0-9_-]+) ")
                .border_style(if title_focused { Style::default().fg(Color::Cyan) } else { Style::default() }),
        );

        f.render_widget(&title_textarea, chunks[0]);
        f.render_widget(&textarea, chunks[1]);
    } else {
        f.render_widget(&textarea, area);
    }

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
