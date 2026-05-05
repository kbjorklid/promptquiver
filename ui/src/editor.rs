use contracts::Tab;
use ratatui::widgets::{Block, Borders, List, ListItem, Clear, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::style::Style;
use ratatui::Frame;
use ratatui::layout::{Rect, Layout, Constraint, Direction};
use crate::types::RenderState;

pub fn render(
    f: &mut Frame<'_>,
    area: Rect,
    state: &mut RenderState<'_, '_>,
) -> Rect {
    let settings = state.settings;
    let palette = crate::utils::get_palette(settings.theme_name.as_deref());
    let active_tab = state.nav.active_tab;
    let is_snippet = active_tab == Tab::Snippets;
    let title_focused = state.editor.title_focused;
    
    let textarea = &mut state.editor.textarea;
    let title_textarea = &mut state.editor.title_textarea;
    
    let verb = if state.editor.editing_id.is_some() { "Edit" } else { "Create" };
    
    let noun = match active_tab {
        Tab::Snippets => "Snippet",
        Tab::Notes => "Note",
        Tab::Archive => "Archived Item",
        Tab::Settings => "Slash Command",
        _ => "Prompt",
    };

    let main_title = if is_snippet {
        format!(" {verb} {noun} (Tab to switch, Ctrl+S to save, Esc to cancel) ")
    } else {
        format!(" {verb} {noun} (Ctrl+S to save, Esc to cancel) ")
    };

    let editor_area = if is_snippet {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title field
                Constraint::Min(3),    // Content field
            ])
            .split(area);

        title_textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .title(" Snippet Name ([a-zA-Z0-9_-]+) ")
                .border_style(if title_focused { Style::default().fg(palette.accent) } else { Style::default().fg(palette.fg) }),
        );
        title_textarea.set_style(Style::default().bg(palette.bg).fg(palette.fg));

        f.render_widget(Clear, area);
        f.render_widget(&*title_textarea, chunks[0]);
        chunks[1]
    } else {
        f.render_widget(Clear, area);
        area
    };

    textarea.set_wrap_mode(ratatui_textarea::WrapMode::WordOrGlyph);
    textarea.set_line_number_style(Style::default().fg(palette.muted));
    textarea.set_cursor_line_style(Style::default());
    textarea.set_style(Style::default().bg(palette.bg).fg(palette.fg));
    textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .title(main_title)
            .border_style(if !title_focused || !is_snippet { Style::default().fg(palette.accent) } else { Style::default().fg(palette.fg) }),
    );
    f.render_widget(&*textarea, editor_area);

    // Render scrollbar for the text area
    let lines_count = textarea.lines().len();
    let cursor_y = textarea.cursor().0;
    
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"))
        .style(Style::default().fg(palette.fg));
    let mut scrollbar_state = ScrollbarState::new(lines_count).position(cursor_y);
    f.render_stateful_widget(
        scrollbar,
        editor_area.inner(ratatui::layout::Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut scrollbar_state,
    );
    
    editor_area
}

pub fn render_autocomplete(
    f: &mut Frame<'_>,
    editor_area: Rect,
    state: &mut RenderState<'_, '_>,
) {
    let settings = state.settings;
    let palette = crate::utils::get_palette(settings.theme_name.as_deref());
    let textarea = &state.editor.textarea;
    let suggestions = &state.editor.autocomplete.suggestions;
    let suggestion_index = state.editor.autocomplete.index;
    let autocomplete_open = state.editor.autocomplete.open;
    let autocomplete_list_state = &mut state.editor.autocomplete.list_state;
    
    if autocomplete_open && !suggestions.is_empty() {
        let screen_cursor = textarea.screen_cursor();
        let col = screen_cursor.col;
        let row = screen_cursor.row;
        
        let popup_width = 80;
        let popup_height_pref = (u16::try_from(suggestions.len()).unwrap_or(u16::MAX).saturating_add(2)).min(10);
        
        // Absolute screen coordinates of the cursor
        let cursor_x = editor_area.x.saturating_add(1).saturating_add(u16::try_from(col).unwrap_or(u16::MAX));
        let cursor_y = editor_area.y.saturating_add(1).saturating_add(u16::try_from(row).unwrap_or(u16::MAX));
        
        // Define safe screen limits
        let top_limit = f.area().top().saturating_add(1);       // Row after header
        let bottom_limit = f.area().bottom().saturating_sub(3); // Row before footer (2) and statusline (1)
        
        let mut popup_area = Rect {
            x: cursor_x.min(f.area().right().saturating_sub(popup_width)),
            y: 0,
            width: popup_width.min(f.area().width),
            height: popup_height_pref,
        };

        let space_below = bottom_limit.saturating_sub(cursor_y.saturating_add(1));
        let space_above = cursor_y.saturating_sub(top_limit);

        // Positioning strategy: 
        // 1. Try below if it fits perfectly.
        // 2. Otherwise try above if it fits perfectly.
        // 3. Otherwise use the side with more room and shrink.
        if space_below >= popup_height_pref {
            popup_area.y = cursor_y.saturating_add(1);
        } else if space_above >= popup_height_pref {
            popup_area.y = cursor_y.saturating_sub(popup_height_pref);
        } else if space_below >= space_above && space_below >= 3 {
            popup_area.y = cursor_y.saturating_add(1);
            popup_area.height = space_below;
        } else if space_above >= 3 {
            popup_area.height = space_above.min(popup_height_pref);
            popup_area.y = cursor_y.saturating_sub(popup_area.height);
        } else {
            // Emergency fallback: just show it below and hope for the best
            popup_area.y = cursor_y.saturating_add(1);
            popup_area.height = space_below.max(1);
        }

        // Final safety constraints
        popup_area.y = popup_area.y.max(top_limit).min(f.area().bottom().saturating_sub(1));
        if popup_area.bottom() > bottom_limit {
            popup_area.height = bottom_limit.saturating_sub(popup_area.y);
        }

        f.render_widget(Clear, popup_area);
        
        let items: Vec<ListItem<'_>> = suggestions
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let name = s.name.as_deref().unwrap_or(&s.text);
                let display_text = if s.r#type == contracts::PromptType::Prompt && !s.text.is_empty() && s.text != *name {
                    format!("{name} - {}", s.text)
                } else {
                    name.to_string()
                };
                
                let style = if i == suggestion_index {
                    Style::default().bg(palette.accent).fg(palette.bg)
                } else {
                    Style::default().fg(palette.fg)
                };
                ListItem::new(display_text).style(style)
            })
            .collect();
        
        let title = match suggestions[0].r#type {
            contracts::PromptType::Snippet => " Snippets ",
            contracts::PromptType::Note => " Files ",
            contracts::PromptType::Prompt => " Commands ",
        };

        let list = List::new(items)
            .style(Style::default().bg(palette.bg))
            .block(Block::default().title(title).borders(Borders::ALL).border_style(Style::default().fg(palette.accent)));
        
        autocomplete_list_state.select(Some(suggestion_index));
        f.render_stateful_widget(list, popup_area, autocomplete_list_state);
    }
}
