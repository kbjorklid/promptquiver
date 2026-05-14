use contracts::{PreviewMode, Tab};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::Stylize;
use ratatui::style::Style;
use ratatui::widgets::Clear;
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;
use ratatui_toaster::{ToastEngine, ToastMessage};

pub mod data_manager;
pub mod editor;
pub mod editor_module;
pub mod footer;
pub mod header;
pub mod history_manager;
pub mod list;
pub mod list_module;
pub mod project_manager;
pub mod project_picker;
pub mod settings;
pub mod shortcuts;
pub mod statusline;
pub mod types;
pub mod utils;

pub use editor_module::EditorModule;
pub use list_module::ListModule;
pub use types::{AppMessage, Mode, RenderState, UpdateContext};

pub fn render(
    f: &mut Frame<'_>,
    mut state: RenderState<'_, '_>,
    toaster: &mut Option<ToastEngine<ToastMessage>>,
) {
    let palette = crate::utils::get_palette(state.settings.theme_name.as_deref());

    // Render global background to ensure theme background covers everything
    f.render_widget(Block::default().bg(palette.bg), f.area());

    let is_searching = state.mode == Mode::Search;
    let (header_chunk, main_chunk, search_chunk, footer_chunk, statusline_chunk) =
        split_layout(f, is_searching);

    header::render(f, header_chunk, state.nav.active_tab, state.mode, state.settings);

    if let Some(s_chunk) = search_chunk {
        render_search_bar(f, s_chunk, &state.nav.search_query, palette.accent);
    }

    // 2. Main Chunk Layout (List + Preview)
    let (content_chunk, preview_chunk) = split_main_chunk(main_chunk, &state);

    let mut editor_content_area = None;

    if state.mode == Mode::Editor
        || state.mode == Mode::ConfirmDiscard
        || state.mode == Mode::ThemePicker
    {
        if state.nav.active_tab == Tab::Settings {
            settings::render(f, content_chunk, &mut state);
        } else {
            editor_content_area = Some(editor::render(f, content_chunk, &mut state));

            if state.mode == Mode::ConfirmDiscard {
                render_discard_popup(f, &palette);
            }
        }
    } else if state.nav.active_tab == Tab::Settings {
        settings::render(f, content_chunk, &mut state);
    } else {
        list::render(f, content_chunk, &mut state);
        if let Some(p_chunk) = preview_chunk {
            let selected_prompt = state.nav.prompts.get(state.nav.selected_index);
            list::render_preview(f, p_chunk, selected_prompt, state.settings);
        }
    }

    if let Some(area) = editor_content_area {
        editor::render_autocomplete(f, area, &mut state);
    }

    // Modals
    if state.mode == Mode::ExportDialog {
        data_manager::render_export_dialog(f, &state);
    }
    if state.mode == Mode::ImportDialog {
        data_manager::render_import_dialog(f, &state);
    }
    if state.mode == Mode::ProjectPicker
        || state.mode == Mode::AddProject
        || state.mode == Mode::RenameProject
    {
        project_picker::render_picker(
            f,
            &state.nav.projects_manager.projects,
            &mut state.nav.projects_manager.project_list_state,
            state.settings,
            if state.mode == Mode::AddProject || state.mode == Mode::RenameProject {
                Some(&state.nav.projects_manager.new_project_name)
            } else {
                None
            },
            state.nav.project_filter,
            state.nav.projects_manager.selecting_startup_project,
        );
    }

    statusline::render(f, statusline_chunk, &state);
    footer::render(f, footer_chunk, &state);

    if state.show_help {
        render_help_modal(f, &palette, state.help_scroll);
    }

    if let Some(ref mut toaster) = toaster {
        toaster.set_area(f.area());
        f.render_widget(&*toaster, f.area());
    }
}

fn split_layout(
    f: &Frame<'_>,
    is_searching: bool,
) -> (
    ratatui::layout::Rect,
    ratatui::layout::Rect,
    Option<ratatui::layout::Rect>,
    ratatui::layout::Rect,
    ratatui::layout::Rect,
) {
    let mut v_constraints = vec![
        Constraint::Length(1), // Header
        Constraint::Min(5),    // Main content
    ];
    if is_searching {
        v_constraints.push(Constraint::Length(1));
    }
    v_constraints.push(Constraint::Length(2)); // Footer
    v_constraints.push(Constraint::Length(1)); // Statusline

    let v_chunks =
        Layout::default().direction(Direction::Vertical).constraints(v_constraints).split(f.area());

    let header_chunk = v_chunks[0];
    let main_chunk = v_chunks[1];

    if is_searching {
        (header_chunk, main_chunk, Some(v_chunks[2]), v_chunks[3], v_chunks[4])
    } else {
        (header_chunk, main_chunk, None, v_chunks[2], v_chunks[3])
    }
}

fn render_search_bar(
    f: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    query: &str,
    color: ratatui::style::Color,
) {
    let prefix = "Search: /";
    let text = format!("{prefix}{query}");
    let paragraph = Paragraph::new(text).style(Style::default().fg(color));
    f.render_widget(paragraph, area);
}

fn split_main_chunk(
    main_chunk: ratatui::layout::Rect,
    state: &RenderState<'_, '_>,
) -> (ratatui::layout::Rect, Option<ratatui::layout::Rect>) {
    let show_preview = state.mode != Mode::Editor
        && state.mode != Mode::ConfirmDiscard
        && state.nav.active_tab != Tab::Settings
        && state.settings.preview_mode != PreviewMode::Hidden;

    if show_preview {
        match state.settings.preview_mode {
            PreviewMode::Side => {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(main_chunk);
                (chunks[0], Some(chunks[1]))
            }
            PreviewMode::Bottom => {
                if main_chunk.height >= 10 {
                    let preview_h = if main_chunk.height > 15 { 10 } else { main_chunk.height - 5 };
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Min(5), Constraint::Length(preview_h)])
                        .split(main_chunk);
                    (chunks[0], Some(chunks[1]))
                } else {
                    (main_chunk, None)
                }
            }
            PreviewMode::Hidden => (main_chunk, None),
        }
    } else {
        (main_chunk, None)
    }
}

fn render_discard_popup(f: &mut Frame<'_>, palette: &ratatui_themes::ThemePalette) {
    let text = ratatui::text::Text::from("\n  Are you sure you want to discard changes?  \n\n            (y) Yes, (n) No            ");
    let popup = tui_popup::Popup::new(text)
        .title(" Discard Changes? ")
        .style(Style::default().bg(palette.accent).fg(palette.bg))
        .border_style(Style::default().fg(palette.accent));
    f.render_widget(&popup, f.area());
}

fn render_help_modal(f: &mut Frame<'_>, palette: &ratatui_themes::ThemePalette, scroll: u16) {
    let help_text = include_str!("../../help.md");
    let text = tui_markdown::from_str(help_text);

    let area = f.area();
    let width = (area.width.saturating_sub(10)).min(100);
    let height = (area.height.saturating_sub(6)).min(40);

    let popup_area = Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    };

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Prompt Quiver Help ")
        .borders(ratatui::widgets::Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(palette.accent))
        .bg(palette.bg);

    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: false })
        .scroll((scroll, 0));

    f.render_widget(paragraph, popup_area);
}
