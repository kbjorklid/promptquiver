use contracts::{Prompt, Tab};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Clear};
use ratatui::layout::{Layout, Constraint, Direction, Rect};
use ratatui::Frame;
use ratatui::style::{Style, Color, Modifier};
use ratatui_textarea::TextArea;

pub fn render(
    f: &mut Frame<'_>,
    active_tab: Tab,
    prompts: &[Prompt],
    selected_index: usize,
    mode: &str,
    textarea: &TextArea<'_>,
    current_branch: Option<&str>,
    suggestions: &[Prompt],
    suggestion_index: usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header/Tabs
            Constraint::Min(0),    // Main area
            Constraint::Length(3), // Footer
        ])
        .split(f.area());
    
    // ... (same header logic)
    let tab_titles = Tab::all().iter().map(|t| format!("{:?}", t)).collect::<Vec<_>>();
    let branch_str = current_branch.map(|b| format!(" [b: {}] ", b)).unwrap_or_default();
    let title = format!(" PROMPT QUIVER {} ", branch_str);
    
    let tabs = Tabs::new(tab_titles)
        .block(Block::default().title(title).borders(Borders::ALL))
        .select(Tab::all().iter().position(|&t| t == active_tab).unwrap_or(0))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    
    f.render_widget(tabs, chunks[0]);

    // Main area - Render list of prompts
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
        f.render_widget(empty_msg, chunks[1]);
    } else {
        f.render_widget(list, chunks[1]);
    }

    // Footer
    let footer_text = if mode == "Editor" {
        if !suggestions.is_empty() {
            " Up/Down: Select | Enter: Complete | Esc: Close ".to_string()
        } else {
            " Ctrl+s: Save | Esc: Cancel ".to_string()
        }
    } else {
        format!(
            " q: Quit | Tab: Next Tab | j/k: Nav | s: Stage | a: Add | e: Edit | Index: {}/{} ",
            if prompts.is_empty() { 0 } else { selected_index + 1 },
            prompts.len()
        )
    };
    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);

    // Editor Overlay
    if mode == "Editor" {
        let area = centered_rect(80, 80, f.area());
        f.render_widget(Clear, area);
        f.render_widget(textarea, area);

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
}


fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}



