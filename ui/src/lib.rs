use contracts::{Prompt, Tab};
use ratatui::layout::{Layout, Constraint, Direction};
use ratatui::Frame;
use ratatui_textarea::TextArea;
use ratatui_toaster::{ToastEngine, ToastMessage};

pub mod header;
pub mod list;
pub mod footer;
pub mod editor;
pub mod utils;
pub mod settings;

pub fn render(
    f: &mut Frame<'_>,
    active_tab: Tab,
    prompts: &[Prompt],
    selected_index: usize,
    mode: &str,
    textarea: &TextArea<'_>,
    current_branch: Option<&str>,
    suggestions: &[Prompt],
    suggestion_index: usize,
    toaster: &mut Option<ToastEngine<ToastMessage>>,
    search_query: &str,
    global_search_query: &str,
    settings: &contracts::Settings,
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

    header::render(f, chunks[0], active_tab, current_branch);

    if mode == "Editor" {
        editor::render(f, chunks[1], textarea, suggestions, suggestion_index);
    } else {
        match active_tab {
            Tab::Settings => {
                settings::render(f, chunks[1], settings, selected_index);
            }
            _ => {
                let display_query = if !global_search_query.is_empty() {
                    global_search_query
                } else {
                    search_query
                };
                list::render(f, chunks[1], active_tab, prompts, selected_index, mode, display_query);
            }
        }
    }

    footer::render(
        f,
        chunks[3],
        mode,
        prompts.len(),
        selected_index,
        !suggestions.is_empty(),
    );

    if let Some(ref mut toaster) = toaster {
        f.render_widget(&*toaster, f.area());
    }
}
