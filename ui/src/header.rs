use crate::utils::get_palette;
use contracts::Tab;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::prelude::Stylize;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Paragraph, Tabs};
use ratatui::Frame;

pub fn render_branding(f: &mut Frame<'_>, area: Rect, palette: &ratatui_themes::ThemePalette) {
    let branding = Paragraph::new(" PROMPT QUIVER ")
        .style(Style::default().fg(palette.accent).bg(palette.bg).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    f.render_widget(branding, area);
}

pub fn render(f: &mut Frame<'_>, area: Rect, active_tab: Tab, settings: &contracts::Settings) {
    let palette = get_palette(settings.theme_name.as_deref());

    // Ensure the whole header area has the theme background
    f.render_widget(ratatui::widgets::Block::default().bg(palette.bg), area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20), // Branding
            Constraint::Min(10),    // Tabs
        ])
        .split(area);

    render_branding(f, chunks[0], &palette);
    render_tabs(f, chunks[1], active_tab, settings, &palette);
}

pub fn render_tabs(
    f: &mut Frame<'_>,
    area: Rect,
    active_tab: Tab,
    settings: &contracts::Settings,
    palette: &ratatui_themes::ThemePalette,
) {
    let visible_tabs = settings.visible_tabs();
    let tab_titles = visible_tabs
        .iter()
        .map(|t| {
            let icon = if settings.use_nerd_font {
                match t {
                    Tab::Prompts => "󰈚 ",
                    Tab::Canned => "󰏪 ",
                    Tab::Notes => "󰎚 ",
                    Tab::Snippets => "󰘦 ",
                    Tab::Archive => "󰗄 ",
                    Tab::Settings => "󰒓 ",
                }
            } else {
                match t {
                    Tab::Prompts => "📝 ",
                    Tab::Canned => "📦 ",
                    Tab::Notes => "📒 ",
                    Tab::Snippets => "✂️ ",
                    Tab::Archive => "📁 ",
                    Tab::Settings => "⚙️ ",
                }
            };
            format!(" {icon}{t:?} ")
        })
        .collect::<Vec<_>>();

    let tabs = Tabs::new(tab_titles)
        .divider("|")
        .select(visible_tabs.iter().position(|&t| t == active_tab).unwrap_or(0))
        .highlight_style(
            Style::default().bg(palette.accent).fg(palette.bg).add_modifier(Modifier::BOLD),
        )
        .style(Style::default().fg(palette.fg).bg(palette.bg));

    f.render_widget(tabs, area);
}
