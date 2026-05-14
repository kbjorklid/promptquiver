use crate::types::RenderState;
use crate::utils::{get_palette, get_zebra_color};
use contracts::{Prompt, Tab};
use ratatui::layout::Rect;
use ratatui::prelude::Stylize;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{
    Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use ratatui::Frame;

#[allow(clippy::too_many_lines)]
pub fn render(f: &mut Frame<'_>, area: Rect, state: &mut RenderState<'_, '_>) {
    let settings = state.settings;
    let palette = get_palette(settings.theme_name.as_deref());
    let zebra_bg = get_zebra_color(palette.bg);

    let active_tab = state.nav.active_tab;
    let search_query = &state.nav.search_query;
    let prompts = &state.nav.prompts;
    let selected_index = state.nav.selected_index;
    let mode_str = match state.mode {
        crate::types::Mode::Move => "Move",
        crate::types::Mode::Search => "Search",
        _ => "List",
    };

    let title = if search_query.is_empty() {
        format!(" {active_tab:?} ")
    } else {
        format!(" {active_tab:?} (Search: {search_query}) ")
    };

    let list_items: Vec<ListItem<'_>> = prompts
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let prefix = if i == selected_index {
                if mode_str == "Move" {
                    if settings.use_nerd_font {
                        "󰹹 "
                    } else {
                        "↕ "
                    }
                } else {
                    "> "
                }
            } else {
                "  "
            };
            let staged_icon = if p.staged && active_tab != Tab::Notes && active_tab != Tab::Snippets
            {
                if settings.use_nerd_font {
                    "󰓎 "
                } else {
                    "🎯 "
                }
            } else {
                ""
            };
            let copy_icon = if p.last_copied && !p.staged {
                if settings.use_nerd_font {
                    "󰆏 "
                } else {
                    "📋 "
                }
            } else {
                ""
            };

            let (display_name, is_draft) =
                if active_tab == Tab::Prompts || active_tab == Tab::Canned {
                    contracts::Processor::get_display_title(&p.text)
                } else {
                    let name = p.name.as_ref().map_or_else(
                        || {
                            let (title, _) = contracts::Processor::extract_title(&p.text);
                            title.unwrap_or_else(|| p.text.lines().next().unwrap_or("").to_string())
                        },
                        std::clone::Clone::clone,
                    );
                    (name, false)
                };

            let mut style = if i == selected_index {
                Style::default().bg(palette.accent).fg(palette.bg).add_modifier(Modifier::BOLD)
            } else if i % 2 == 0 {
                Style::default().bg(zebra_bg).fg(palette.fg)
            } else {
                Style::default().bg(palette.bg).fg(palette.fg)
            };

            if is_draft {
                style = style.add_modifier(Modifier::DIM);
            }

            let ai_indicator = if state
                .ai_pending_titles
                .is_some_and(|s| s.contains(&p.id))
            {
                " …"
            } else {
                ""
            };
            ListItem::new(format!(
                "{prefix}{staged_icon}{copy_icon}{display_name}{ai_indicator}"
            ))
            .style(style)
        })
        .collect();

    let list = List::new(list_items)
        .block(Block::default().borders(Borders::ALL).title(title.clone()))
        .style(Style::default().bg(palette.bg).fg(palette.fg));

    if prompts.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .style(Style::default().bg(palette.bg).fg(palette.fg));
        let msg = "\n\n\n\n       ╭─────────────────────────╮\n       │   No items found here   │\n       │    Press 'a' to add     │\n       ╰─────────────────────────╯".to_string();
        let empty_msg = Paragraph::new(msg)
            .style(Style::default().fg(palette.muted))
            .alignment(ratatui::layout::Alignment::Center)
            .block(block);
        f.render_widget(empty_msg, area);
    } else {
        f.render_stateful_widget(list, area, &mut state.nav.list_state);

        // Render scrollbar
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .style(Style::default().fg(palette.fg));

        let mut scrollbar_state = ScrollbarState::new(prompts.len()).position(selected_index);

        f.render_stateful_widget(
            scrollbar,
            area.inner(ratatui::layout::Margin { vertical: 1, horizontal: 0 }),
            &mut scrollbar_state,
        );
    }
}

pub fn render_preview(
    f: &mut Frame<'_>,
    area: Rect,
    prompt: Option<&Prompt>,
    settings: &contracts::Settings,
) {
    let palette = get_palette(settings.theme_name.as_deref());

    let (color, title) = prompt.map_or_else(
        || (palette.muted, " Preview ".to_string()),
        |p| {
            let color = match p.r#type {
                contracts::PromptType::Prompt => palette.success,
                contracts::PromptType::Snippet => palette.secondary,
                contracts::PromptType::Note => palette.info,
            };

            let (display_title, _is_draft) = contracts::Processor::get_display_title(&p.text);
            let has_explicit_title = contracts::Processor::extract_title(&p.text).0.is_some();

            let title = if has_explicit_title {
                format!(" Preview: {display_title} ")
            } else {
                " Preview ".to_string()
            };

            (color, title)
        },
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
        .bg(palette.bg)
        .title(title);

    if let Some(prompt) = prompt {
        let (_, display_content) = contracts::Processor::extract_title(&prompt.text);
        let lines = crate::utils::highlight_text(&display_content, &palette);
        let paragraph =
            Paragraph::new(lines).block(block).wrap(ratatui::widgets::Wrap { trim: true });
        f.render_widget(paragraph, area);
    } else {
        let empty = Paragraph::new("No selection").block(block);
        f.render_widget(empty, area);
    }
}
