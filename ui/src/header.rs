use contracts::Tab;
use ratatui::widgets::{Tabs, Paragraph};
use ratatui::style::{Style, Color, Modifier};
use ratatui::Frame;
use ratatui::layout::{Rect, Alignment, Layout, Constraint, Direction};

pub fn render_branding(f: &mut Frame<'_>, area: Rect) {
    let branding = Paragraph::new(" PROMPT QUIVER ")
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    f.render_widget(branding, area);
}

pub fn render(f: &mut Frame<'_>, area: Rect, active_tab: Tab, settings: &contracts::Settings) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20), // Branding
            Constraint::Min(10),   // Tabs
        ])
        .split(area);

    render_branding(f, chunks[0]);
    render_tabs(f, chunks[1], active_tab, settings);
}

pub fn render_tabs(f: &mut Frame<'_>, area: Rect, active_tab: Tab, settings: &contracts::Settings) {
    let tab_titles = Tab::all().iter().map(|t| {
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
        format!(" {}{} ", icon, format!("{:?}", t))
    }).collect::<Vec<_>>();
    
    let tabs = Tabs::new(tab_titles)
        .divider("|")
        .select(Tab::all().iter().position(|&t| t == active_tab).unwrap_or(0))
        .highlight_style(Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD))
        .style(Style::default().fg(Color::Gray));
    
    f.render_widget(tabs, area);
}
