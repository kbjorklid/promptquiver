use crate::types::RenderState;
use crate::utils::get_palette;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn render_export_dialog(f: &mut Frame<'_>, state: &RenderState<'_, '_>) {
    let palette = get_palette(state.settings.theme_name.as_deref());
    let area = f.area();

    let width = 60;
    let height = 12;
    let popup_area = crate::utils::centered_rect_fixed(width, height, area);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Export Data to TOML ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.accent))
        .bg(palette.bg);
    f.render_widget(block, popup_area);

    let inner = Rect {
        x: popup_area.x + 2,
        y: popup_area.y + 2,
        width: popup_area.width.saturating_sub(4),
        height: popup_area.height.saturating_sub(4),
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Label
            Constraint::Length(3), // Path input
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Checkbox
            Constraint::Min(0),    // Actions
        ])
        .split(inner);

    // Path Label
    f.render_widget(Paragraph::new("Target File Path:"), chunks[0]);

    // Path Input
    let path_style = if state.nav.data_manager.focus_checkbox {
        Style::default().fg(palette.fg)
    } else {
        Style::default().fg(palette.accent).add_modifier(Modifier::BOLD)
    };
    let path_block = Block::default().borders(Borders::ALL).border_style(path_style);
    f.render_widget(
        Paragraph::new(state.nav.data_manager.path.as_str()).block(path_block),
        chunks[1],
    );

    // Checkbox
    let cb_style = if state.nav.data_manager.focus_checkbox {
        Style::default().fg(palette.accent).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.fg)
    };
    let cb_text = format!(
        "{} Include Archived Prompts",
        if state.nav.data_manager.include_archived { "[x]" } else { "[ ]" }
    );
    f.render_widget(Paragraph::new(cb_text).style(cb_style), chunks[3]);

    // Actions
    let action_text = "  [ Enter ] Export    [ Esc ] Cancel  ";
    f.render_widget(
        Paragraph::new(action_text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(palette.muted)),
        chunks[4],
    );
}

pub fn render_import_dialog(f: &mut Frame<'_>, state: &RenderState<'_, '_>) {
    let palette = get_palette(state.settings.theme_name.as_deref());
    let area = f.area();

    let width = 60;
    let height = 10;
    let popup_area = crate::utils::centered_rect_fixed(width, height, area);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Import Data from TOML ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.accent))
        .bg(palette.bg);
    f.render_widget(block, popup_area);

    let inner = Rect {
        x: popup_area.x + 2,
        y: popup_area.y + 2,
        width: popup_area.width.saturating_sub(4),
        height: popup_area.height.saturating_sub(4),
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Label
            Constraint::Length(3), // Path input
            Constraint::Min(0),    // Actions
        ])
        .split(inner);

    // Path Label
    f.render_widget(Paragraph::new("Source File Path:"), chunks[0]);

    // Path Input
    let path_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.accent).add_modifier(Modifier::BOLD));
    f.render_widget(
        Paragraph::new(state.nav.data_manager.path.as_str()).block(path_block),
        chunks[1],
    );

    // Actions
    let action_text = "\n  [ Enter ] Import    [ Esc ] Cancel  ";
    let warning_text = "\n  WARNING: This will OVERWRITE your current database!  ";
    let text = format!("{}\n{}", action_text, warning_text.red().bold());

    f.render_widget(
        Paragraph::new(ratatui::text::Text::from(text)).alignment(Alignment::Center),
        chunks[2],
    );
}
