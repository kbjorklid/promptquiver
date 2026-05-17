use crate::list_module::MetadataEditorFocus;
use crate::types::RenderState;
use crate::utils::{format_path, get_palette};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

pub fn render_metadata_editor(f: &mut Frame<'_>, state: &mut RenderState<'_, '_>) {
    let palette = get_palette(state.settings.theme_name.as_deref());
    let area = f.area();

    let popup_area = crate::utils::centered_rect_fixed(62, 20, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Prompt Metadata Editor ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.accent))
        .bg(palette.bg);
    f.render_widget(block, popup_area);

    let inner = Rect {
        x: popup_area.x + 2,
        y: popup_area.y + 1,
        width: popup_area.width.saturating_sub(4),
        height: popup_area.height.saturating_sub(3),
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Folder label
            Constraint::Length(1), // Folder checkbox
            Constraint::Length(1), // spacer
            Constraint::Length(1), // Branch label
            Constraint::Length(1), // Branch checkbox
            Constraint::Length(1), // spacer
            Constraint::Length(1), // Project label
            Constraint::Min(4),    // Project list (bordered, min 2 visible items)
            Constraint::Length(1), // Hints
        ])
        .split(inner);

    let meta = &state.nav.metadata_editor;
    let current_path = &state.nav.current_path;
    let current_branch = state.current_branch;

    let section_label_style = Style::default().fg(palette.secondary).add_modifier(Modifier::BOLD);

    // Folder section
    let folder_focused = meta.focus == MetadataEditorFocus::FolderCheckbox;
    f.render_widget(Paragraph::new("Folder").style(section_label_style), chunks[0]);

    let folder_line = if meta.folder_disabled {
        format!("[✓] Already in current folder: {}", format_path(current_path))
    } else {
        let cb = if meta.use_current_folder { "[x]" } else { "[ ]" };
        format!("{cb} Move to current folder: {}", format_path(current_path))
    };
    let folder_style = if meta.folder_disabled {
        Style::default().fg(palette.muted)
    } else if folder_focused {
        Style::default().fg(palette.accent).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.fg)
    };
    f.render_widget(Paragraph::new(folder_line).style(folder_style), chunks[1]);

    // Branch section
    let branch_focused = meta.focus == MetadataEditorFocus::BranchCheckbox;
    f.render_widget(Paragraph::new("Branch").style(section_label_style), chunks[3]);

    let branch_line = if meta.branch_disabled {
        if current_branch.is_none() {
            "[—] No current branch detected".to_string()
        } else {
            format!("[✓] Already on current branch: {}", current_branch.unwrap_or(""))
        }
    } else {
        let cb = if meta.use_current_branch { "[x]" } else { "[ ]" };
        format!("{cb} Move to current branch: {}", current_branch.unwrap_or(""))
    };
    let branch_style = if meta.branch_disabled {
        Style::default().fg(palette.muted)
    } else if branch_focused {
        Style::default().fg(palette.accent).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.fg)
    };
    f.render_widget(Paragraph::new(branch_line).style(branch_style), chunks[4]);

    // Project section
    let project_focused = meta.focus == MetadataEditorFocus::ProjectList;
    f.render_widget(Paragraph::new("Project").style(section_label_style), chunks[6]);

    let mut items = vec![ListItem::new("  None (Default)  ")];
    for p in &state.nav.projects_manager.projects {
        items.push(ListItem::new(format!("  {}  ", p.title)));
    }

    let border_style = if project_focused {
        Style::default().fg(palette.accent)
    } else {
        Style::default().fg(palette.muted)
    };
    let highlight_bg = if project_focused { palette.accent } else { palette.secondary };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).border_style(border_style).bg(palette.bg))
        .highlight_style(
            Style::default().bg(highlight_bg).fg(palette.bg).add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, chunks[7], &mut state.nav.metadata_editor.project_list_state);

    // Hints
    let hints =
        "[ Tab ] Focus  [ Space ] Toggle  [ j/k ] Project  [ Enter ] Accept  [ Esc ] Cancel";
    f.render_widget(Paragraph::new(hints).style(Style::default().fg(palette.muted)), chunks[8]);
}
