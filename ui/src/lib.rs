use contracts::{Prompt, Tab};
use ratatui::layout::{Layout, Constraint, Direction};
use ratatui::widgets::{Block, Borders, Paragraph, Clear};
use ratatui::style::{Style, Color};
use ratatui::Frame;
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

#[derive(Debug, Clone, Copy)]
pub struct RenderState<'a, 'b> {
    pub active_tab: Tab,
    pub prompts: &'a [Prompt],
    pub selected_index: usize,
    pub mode: &'a str,
    pub textarea: &'a TextArea<'b>,
    pub title_textarea: &'a TextArea<'b>,
    pub title_focused: bool,
    pub current_branch: Option<&'a str>,
    pub current_path: &'a str,
    pub suggestions: &'a [Prompt],
    pub suggestion_index: usize,
    pub search_query: &'a str,
    pub global_search_query: &'a str,
    pub settings: &'a contracts::Settings,
}

pub fn render(
    f: &mut Frame<'_>,
    state: RenderState<'_, '_>,
    toaster: &mut Option<ToastEngine<ToastMessage>>,
) {
    let show_preview = state.mode != "Editor" 
        && state.mode != "Confirm Discard" 
        && state.active_tab != Tab::Settings;

    let available_for_main = f.area().height.saturating_sub(6); // 3 for header, 1 for statusline, 2 for footer
    let mut constraints = vec![Constraint::Length(3)]; // Header

    let content_chunk;
    let mut preview_chunk = None;
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
        constraints.push(Constraint::Length(2)); // Footer (max 2 lines)
        constraints.push(Constraint::Length(1)); // Statusline

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(f.area());

        header_chunk = chunks[0];
        content_chunk = chunks[1];
        preview_chunk = Some(chunks[2]);
        footer_chunk = chunks[3];
        statusline_chunk = chunks[4];
    } else {
        constraints.push(Constraint::Min(5));
        constraints.push(Constraint::Length(2)); // Footer
        constraints.push(Constraint::Length(1)); // Statusline

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(f.area());

        header_chunk = chunks[0];
        content_chunk = chunks[1];
        footer_chunk = chunks[2];
        statusline_chunk = chunks[3];
    }

    header::render(f, header_chunk, state.active_tab);


    if state.mode == "Editor" || state.mode == "Confirm Discard" {
        if state.active_tab == Tab::Settings {
            settings::render(f, content_chunk, state.settings, state.selected_index, Some(state.textarea));
        } else {
            editor::render(
                f,
                content_chunk,
                state.textarea,
                state.title_textarea,
                state.title_focused,
                state.active_tab,
                state.suggestions,
                state.suggestion_index,
            );

            if state.mode == "Confirm Discard" {
                let area = utils::centered_rect(60, 25, f.area());
                f.render_widget(Clear, area);
                let block = Block::default()
                    .title(" Discard Changes? ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow));
                let text = Paragraph::new("\nAre you sure you want to discard changes?\n\n(y) Yes, (n) No")
                    .alignment(ratatui::layout::Alignment::Center)
                    .block(block);
                f.render_widget(text, area);
            }
        }
    } else {
        if state.active_tab == Tab::Settings {
            settings::render(f, content_chunk, state.settings, state.selected_index, None);
        } else {
            let display_query = if state.global_search_query.is_empty() {
                state.search_query
            } else {
                state.global_search_query
            };
            list::render(f, content_chunk, state.active_tab, state.prompts, state.selected_index, state.mode, display_query);
            
            if let Some(p_chunk) = preview_chunk {
                let selected_prompt = state.prompts.get(state.selected_index);
                list::render_preview(f, p_chunk, selected_prompt);
            }
        }
    }

    statusline::render(
        f,
        statusline_chunk,
        state.current_path,
        state.current_branch,
        state.prompts.len(),
    );

    footer::render(
        f,
        footer_chunk,
        state.mode,
        state.active_tab,
        state.prompts.len(),
        state.selected_index,
        !state.suggestions.is_empty(),
    );

    if let Some(ref mut toaster) = toaster {
        toaster.set_area(f.area());
        f.render_widget(&*toaster, f.area());
    }
}
