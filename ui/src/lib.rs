use contracts::{Tab};
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

    let show_preview = state.mode != Mode::Editor 
        && state.mode != Mode::ConfirmDiscard 
        && state.nav.active_tab != Tab::Settings;

    let is_searching = state.mode == Mode::Search || state.mode == Mode::GlobalSearch;
    
    let mut available_for_main = f.area().height.saturating_sub(4); // 1 for header, 1 for statusline, 2 for footer
    if is_searching {
        available_for_main = available_for_main.saturating_sub(1);
    }

    let mut constraints = vec![Constraint::Length(1)]; // Header

    let content_chunk;
    let mut preview_chunk = None;
    let mut search_chunk = None;
    let statusline_chunk;
    let footer_chunk;
    let header_chunk;

    if show_preview && available_for_main >= 10 {
        let preview_h = if available_for_main > 15 {
            10
        } else {
            available_for_main - 5
        };
        constraints.push(Constraint::Min(5));
        constraints.push(Constraint::Length(preview_h));
        if is_searching {
            constraints.push(Constraint::Length(1));
        }
        constraints.push(Constraint::Length(2)); // Footer (max 2 lines)
        constraints.push(Constraint::Length(1)); // Statusline

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(f.area());

        header_chunk = chunks[0];
        content_chunk = chunks[1];
        preview_chunk = Some(chunks[2]);
        if is_searching {
            search_chunk = Some(chunks[3]);
            footer_chunk = chunks[4];
            statusline_chunk = chunks[5];
        } else {
            footer_chunk = chunks[3];
            statusline_chunk = chunks[4];
        }
    } else {
        constraints.push(Constraint::Min(5));
        if is_searching {
            constraints.push(Constraint::Length(1));
        }
        constraints.push(Constraint::Length(2)); // Footer
        constraints.push(Constraint::Length(1)); // Statusline

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(f.area());

        header_chunk = chunks[0];
        content_chunk = chunks[1];
        if is_searching {
            search_chunk = Some(chunks[2]);
            footer_chunk = chunks[3];
            statusline_chunk = chunks[4];
        } else {
            footer_chunk = chunks[2];
            statusline_chunk = chunks[3];
        }
    }

    header::render(f, header_chunk, state.nav.active_tab, state.settings);

    let palette = crate::utils::get_palette(state.settings.theme_name.as_deref());

    if let Some(s_chunk) = search_chunk {
        let query = if state.mode == Mode::GlobalSearch { &state.nav.global_search_query } else { &state.nav.search_query };
        let prefix = if state.mode == Mode::GlobalSearch { "Global Search: /" } else { "Search: /" };
        let text = format!("{}{}", prefix, query);
        let paragraph = Paragraph::new(text).style(Style::default().fg(palette.accent));
        f.render_widget(paragraph, s_chunk);
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
            let display_query = if state.nav.global_search_query.is_empty() {
                &state.nav.search_query
            } else {
                &state.nav.global_search_query
            };
            let mode_str = match state.mode {
                Mode::Move => "Move",
                Mode::Search => "Search",
                Mode::GlobalSearch => "Global Search",
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
            Mode::GlobalSearch => "Global Search",
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
