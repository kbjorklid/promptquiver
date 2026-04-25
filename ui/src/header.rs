use contracts::Tab;
use ratatui::widgets::{Block, Borders, Tabs, Paragraph};
use ratatui::style::{Style, Color, Modifier};
use ratatui::Frame;
use ratatui::layout::{Rect, Alignment, Layout, Constraint, Direction};

pub fn render_branding(f: &mut Frame<'_>, area: Rect) {
    let branding = Paragraph::new(" PROMPT QUIVER ")
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    f.render_widget(branding, area);
}

pub fn render(f: &mut Frame<'_>, area: Rect, active_tab: Tab, current_branch: Option<&str>) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20), // Branding
            Constraint::Min(10),   // Tabs
        ])
        .split(area);

    render_branding(f, chunks[0]);
    render_tabs(f, chunks[1], active_tab, current_branch);
}

pub fn render_tabs(f: &mut Frame<'_>, area: Rect, active_tab: Tab, current_branch: Option<&str>) {
    let tab_titles = Tab::all().iter().map(|t| format!(" {:?} ", t)).collect::<Vec<_>>();
    let branch_str = current_branch.map(|b| format!(" [b: {}] ", b)).unwrap_or_default();
    
    let tabs = Tabs::new(tab_titles)
        .block(Block::default().title(branch_str).borders(Borders::ALL))
        .select(Tab::all().iter().position(|&t| t == active_tab).unwrap_or(0))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .style(Style::default().fg(Color::Gray));
    
    f.render_widget(tabs, area);
}
