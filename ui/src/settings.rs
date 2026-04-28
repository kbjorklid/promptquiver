use contracts::{Settings, Tab};
use ratatui::widgets::{Block, Borders, List, ListItem, Clear, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::style::{Style, Modifier};
use ratatui::Frame;
use ratatui::layout::{Rect, Layout, Constraint, Direction};
use ratatui_textarea::TextArea;
use crate::utils::get_palette;

pub fn render(
    f: &mut Frame<'_>,
    area: Rect,
    settings: &Settings,
    selected_index: usize,
    textarea: Option<&TextArea<'_>>,
    slash_list_state: &mut ratatui::widgets::ListState,
    theme_list_state: &mut ratatui::widgets::ListState,
    theme_picker_open: bool,
) {
    let palette = get_palette(settings.theme_name.as_deref());
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Tab Visibility
            Constraint::Min(5),    // Slash Commands
            Constraint::Length(5),  // Advanced (increased from 4 to 5)
        ])
        .split(area);

    // Tab Visibility
    let tabs = Tab::all();
    let items: Vec<ListItem<'_>> = tabs.iter().enumerate().map(|(i, t)| {
        let is_visible = settings.tab_visibility.get(t).copied().unwrap_or(true);
        let prefix = if i == selected_index { "> " } else { "  " };
        let status = if is_visible { "[x]" } else { "[ ]" };
        let style = if i == selected_index {
            Style::default().fg(palette.accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.fg)
        };
        ListItem::new(format!("{prefix} {status} {t:?}")).style(style)
    }).collect();

    let tab_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Tab Visibility (Space to toggle) "))
        .style(Style::default().bg(palette.bg));
    f.render_widget(tab_list, chunks[0]);

    // Slash Commands
    let tabs_len = tabs.len();
    let slash_len = settings.slash_commands.len();
    
    let is_slash_focused = selected_index >= tabs_len && selected_index < tabs_len + slash_len + 1;
    let slash_block = Block::default()
        .borders(Borders::ALL)
        .title(" Slash Commands (Enter to edit, d to delete) ")
        .border_style(if is_slash_focused { Style::default().fg(palette.accent) } else { Style::default().fg(palette.fg) });

    let mut slash_items: Vec<ListItem<'_>> = settings.slash_commands.iter().enumerate().map(|(i, cmd)| {
        let idx = tabs_len + i;
        let prefix = if idx == selected_index { "> " } else { "  " };
        let style = if idx == selected_index {
            Style::default().fg(palette.accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.fg)
        };
        
        if idx == selected_index && textarea.is_some() {
            ListItem::new(format!("{prefix} /")).style(style)
        } else {
            ListItem::new(format!("{prefix} /{cmd}")).style(style)
        }
    }).collect();

    // Add New item
    let add_idx = tabs_len + slash_len;
    let add_prefix = if add_idx == selected_index { "> " } else { "  " };
    let add_style = if add_idx == selected_index {
        Style::default().fg(palette.accent).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.muted)
    };
    if add_idx == selected_index && textarea.is_some() {
        slash_items.push(ListItem::new(format!("{add_prefix} ")).style(add_style));
    } else {
        slash_items.push(ListItem::new(format!("{add_prefix} + Add New Slash Command")).style(add_style));
    }

    let slash_list = List::new(slash_items).block(slash_block).style(Style::default().bg(palette.bg));
    f.render_stateful_widget(slash_list, chunks[1], slash_list_state);

    // Render scrollbar for slash commands
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"))
        .style(Style::default().fg(palette.fg));
    
    let mut scrollbar_state = ScrollbarState::new(slash_len + 1);
    if is_slash_focused {
        scrollbar_state = scrollbar_state.position(selected_index - tabs_len);
    }
        
    f.render_stateful_widget(
        scrollbar,
        chunks[1].inner(ratatui::layout::Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut scrollbar_state,
    );

    // Render TextArea in-line
    if let Some(ta) = textarea {
        if selected_index >= tabs_len && selected_index <= tabs_len + slash_len {
            let offset = slash_list_state.offset();
            let relative_idx = selected_index - tabs_len;
            
            if relative_idx >= offset {
                let y_offset = (relative_idx - offset) as u16;
                if y_offset < chunks[1].height.saturating_sub(2) {
                    let area = Rect {
                        x: chunks[1].x + 5,
                        y: chunks[1].y + 1 + y_offset,
                        width: chunks[1].width.saturating_sub(7),
                        height: 1,
                    };
                    f.render_widget(Clear, area);
                    f.render_widget(ta, area);
                }
            }
        }
    }

    // Advanced
    let advanced_idx = tabs_len + slash_len + 1;
    let is_advanced_focused = selected_index >= advanced_idx;
    
    let advanced_block = Block::default()
        .borders(Borders::ALL)
        .title(" Advanced (Space to toggle) ")
        .border_style(if is_advanced_focused { Style::default().fg(palette.accent) } else { Style::default().fg(palette.fg) });

    let claude_status = if settings.enable_claude_commands { "[ON]" } else { "[OFF]" };
    let nerd_status = if settings.use_nerd_font { "[ON]" } else { "[OFF]" };
    let current_theme = settings.theme_name.as_deref().unwrap_or("Default");

    let advanced_items = vec![
        ListItem::new(format!("{} Enable Claude Commands: {}", if selected_index == advanced_idx { ">" } else { " " }, claude_status))
            .style(if selected_index == advanced_idx { Style::default().fg(palette.accent).add_modifier(Modifier::BOLD) } else { Style::default().fg(palette.fg) }),
        ListItem::new(format!("{} Use Nerd Font Icons: {}", if selected_index == advanced_idx + 1 { ">" } else { " " }, nerd_status))
            .style(if selected_index == advanced_idx + 1 { Style::default().fg(palette.accent).add_modifier(Modifier::BOLD) } else { Style::default().fg(palette.fg) }),
        ListItem::new(format!("{} Theme: {}", if selected_index == advanced_idx + 2 { ">" } else { " " }, current_theme))
            .style(if selected_index == advanced_idx + 2 { Style::default().fg(palette.accent).add_modifier(Modifier::BOLD) } else { Style::default().fg(palette.fg) }),
    ];

    let advanced_list = List::new(advanced_items).block(advanced_block).style(Style::default().bg(palette.bg));
    f.render_widget(advanced_list, chunks[2]);

    if theme_picker_open {
        use ratatui_themes::ThemeName;
        let themes = ThemeName::all();
        let items: Vec<ListItem<'_>> = themes.iter().map(|t| {
            ListItem::new(format!("{t:?}"))
        }).collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(" Select Theme "))
            .style(Style::default().bg(palette.bg).fg(palette.fg))
            .highlight_style(Style::default().bg(palette.accent).fg(palette.bg).add_modifier(Modifier::BOLD))
            .highlight_symbol("> ");

        let area = crate::utils::centered_rect(60, 60, f.area());
        f.render_widget(Clear, area);
        f.render_stateful_widget(list, area, theme_list_state);
    }
}
