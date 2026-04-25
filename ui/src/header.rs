use contracts::Tab;
use ratatui::widgets::{Block, Borders, Tabs};
use ratatui::style::{Style, Color, Modifier};
use ratatui::Frame;
use ratatui::layout::Rect;

pub fn render(f: &mut Frame<'_>, area: Rect, active_tab: Tab, current_branch: Option<&str>) {
    let tab_titles = Tab::all().iter().map(|t| format!("{:?}", t)).collect::<Vec<_>>();
    let branch_str = current_branch.map(|b| format!(" [b: {}] ", b)).unwrap_or_default();
    let title = format!(" PROMPT QUIVER {} ", branch_str);
    
    let tabs = Tabs::new(tab_titles)
        .block(Block::default().title(title).borders(Borders::ALL))
        .select(Tab::all().iter().position(|&t| t == active_tab).unwrap_or(0))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    
    f.render_widget(tabs, area);
}
