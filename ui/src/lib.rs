use contracts::{Prompt, Tab};
use ratatui::layout::{Layout, Constraint, Direction};
use ratatui::Frame;
use ratatui_textarea::TextArea;

pub mod header;
pub mod list;
pub mod footer;
pub mod editor;
pub mod utils;

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
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header/Tabs
            Constraint::Min(0),    // Main area
            Constraint::Length(3), // Footer
        ])
        .split(f.area());
    
    header::render(f, chunks[0], active_tab, current_branch);
    
    list::render(f, chunks[1], active_tab, prompts, selected_index);

    footer::render(
        f,
        chunks[2],
        mode,
        prompts.len(),
        selected_index,
        !suggestions.is_empty(),
    );

    if mode == "Editor" {
        editor::render(f, textarea, suggestions, suggestion_index);
    }
}
