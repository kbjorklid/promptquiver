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

#[derive(Debug, Clone, Copy)]
pub struct RenderState<'a, 'b> {
    pub active_tab: Tab,
    pub prompts: &'a [Prompt],
    pub selected_index: usize,
    pub mode: &'a str,
    pub textarea: &'a TextArea<'b>,
    pub current_branch: Option<&'a str>,
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
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Content
            Constraint::Length(10), // Preview
            Constraint::Length(3), // Footer
        ])
        .split(f.area());

    header::render(f, chunks[0], state.active_tab, state.current_branch);

    if state.mode == "Editor" || state.mode == "Confirm Discard" {
        if state.active_tab == Tab::Settings {
            settings::render(f, chunks[1], state.settings, state.selected_index, Some(state.textarea));
        } else {
            editor::render(f, chunks[1], state.textarea, state.suggestions, state.suggestion_index);

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
            settings::render(f, chunks[1], state.settings, state.selected_index, None);
        } else {
            let display_query = if state.global_search_query.is_empty() {
                state.search_query
            } else {
                state.global_search_query
            };
            list::render(f, chunks[1], state.active_tab, state.prompts, state.selected_index, state.mode, display_query);
            
            let selected_prompt = state.prompts.get(state.selected_index);
            list::render_preview(f, chunks[2], selected_prompt);
        }
    }

    footer::render(
        f,
        chunks[3],
        state.mode,
        state.prompts.len(),
        state.selected_index,
        !state.suggestions.is_empty(),
    );

    if let Some(ref mut toaster) = toaster {
        f.render_widget(&*toaster, f.area());
    }
}
