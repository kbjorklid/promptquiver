use crate::types::{Mode, RenderState};
use crate::utils::get_palette;
use contracts::Tab;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{
    Block, Borders, Clear, Gauge, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use ratatui::Frame;
use ratatui_themes::ThemePalette;

const AI_SECTION_ITEMS: usize = 6;

pub fn render(f: &mut Frame<'_>, area: Rect, state: &mut RenderState<'_, '_>) {
    let settings = state.settings;
    let palette = get_palette(settings.theme_name.as_deref());

    // Calculate heights
    let tabs: Vec<Tab> = Tab::all().into_iter().filter(|&t| t != Tab::Settings).collect();
    let tabs_len = tabs.len();
    let slash_len = settings.slash_commands.len();

    let tab_height = 8;
    let slash_height = u16::try_from(slash_len + 3).unwrap_or(u16::MAX);
    let mut advanced_count: usize = 7;
    if settings.startup_behavior == contracts::StartupBehavior::Specific {
        advanced_count += 1;
    }
    let advanced_height = u16::try_from(advanced_count + 2).unwrap_or(u16::MAX);

    let maintenance_height = 4;

    let ai_height = u16::try_from(AI_SECTION_ITEMS + 2).unwrap_or(u16::MAX);

    let total_height = tab_height + slash_height + maintenance_height + advanced_height + ai_height;

    update_scroll_offset(
        area,
        state,
        total_height,
        tab_height,
        slash_height,
        maintenance_height,
        advanced_height,
        tabs_len,
        slash_len,
    );

    let scroll_offset = state.nav.settings_scroll_offset;

    // Tab Visibility
    let tab_area = Rect {
        x: area.x,
        y: area.y.saturating_add(0).saturating_sub(scroll_offset),
        width: area.width,
        height: tab_height,
    };
    render_tab_visibility(f, area, tab_area, state, &tabs);

    // Slash Commands
    let slash_area = Rect {
        x: area.x,
        y: area.y.saturating_add(tab_height).saturating_sub(scroll_offset),
        width: area.width,
        height: slash_height,
    };
    render_slash_commands(f, area, slash_area, state, tabs_len);

    // Maintenance
    let maintenance_area = Rect {
        x: area.x,
        y: area
            .y
            .saturating_add(tab_height)
            .saturating_add(slash_height)
            .saturating_sub(scroll_offset),
        width: area.width,
        height: maintenance_height,
    };
    render_maintenance(f, area, maintenance_area, state, tabs_len, slash_len);

    // Advanced
    let advanced_area = Rect {
        x: area.x,
        y: area
            .y
            .saturating_add(tab_height)
            .saturating_add(slash_height)
            .saturating_add(maintenance_height)
            .saturating_sub(scroll_offset),
        width: area.width,
        height: advanced_height,
    };
    render_advanced(f, area, advanced_area, state, tabs_len, slash_len);

    // AI
    let ai_area = Rect {
        x: area.x,
        y: area
            .y
            .saturating_add(tab_height)
            .saturating_add(slash_height)
            .saturating_add(maintenance_height)
            .saturating_add(advanced_height)
            .saturating_sub(scroll_offset),
        width: area.width,
        height: ai_height,
    };
    render_ai(f, area, ai_area, state, tabs_len, slash_len);

    render_scrollbar(f, area, total_height, scroll_offset, &palette);

    if state.mode == Mode::ThemePicker {
        render_theme_picker(f, &palette, &mut state.nav.theme_list_state);
    }
}

#[allow(clippy::too_many_arguments)]
fn update_scroll_offset(
    area: Rect,
    state: &mut RenderState<'_, '_>,
    total_height: u16,
    tab_height: u16,
    slash_height: u16,
    maintenance_height: u16,
    advanced_height: u16,
    tabs_len: usize,
    slash_len: usize,
) {
    let selected_index = state.nav.selected_index;
    let maintenance_start = tabs_len + slash_len + 1;
    let advanced_start = maintenance_start + 2;
    let ai_start = advanced_start + 5
        + usize::from(
            state.settings.startup_behavior == contracts::StartupBehavior::Specific,
        );

    let selected_y = if selected_index < tabs_len {
        u16::try_from(selected_index).unwrap_or(u16::MAX).saturating_add(1)
    } else if selected_index <= tabs_len + slash_len {
        tab_height
            .saturating_add(u16::try_from(selected_index - tabs_len).unwrap_or(u16::MAX))
            .saturating_add(1)
    } else if selected_index < maintenance_start + 2 {
        tab_height
            .saturating_add(slash_height)
            .saturating_add(
                u16::try_from(selected_index - maintenance_start).unwrap_or(u16::MAX),
            )
            .saturating_add(1)
    } else if selected_index < ai_start {
        tab_height
            .saturating_add(slash_height)
            .saturating_add(maintenance_height)
            .saturating_add(
                u16::try_from(selected_index - advanced_start).unwrap_or(u16::MAX),
            )
            .saturating_add(1)
    } else {
        tab_height
            .saturating_add(slash_height)
            .saturating_add(maintenance_height)
            .saturating_add(advanced_height)
            .saturating_add(
                u16::try_from(selected_index - ai_start).unwrap_or(u16::MAX),
            )
            .saturating_add(1)
    };

    let scroll_offset = &mut state.nav.settings_scroll_offset;
    if selected_y < *scroll_offset + 1 {
        *scroll_offset = selected_y.saturating_sub(1);
    } else if selected_y > *scroll_offset + area.height.saturating_sub(2) {
        *scroll_offset = selected_y.saturating_sub(area.height.saturating_sub(2));
    }

    let max_scroll = total_height.saturating_sub(area.height);
    if *scroll_offset > max_scroll {
        *scroll_offset = max_scroll;
    }
}

fn render_tab_visibility(
    f: &mut Frame<'_>,
    area: Rect,
    tab_area: Rect,
    state: &RenderState<'_, '_>,
    tabs: &[Tab],
) {
    let palette = get_palette(state.settings.theme_name.as_deref());
    let selected_index = state.nav.selected_index;
    let is_tab_focused = selected_index < tabs.len();

    let items: Vec<ListItem<'_>> = tabs
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let is_visible = state.settings.tab_visibility.get(t).copied().unwrap_or(true);
            let prefix = if i == selected_index { "> " } else { "  " };
            let status = if is_visible { "[x]" } else { "[ ]" };
            let style = if i == selected_index {
                Style::default().bg(palette.accent).fg(palette.bg).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.fg)
            };
            ListItem::new(format!("{prefix} {status} {t:?}")).style(style)
        })
        .collect();

    let tab_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Tab Visibility (Space to toggle) ")
                .border_style(if is_tab_focused {
                    Style::default().fg(palette.accent)
                } else {
                    Style::default().fg(palette.fg)
                }),
        )
        .style(Style::default().bg(palette.bg));

    if tab_area.y < area.y + area.height && tab_area.y + tab_area.height > area.y {
        f.render_widget(tab_list, area.intersection(tab_area));
    }
}

fn render_slash_commands(
    f: &mut Frame<'_>,
    area: Rect,
    slash_area: Rect,
    state: &mut RenderState<'_, '_>,
    tabs_len: usize,
) {
    let palette = get_palette(state.settings.theme_name.as_deref());
    let selected_index = state.nav.selected_index;
    let slash_len = state.settings.slash_commands.len();
    let is_slash_focused = selected_index >= tabs_len && selected_index < tabs_len + slash_len + 1;
    let textarea = if state.mode == Mode::Editor { Some(&state.editor.textarea) } else { None };

    let mut slash_items: Vec<ListItem<'_>> = state
        .settings
        .slash_commands
        .iter()
        .enumerate()
        .map(|(i, cmd)| {
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
        })
        .collect();

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
        slash_items
            .push(ListItem::new(format!("{add_prefix} + Add New Slash Command")).style(add_style));
    }

    let slash_list = List::new(slash_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Slash Commands (Enter to edit, d to delete) ")
                .border_style(if is_slash_focused {
                    Style::default().fg(palette.accent)
                } else {
                    Style::default().fg(palette.fg)
                }),
        )
        .style(Style::default().bg(palette.bg));

    if slash_area.y < area.y + area.height && slash_area.y + slash_area.height > area.y {
        let render_area = area.intersection(slash_area);
        f.render_stateful_widget(slash_list, render_area, &mut state.nav.settings_slash_list_state);
        if let Some(ta) = textarea {
            render_slash_editor(f, area, slash_area, state.nav, tabs_len, slash_len, ta);
        }
    }
}

fn render_slash_editor(
    f: &mut Frame<'_>,
    area: Rect,
    slash_area: Rect,
    nav: &crate::list_module::ListModule,
    tabs_len: usize,
    slash_len: usize,
    ta: &ratatui_textarea::TextArea<'_>,
) {
    let selected_index = nav.selected_index;
    if selected_index >= tabs_len && selected_index <= tabs_len + slash_len {
        let offset = nav.settings_slash_list_state.offset();
        let relative_idx = selected_index - tabs_len;

        if relative_idx >= offset {
            let y_offset = u16::try_from(relative_idx - offset).unwrap_or(u16::MAX);
            let line_y = slash_area.y + 1 + y_offset;
            if line_y >= area.y && line_y < area.y + area.height {
                let cmd_area = Rect {
                    x: slash_area.x + 5,
                    y: line_y,
                    width: slash_area.width.saturating_sub(7),
                    height: 1,
                };
                f.render_widget(Clear, cmd_area);
                f.render_widget(ta, cmd_area);
            }
        }
    }
}

fn render_maintenance(
    f: &mut Frame<'_>,
    area: Rect,
    maintenance_area: Rect,
    state: &RenderState<'_, '_>,
    tabs_len: usize,
    slash_len: usize,
) {
    let palette = get_palette(state.settings.theme_name.as_deref());
    let selected_index = state.nav.selected_index;
    let maintenance_idx = tabs_len + slash_len + 1;
    let is_maintenance_focused =
        selected_index >= maintenance_idx && selected_index < maintenance_idx + 2;

    let maintenance_block = Block::default()
        .borders(Borders::ALL)
        .title(" Maintenance (Enter to run) ")
        .border_style(if is_maintenance_focused {
            Style::default().fg(palette.accent)
        } else {
            Style::default().fg(palette.fg)
        });

    let items = vec![
        ListItem::new(format!(
            "{} Export Data (TOML)",
            if selected_index == maintenance_idx { ">" } else { " " }
        ))
        .style(if selected_index == maintenance_idx {
            Style::default().bg(palette.accent).fg(palette.bg).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.fg)
        }),
        ListItem::new(format!(
            "{} Import Data (TOML)",
            if selected_index == maintenance_idx + 1 { ">" } else { " " }
        ))
        .style(if selected_index == maintenance_idx + 1 {
            Style::default().bg(palette.accent).fg(palette.bg).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.fg)
        }),
    ];

    let maintenance_list =
        List::new(items).block(maintenance_block).style(Style::default().bg(palette.bg));

    if maintenance_area.y < area.y + area.height
        && maintenance_area.y + maintenance_area.height > area.y
    {
        f.render_widget(maintenance_list, area.intersection(maintenance_area));
    }
}

fn advanced_item(
    label: &str,
    selected: bool,
    palette: &ratatui_themes::ThemePalette,
) -> ListItem<'static> {
    let text = format!("{} {}", if selected { ">" } else { " " }, label);
    ListItem::new(text).style(if selected {
        Style::default().bg(palette.accent).fg(palette.bg).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.fg)
    })
}

fn build_advanced_items(
    state: &RenderState<'_, '_>,
    selected_index: usize,
    advanced_idx: usize,
    palette: &ratatui_themes::ThemePalette,
) -> Vec<ListItem<'static>> {
    let s = state.settings;
    let on_off = |v: bool| if v { "[ON]" } else { "[OFF]" };
    let theme = s.theme_name.as_deref().unwrap_or("Default");
    let behavior = format!("{:?}", s.startup_behavior);

    let mut items = vec![
        advanced_item(
            &format!(
                "Enable Claude Command and Skill Discovery: {}",
                on_off(s.enable_claude_commands)
            ),
            selected_index == advanced_idx,
            palette,
        ),
        advanced_item(
            &format!(
                "Enable Claude Built-in Commands: {}",
                on_off(s.enable_claude_builtin_commands)
            ),
            selected_index == advanced_idx + 1,
            palette,
        ),
        advanced_item(
            &format!("Use Nerd Font Icons: {}", on_off(s.use_nerd_font)),
            selected_index == advanced_idx + 2,
            palette,
        ),
        advanced_item(&format!("Theme: {theme}"), selected_index == advanced_idx + 3, palette),
        advanced_item(
            &format!("Project selection at startup: {behavior}"),
            selected_index == advanced_idx + 4,
            palette,
        ),
    ];

    if s.startup_behavior == contracts::StartupBehavior::Specific {
        let project_name = s.specific_project_id.map_or_else(
            || "Default".into(),
            |id| {
                state
                    .nav
                    .projects_manager
                    .projects
                    .iter()
                    .find(|p| p.id == id)
                    .map_or_else(|| "Default".into(), |p| p.title.clone())
            },
        );
        items.push(advanced_item(
            &format!("Startup Project: {project_name}"),
            selected_index == advanced_idx + 5,
            palette,
        ));
    }

    items
}

fn render_advanced(
    f: &mut Frame<'_>,
    area: Rect,
    advanced_area: Rect,
    state: &RenderState<'_, '_>,
    tabs_len: usize,
    slash_len: usize,
) {
    let palette = get_palette(state.settings.theme_name.as_deref());
    let selected_index = state.nav.selected_index;
    let advanced_idx = tabs_len + slash_len + 1 + 2;
    let is_advanced_focused = selected_index >= advanced_idx;

    let advanced_block = Block::default()
        .borders(Borders::ALL)
        .title(" Advanced (Space to toggle) ")
        .border_style(if is_advanced_focused {
            Style::default().fg(palette.accent)
        } else {
            Style::default().fg(palette.fg)
        });

    let items = build_advanced_items(state, selected_index, advanced_idx, &palette);
    let advanced_list =
        List::new(items).block(advanced_block).style(Style::default().bg(palette.bg));

    if advanced_area.y < area.y + area.height && advanced_area.y + advanced_area.height > area.y {
        f.render_widget(advanced_list, area.intersection(advanced_area));
    }
}

fn render_scrollbar(
    f: &mut Frame<'_>,
    area: Rect,
    total_height: u16,
    scroll_offset: u16,
    palette: &ThemePalette,
) {
    if total_height > area.height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .style(Style::default().fg(palette.fg));

        let mut scrollbar_state =
            ScrollbarState::new(total_height as usize).position(scroll_offset as usize);

        f.render_stateful_widget(
            scrollbar,
            area.inner(ratatui::layout::Margin { vertical: 0, horizontal: 0 }),
            &mut scrollbar_state,
        );
    }
}

fn render_ai(
    f: &mut Frame<'_>,
    area: Rect,
    ai_area: Rect,
    state: &RenderState<'_, '_>,
    tabs_len: usize,
    slash_len: usize,
) {
    let palette = get_palette(state.settings.theme_name.as_deref());
    let selected_index = state.nav.selected_index;
    let s = state.settings;

    let maintenance_start = tabs_len + slash_len + 1;
    let advanced_start = maintenance_start + 2;
    let ai_idx = advanced_start + 5
        + usize::from(s.startup_behavior == contracts::StartupBehavior::Specific);
    let is_ai_focused = selected_index >= ai_idx;

    let on_off = |v: bool| if v { "[ON]" } else { "[OFF]" };
    let tier_label = match s.ai_model_tier {
        contracts::ModelTier::Fast => "Fast (gemma-4-E2B-it)",
        contracts::ModelTier::Balanced => "Balanced (gemma-4-E4B-it)",
        contracts::ModelTier::Quality => "Quality (gemma-3-12b-it)",
    };
    let token_display = if s.hf_token.as_ref().is_some_and(|t| !t.is_empty()) {
        "••••••••"
    } else {
        "(not set)"
    };
    let path_display = s.ai_model_path.as_deref().unwrap_or("(not set)");

    let labels = [
        format!("Enable AI features: {}", on_off(s.ai_enabled)),
        format!("Model tier: {tier_label}"),
        format!("Auto-title on save: {}", on_off(s.ai_auto_title)),
        "Download model (Enter)".to_string(),
        format!("HuggingFace token: {token_display}"),
        format!("Custom model path: {path_display}"),
    ];

    let items: Vec<ListItem<'_>> = labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let idx = ai_idx + i;
            let is_sel = selected_index == idx;
            let style = if is_sel {
                Style::default().bg(palette.accent).fg(palette.bg).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.fg)
            };
            ListItem::new(format!("{} {label}", if is_sel { ">" } else { " " })).style(style)
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" AI (Space to toggle, Enter to act) ")
        .border_style(if is_ai_focused {
            Style::default().fg(palette.accent)
        } else {
            Style::default().fg(palette.fg)
        });

    let list = List::new(items).block(block).style(Style::default().bg(palette.bg));

    if ai_area.y < area.y + area.height && ai_area.y + ai_area.height > area.y {
        f.render_widget(list, area.intersection(ai_area));
    }

    // Download progress bar
    if let Some(pct) = state.ai_download_progress {
        let bar_area = Rect {
            x: ai_area.x + 2,
            y: ai_area.y + 4,
            width: ai_area.width.saturating_sub(4),
            height: 1,
        };
        if bar_area.y >= area.y && bar_area.y < area.y + area.height {
            let gauge = Gauge::default()
                .gauge_style(Style::default().fg(palette.accent).bg(palette.bg))
                .ratio(f64::from(pct).clamp(0.0, 1.0));
            f.render_widget(gauge, area.intersection(bar_area));
        }
    }
}

fn render_theme_picker(
    f: &mut Frame<'_>,
    palette: &ThemePalette,
    theme_list_state: &mut ratatui::widgets::ListState,
) {
    use ratatui_themes::ThemeName;
    let themes = ThemeName::all();
    let items: Vec<ListItem<'_>> = themes.iter().map(|t| ListItem::new(format!("{t:?}"))).collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Select Theme "))
        .style(Style::default().bg(palette.bg).fg(palette.fg))
        .highlight_style(
            Style::default().bg(palette.accent).fg(palette.bg).add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let picker_area = crate::utils::centered_rect(60, 60, f.area());
    f.render_widget(Clear, picker_area);
    f.render_stateful_widget(list, picker_area, theme_list_state);
}
