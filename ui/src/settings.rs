use contracts::{Settings, Tab};
use ratatui::widgets::{Block, Borders, List, ListItem, Clear, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::style::{Style, Modifier};
use ratatui::Frame;
use ratatui::layout::Rect;
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
    scroll_offset: &mut u16,
    projects: &[contracts::Project],
) {
    let palette = get_palette(settings.theme_name.as_deref());
    
    // Calculate heights
    let tabs: Vec<Tab> = Tab::all().into_iter().filter(|&t| t != Tab::Settings).collect();
    let tabs_len = tabs.len();
    let slash_len = settings.slash_commands.len();
    
    let tab_height = 8;
    let slash_height = (slash_len + 3) as u16;
    
    let mut advanced_count = 6;
    if settings.startup_behavior == contracts::StartupBehavior::Specific {
        advanced_count += 1;
    }
    let advanced_height = (advanced_count + 2) as u16;
    let total_height = tab_height + slash_height + advanced_height;

    // Determine current selected Y position (relative to start of settings)
    let selected_y = if selected_index < tabs_len {
        (selected_index as u16) + 1
    } else if selected_index <= tabs_len + slash_len {
        tab_height + (selected_index - tabs_len) as u16 + 1
    } else {
        tab_height + slash_height + (selected_index - (tabs_len + slash_len + 1)) as u16 + 1
    };

    // Keep selected item in view
    if selected_y < *scroll_offset + 1 {
        *scroll_offset = selected_y.saturating_sub(1);
    } else if selected_y > *scroll_offset + area.height.saturating_sub(2) {
        *scroll_offset = selected_y.saturating_sub(area.height.saturating_sub(2));
    }

    // Limit scroll offset
    let max_scroll = total_height.saturating_sub(area.height);
    if *scroll_offset > max_scroll {
        *scroll_offset = max_scroll;
    }

    // Tab Visibility
    let tabs_len = tabs.len();
    let is_tab_focused = selected_index < tabs_len;
    let tab_area = Rect {
        x: area.x,
        y: area.y.saturating_add(0).saturating_sub(*scroll_offset),
        width: area.width,
        height: tab_height,
    };

    let items: Vec<ListItem<'_>> = tabs.iter().enumerate().map(|(i, t)| {
        let is_visible = settings.tab_visibility.get(t).copied().unwrap_or(true);
        let prefix = if i == selected_index { "> " } else { "  " };
        let status = if is_visible { "[x]" } else { "[ ]" };
        let style = if i == selected_index {
            Style::default().bg(palette.accent).fg(palette.bg).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.fg)
        };
        ListItem::new(format!("{prefix} {status} {t:?}")).style(style)
    }).collect();

    let tab_list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(" Tab Visibility (Space to toggle) ")
            .border_style(if is_tab_focused { Style::default().fg(palette.accent) } else { Style::default().fg(palette.fg) }))
        .style(Style::default().bg(palette.bg));
    
    // Only render if visible
    if tab_area.y < area.y + area.height && tab_area.y + tab_area.height > area.y {
        let render_area = area.intersection(tab_area);
        // We need to handle the fact that List doesn't support offset easily when rendered partially
        // But intersection + clearing might work if we are careful.
        // Actually, Ratatui clips automatically if we provide a smaller area.
        f.render_widget(tab_list, render_area);
    }

    // Slash Commands
    let slash_area = Rect {
        x: area.x,
        y: area.y.saturating_add(tab_height).saturating_sub(*scroll_offset),
        width: area.width,
        height: slash_height,
    };

    let is_slash_focused = selected_index >= tabs_len && selected_index < tabs_len + slash_len + 1;
    let slash_block = Block::default()
        .borders(Borders::ALL)
        .title(" Slash Commands (Enter to edit, d to delete) ")
        .border_style(if is_slash_focused { Style::default().fg(palette.accent) } else { Style::default().fg(palette.fg) });

    let mut slash_items: Vec<ListItem<'_>> = settings.slash_commands.iter().enumerate().map(|(i, cmd)| {
        let idx = tabs_len + i;
        let prefix = if idx == selected_index { "> " } else { "  " };
        let style = if idx == selected_index {
            Style::default().bg(palette.accent).fg(palette.bg).add_modifier(Modifier::BOLD)
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
        Style::default().bg(palette.accent).fg(palette.bg).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.muted)
    };
    if add_idx == selected_index && textarea.is_some() {
        slash_items.push(ListItem::new(format!("{add_prefix} ")).style(add_style));
    } else {
        slash_items.push(ListItem::new(format!("{add_prefix} + Add New Slash Command")).style(add_style));
    }

    let slash_list = List::new(slash_items).block(slash_block).style(Style::default().bg(palette.bg));
    
    if slash_area.y < area.y + area.height && slash_area.y + slash_area.height > area.y {
        let render_area = area.intersection(slash_area);
        f.render_stateful_widget(slash_list, render_area, slash_list_state);

        // Render TextArea in-line
        if let Some(ta) = textarea {
            if selected_index >= tabs_len && selected_index <= tabs_len + slash_len {
                let offset = slash_list_state.offset();
                let relative_idx = selected_index - tabs_len;
                
                if relative_idx >= offset {
                    let y_offset = (relative_idx - offset) as u16;
                    // Check if the line is within the visible portion of slash_area AND within the screen area
                    let line_y = slash_area.y + 1 + y_offset;
                    if line_y >= area.y && line_y < area.y + area.height {
                        let ta_area = Rect {
                            x: slash_area.x + 5,
                            y: line_y,
                            width: slash_area.width.saturating_sub(7),
                            height: 1,
                        };
                        f.render_widget(Clear, ta_area);
                        f.render_widget(ta, ta_area);
                    }
                }
            }
        }
    }

    // Advanced
    let advanced_area = Rect {
        x: area.x,
        y: area.y.saturating_add(tab_height).saturating_add(slash_height).saturating_sub(*scroll_offset),
        width: area.width,
        height: advanced_height,
    };

    let advanced_idx = tabs_len + slash_len + 1;
    let is_advanced_focused = selected_index >= advanced_idx;
    
    let advanced_block = Block::default()
        .borders(Borders::ALL)
        .title(" Advanced (Space to toggle) ")
        .border_style(if is_advanced_focused { Style::default().fg(palette.accent) } else { Style::default().fg(palette.fg) });

    let claude_status = if settings.enable_claude_commands { "[ON]" } else { "[OFF]" };
    let nerd_status = if settings.use_nerd_font { "[ON]" } else { "[OFF]" };
    let current_theme = settings.theme_name.as_deref().unwrap_or("Default");
    let behavior_status = format!("{:?}", settings.startup_behavior);

    let mut advanced_items = vec![
        ListItem::new(format!("{} Enable Claude Commands: {}", if selected_index == advanced_idx { ">" } else { " " }, claude_status))
            .style(if selected_index == advanced_idx { Style::default().bg(palette.accent).fg(palette.bg).add_modifier(Modifier::BOLD) } else { Style::default().fg(palette.fg) }),
        ListItem::new(format!("{} Use Nerd Font Icons: {}", if selected_index == advanced_idx + 1 { ">" } else { " " }, nerd_status))
            .style(if selected_index == advanced_idx + 1 { Style::default().bg(palette.accent).fg(palette.bg).add_modifier(Modifier::BOLD) } else { Style::default().fg(palette.fg) }),
        ListItem::new(format!("{} Theme: {}", if selected_index == advanced_idx + 2 { ">" } else { " " }, current_theme))
            .style(if selected_index == advanced_idx + 2 { Style::default().bg(palette.accent).fg(palette.bg).add_modifier(Modifier::BOLD) } else { Style::default().fg(palette.fg) }),
        ListItem::new(format!("{} Startup Behavior: {}", if selected_index == advanced_idx + 3 { ">" } else { " " }, behavior_status))
            .style(if selected_index == advanced_idx + 3 { Style::default().bg(palette.accent).fg(palette.bg).add_modifier(Modifier::BOLD) } else { Style::default().fg(palette.fg) }),
    ];

    if settings.startup_behavior == contracts::StartupBehavior::Specific {
        let project_name = if let Some(id) = settings.specific_project_id {
            projects.iter().find(|p| p.id == id).map(|p| p.title.clone()).unwrap_or_else(|| "Default".into())
        } else {
            "Default".into()
        };
        advanced_items.push(
            ListItem::new(format!("{} Startup Project: {}", if selected_index == advanced_idx + 4 { ">" } else { " " }, project_name))
                .style(if selected_index == advanced_idx + 4 { Style::default().bg(palette.accent).fg(palette.bg).add_modifier(Modifier::BOLD) } else { Style::default().fg(palette.fg) })
        );
    }

    let advanced_list = List::new(advanced_items).block(advanced_block).style(Style::default().bg(palette.bg));
    
    if advanced_area.y < area.y + area.height && advanced_area.y + advanced_area.height > area.y {
        let render_area = area.intersection(advanced_area);
        f.render_widget(advanced_list, render_area);
    }

    // Render scrollbar for the whole settings view
    if total_height > area.height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .style(Style::default().fg(palette.fg));
        
        let mut scrollbar_state = ScrollbarState::new(total_height as usize)
            .position(*scroll_offset as usize);
            
        f.render_stateful_widget(
            scrollbar,
            area.inner(ratatui::layout::Margin {
                vertical: 0,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }

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

        let picker_area = crate::utils::centered_rect(60, 60, f.area());
        f.render_widget(Clear, picker_area);
        f.render_stateful_widget(list, picker_area, theme_list_state);
    }
}

