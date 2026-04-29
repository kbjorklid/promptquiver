use contracts::{Tab, PreviewMode};
use ratatui::layout::{Layout, Constraint, Direction};
use ratatui::widgets::{Paragraph, Block};
use ratatui::style::Style;
use ratatui::Frame;
use ratatui::prelude::Stylize;
use ratatui_toaster::{ToastEngine, ToastMessage};

pub mod header;
pub mod list;
pub mod footer;
pub mod editor;
pub mod utils;
pub mod settings;
pub mod statusline;
pub mod shortcuts;
pub mod types;
pub mod list_module;
pub mod editor_module;

pub use types::{Mode, AppMessage, UpdateContext};
pub use list_module::ListModule;
pub use editor_module::EditorModule;

#[derive(Debug)]
pub struct RenderState<'a, 'b> {
    pub nav: &'a mut ListModule,
    pub editor: &'a mut EditorModule<'b>,
    pub mode: Mode,
    pub settings: &'a contracts::Settings,
    pub current_branch: Option<&'a str>,
}

pub fn render(
    f: &mut Frame<'_>,
    state: RenderState<'_, '_>,
    toaster: &mut Option<ToastEngine<ToastMessage>>,
) {
    let palette = crate::utils::get_palette(state.settings.theme_name.as_deref());
    
    // Render global background to ensure theme background covers everything
    f.render_widget(Block::default().bg(palette.bg), f.area());

    let is_searching = state.mode == Mode::Search;
    
    // 1. Vertical Split for Header, Main, Search, Footer, Statusline
    let mut v_constraints = vec![
        Constraint::Length(1), // Header
        Constraint::Min(5),    // Main content
    ];
    if is_searching {
        v_constraints.push(Constraint::Length(1));
    }
    v_constraints.push(Constraint::Length(2)); // Footer
    v_constraints.push(Constraint::Length(1)); // Statusline

    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(v_constraints)
        .split(f.area());

    let header_chunk = v_chunks[0];
    let main_chunk = v_chunks[1];
    
    let (search_chunk, footer_chunk, statusline_chunk) = if is_searching {
        (Some(v_chunks[2]), v_chunks[3], v_chunks[4])
    } else {
        (None, v_chunks[2], v_chunks[3])
    };

    header::render(f, header_chunk, state.nav.active_tab, state.settings);

    if let Some(s_chunk) = search_chunk {
        let query = &state.nav.search_query;
        let prefix = "Search: /";
        let text = format!("{}{}", prefix, query);
        let paragraph = Paragraph::new(text).style(Style::default().fg(palette.accent));
        f.render_widget(paragraph, s_chunk);
    }

    // 2. Main Chunk Layout (List + Preview)
    let show_preview = state.mode != Mode::Editor 
        && state.mode != Mode::ConfirmDiscard 
        && state.nav.active_tab != Tab::Settings
        && state.settings.preview_mode != PreviewMode::Hidden;

    let content_chunk;
    let mut preview_chunk = None;

    if show_preview {
        match state.settings.preview_mode {
            PreviewMode::Side => {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(main_chunk);
                content_chunk = chunks[0];
                preview_chunk = Some(chunks[1]);
            }
            PreviewMode::Bottom => {
                // Check if there's enough height
                if main_chunk.height >= 10 {
                    let preview_h = if main_chunk.height > 15 { 10 } else { main_chunk.height - 5 };
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Min(5), Constraint::Length(preview_h)])
                        .split(main_chunk);
                    content_chunk = chunks[0];
                    preview_chunk = Some(chunks[1]);
                } else {
                    content_chunk = main_chunk;
                }
            }
            PreviewMode::Hidden => {
                content_chunk = main_chunk;
            }
        }
    } else {
        content_chunk = main_chunk;
    }

    let mut editor_content_area = None;

    if state.mode == Mode::Editor || state.mode == Mode::ConfirmDiscard || state.mode == Mode::ThemePicker {
        if state.nav.active_tab == Tab::Settings {
            settings::render(
                f, 
                content_chunk, 
                state.settings, 
                state.nav.selected_index, 
                if state.mode == Mode::Editor { Some(&mut state.editor.textarea) } else { None }, 
                &mut state.nav.settings_slash_list_state,
                &mut state.nav.theme_list_state,
                state.mode == Mode::ThemePicker
            );
        } else {
            editor_content_area = Some(editor::render(
                f,
                content_chunk,
                &mut state.editor.textarea,
                &mut state.editor.title_textarea,
                state.editor.title_focused,
                state.nav.active_tab,
                state.settings,
            ));

            if state.mode == Mode::ConfirmDiscard {
                let text = ratatui::text::Text::from("\n  Are you sure you want to discard changes?  \n\n            (y) Yes, (n) No            ");
                let popup = tui_popup::Popup::new(text)
                    .title(" Discard Changes? ")
                    .style(Style::default().bg(palette.accent).fg(palette.bg))
                    .border_style(Style::default().fg(palette.accent));
                f.render_widget(&popup, f.area());
            }
        }
    } else {
        if state.nav.active_tab == Tab::Settings {
            settings::render(f, content_chunk, state.settings, state.nav.selected_index, None, &mut state.nav.settings_slash_list_state, &mut state.nav.theme_list_state, false);
        } else {
            let display_query = &state.nav.search_query;
            let mode_str = match state.mode {
                Mode::Move => "Move",
                Mode::Search => "Search",
                _ => "List",
            };
            list::render(f, content_chunk, state.nav.active_tab, &state.nav.prompts, state.nav.selected_index, mode_str, display_query, state.settings, &mut state.nav.list_state);
            
            if let Some(p_chunk) = preview_chunk {
                let selected_prompt = state.nav.prompts.get(state.nav.selected_index);
                list::render_preview(f, p_chunk, selected_prompt, state.settings);
            }
        }
    }

    if let Some(area) = editor_content_area {
        editor::render_autocomplete(
            f, 
            area, 
            &state.editor.textarea, 
            &state.editor.autocomplete.suggestions, 
            state.editor.autocomplete.index, 
            state.editor.autocomplete.open,
            &mut state.editor.autocomplete.list_state, 
            state.settings
        );
    }

    statusline::render(
        f,
        statusline_chunk,
        &state.nav.current_path,
        state.current_branch,
        state.nav.prompts.len(),
        state.nav.folder_filter,
        state.settings,
    );

    footer::render(
        f,
        footer_chunk,
        match state.mode {
            Mode::List => "List",
            Mode::Editor => "Editor",
            Mode::Move => "Move",
            Mode::Search => "Search",
            Mode::ConfirmDiscard => "Confirm Discard",
            Mode::ThemePicker => "Theme Picker",
        },
        state.nav.active_tab,
        state.nav.prompts.len(),
        state.nav.selected_index,
        !state.editor.autocomplete.suggestions.is_empty(),
        state.settings,
    );

    if let Some(ref mut toaster) = toaster {
        toaster.set_area(f.area());
        f.render_widget(&*toaster, f.area());
    }
}
