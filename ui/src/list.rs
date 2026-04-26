use contracts::{Prompt, Tab};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::style::{Style, Color, Modifier};
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
    settings: &contracts::Settings,
    list_state: &mut ratatui::widgets::ListState,
) {
    let title = if search_query.is_empty() {
        format!(" {active_tab:?} ")
    } else if mode == "Global Search" {
        format!(" {active_tab:?} (Global Search: {search_query}) ")
    } else {
        format!(" {active_tab:?} (Search: {search_query}) ")
    };

    let list_items: Vec<ListItem<'_>> = prompts
        .iter()
        .enumerate()

        .map(|(i, p)| {
            let prefix = if i == selected_index {
                if mode == "Move" {
                    if settings.use_nerd_font { "󰹹 " } else { "↕ " }
                } else {
                    "> "
                }
            } else {
                "  "
            };
            let staged_icon = if p.staged {
                if settings.use_nerd_font { "󰓎 " } else { "🎯 " }
            } else {
                ""
            };
            let copy_icon = if p.last_copied && !p.staged {
                if settings.use_nerd_font { "󰆏 " } else { "📋 " }
            } else {
                ""
            };
            
            let display_name = p.name.as_ref().map_or_else(
                || {
                    let (title, _) = contracts::Processor::extract_title(&p.text);
                    title.unwrap_or_else(|| p.text.lines().next().unwrap_or("").to_string())
                },
                std::clone::Clone::clone,
            );
            
            let style = if i == selected_index {
                if mode == "Move" {
                    Style::default().bg(Color::Indexed(236)).fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().bg(Color::Indexed(240)).fg(Color::White).add_modifier(Modifier::BOLD)
                }
            } else if i % 2 == 0 {
                Style::default().bg(Color::Indexed(234)).fg(Color::Gray)
            } else {
                Style::default().fg(Color::Gray)
            };

            ListItem::new(format!("{prefix}{staged_icon}{copy_icon}{display_name}")).style(style)
        })
        .collect();

    let list = List::new(list_items)
        .block(Block::default().borders(Borders::ALL).title(title.clone()));
    
    if prompts.is_empty() {
        let block = Block::default().borders(Borders::ALL).title(title);
        let msg = format!("\n\n\n\n       ╭─────────────────────────╮\n       │   No items found here   │\n       │    Press 'a' to add     │\n       ╰─────────────────────────╯");
        let empty_msg = Paragraph::new(msg)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(ratatui::layout::Alignment::Center)
            .block(block);
        f.render_widget(empty_msg, area);
    } else {
        f.render_stateful_widget(list, area, list_state);

        // Render scrollbar
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        
        let mut scrollbar_state = ScrollbarState::new(prompts.len())
            .position(selected_index);
            
        f.render_stateful_widget(
            scrollbar,
            area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

pub fn render_preview(
    f: &mut Frame<'_>,
    area: Rect,
    prompt: Option<&Prompt>,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Preview ");

    if let Some(prompt) = prompt {
        let lines = crate::utils::highlight_text(&prompt.text);
        let paragraph = Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
    } else {
        let empty = Paragraph::new("No selection").block(block);
        f.render_widget(empty, area);
    }
}
