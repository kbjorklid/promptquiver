use contracts::{Prompt, Tab};
use ratatui::widgets::{Block, Borders, List, ListItem, Clear, Scrollbar, ScrollbarOrientation, ScrollbarState};
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

    let editor_area = if is_snippet {
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

        f.render_widget(Clear, area);
        f.render_widget(&title_textarea, chunks[0]);
        chunks[1]
    } else {
        f.render_widget(Clear, area);
        area
    };

    let mut textarea = textarea.clone();
    textarea.set_line_number_style(Style::default().fg(Color::DarkGray));
    textarea.set_cursor_line_style(Style::default().bg(Color::Indexed(236)));
    textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .title(main_title)
            .border_style(if !title_focused || !is_snippet { Style::default().fg(Color::Cyan) } else { Style::default() }),
    );
    f.render_widget(&textarea, editor_area);

    // Render scrollbar for the text area
    let lines_count = textarea.lines().len();
    let cursor_y = textarea.cursor().0;
    
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    let mut scrollbar_state = ScrollbarState::new(lines_count).position(cursor_y);
    f.render_stateful_widget(
        scrollbar,
        editor_area.inner(ratatui::layout::Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut scrollbar_state,
    );

    // Autocomplete popup
    if !suggestions.is_empty() {
        let cursor = textarea.cursor();
        let row = cursor.0;
        let col = cursor.1;
        
        let popup_width = 30;
        let popup_height = (suggestions.len() as u16 + 2).min(10);
        
        // Heuristic: position relative to cursor
        // Note: We don't have access to scroll state from TextArea, 
        // so we constrain it to the editor area to handle scrolled text better.
        let mut cursor_x = editor_area.x.saturating_add(1).saturating_add(col as u16);
        let mut cursor_y = editor_area.y.saturating_add(1).saturating_add(row as u16);
        
        // Constrain to editor content area
        cursor_x = cursor_x.min(editor_area.right().saturating_sub(1));
        cursor_y = cursor_y.min(editor_area.bottom().saturating_sub(2));

        let mut popup_area = Rect {
            x: cursor_x,
            y: cursor_y.saturating_add(1), // Default to below cursor
            width: popup_width,
            height: popup_height,
        };

        // If not enough space below, show above
        if popup_area.y + popup_area.height > f.area().bottom() {
            if cursor_y >= popup_height {
                popup_area.y = cursor_y.saturating_sub(popup_height);
            } else {
                // Constrain to bottom
                popup_area.y = f.area().bottom().saturating_sub(popup_height);
            }
        }

        // Final constraints to stay within frame
        popup_area.x = popup_area.x.min(f.area().right().saturating_sub(popup_width));
        popup_area.y = popup_area.y.min(f.area().bottom().saturating_sub(popup_height));
        popup_area.width = popup_area.width.min(f.area().width);
        popup_area.height = popup_area.height.min(f.area().height);
        
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
