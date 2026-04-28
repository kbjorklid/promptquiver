use contracts::{Prompt, Tab};
use ratatui::layout::{Layout, Constraint, Direction};
use ratatui::widgets::{Paragraph, Block};
use ratatui::style::Style;
use ratatui::Frame;
use ratatui::prelude::Stylize;
use ratatui_textarea::TextArea;
use ratatui_toaster::{ToastEngine, ToastMessage};

pub mod header;
pub mod list;
pub mod footer;
pub mod editor;
pub mod utils;
pub mod settings;
pub mod statusline;
pub mod shortcuts;

#[derive(Debug)]
pub struct RenderState<'a, 'b> {
    pub active_tab: Tab,
    pub prompts: &'a [Prompt],
    pub selected_index: usize,
    pub list_state: &'a mut ratatui::widgets::ListState,
    pub settings_slash_list_state: &'a mut ratatui::widgets::ListState,
    pub theme_list_state: &'a mut ratatui::widgets::ListState,
    pub mode: &'a str,
    pub textarea: &'a mut TextArea<'b>,
    pub title_textarea: &'a mut TextArea<'b>,
    pub title_focused: bool,
    pub current_branch: Option<&'a str>,
    pub current_path: &'a str,
    pub suggestions: &'a [Prompt],
    pub suggestion_index: usize,
    pub autocomplete_list_state: &'a mut ratatui::widgets::ListState,
    pub search_query: &'a str,
    pub global_search_query: &'a str,
    pub settings: &'a contracts::Settings,
}

pub fn render(
    f: &mut Frame<'_>,
    state: RenderState<'_, '_>,
    toaster: &mut Option<ToastEngine<ToastMessage>>,
) {
    let palette = crate::utils::get_palette(state.settings.theme_name.as_deref());
    
    // Render global background to ensure theme background covers everything
    f.render_widget(Block::default().bg(palette.bg), f.area());

    let show_preview = state.mode != "Editor" 
        && state.mode != "Confirm Discard" 
        && state.active_tab != Tab::Settings;

    let is_searching = state.mode == "Search" || state.mode == "Global Search";
    
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

    header::render(f, header_chunk, state.active_tab, state.settings);

    let palette = crate::utils::get_palette(state.settings.theme_name.as_deref());

    if let Some(s_chunk) = search_chunk {
        let query = if state.mode == "Global Search" { state.global_search_query } else { state.search_query };
        let prefix = if state.mode == "Global Search" { "Global Search: /" } else { "Search: /" };
        let text = format!("{}{}", prefix, query);
        let paragraph = Paragraph::new(text).style(Style::default().fg(palette.accent));
        f.render_widget(paragraph, s_chunk);
    }


    let mut editor_content_area = None;

    if state.mode == "Editor" || state.mode == "Confirm Discard" || state.mode == "Theme Picker" {
        if state.active_tab == Tab::Settings {
            settings::render(
                f, 
                content_chunk, 
                state.settings, 
                state.selected_index, 
                if state.mode == "Editor" { Some(state.textarea) } else { None }, 
                state.settings_slash_list_state,
                state.theme_list_state,
                state.mode == "Theme Picker"
            );
        } else {
            editor_content_area = Some(editor::render(
                f,
                content_chunk,
                state.textarea,
                state.title_textarea,
                state.title_focused,
                state.active_tab,
                state.settings,
            ));

            if state.mode == "Confirm Discard" {
                let text = ratatui::text::Text::from("\n  Are you sure you want to discard changes?  \n\n            (y) Yes, (n) No            ");
                let popup = tui_popup::Popup::new(text)
                    .title(" Discard Changes? ")
                    .style(Style::default().bg(palette.accent).fg(palette.bg))
                    .border_style(Style::default().fg(palette.accent));
                f.render_widget(&popup, f.area());
            }
        }
    } else {
        if state.active_tab == Tab::Settings {
            settings::render(f, content_chunk, state.settings, state.selected_index, None, state.settings_slash_list_state, state.theme_list_state, false);
        } else {
            let display_query = if state.global_search_query.is_empty() {
                state.search_query
            } else {
                state.global_search_query
            };
            list::render(f, content_chunk, state.active_tab, state.prompts, state.selected_index, state.mode, display_query, state.settings, state.list_state);
            
            if let Some(p_chunk) = preview_chunk {
                let selected_prompt = state.prompts.get(state.selected_index);
                list::render_preview(f, p_chunk, selected_prompt, state.settings);
            }
        }
    }

    if let Some(area) = editor_content_area {
        editor::render_autocomplete(
            f, 
            area, 
            state.textarea, 
            state.suggestions, 
            state.suggestion_index, 
            state.autocomplete_list_state, 
            state.settings
        );
    }

    statusline::render(
        f,
        statusline_chunk,
        state.current_path,
        state.current_branch,
        state.prompts.len(),
        state.settings,
    );

    footer::render(
        f,
        footer_chunk,
        state.mode,
        state.active_tab,
        state.prompts.len(),
        state.selected_index,
        !state.suggestions.is_empty(),
        state.settings,
    );

    if let Some(ref mut toaster) = toaster {
        toaster.set_area(f.area());
        f.render_widget(&*toaster, f.area());
    }
}
