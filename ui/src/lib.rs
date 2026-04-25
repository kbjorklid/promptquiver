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
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Atlas Branding
            Constraint::Length(3), // Tabs
            Constraint::Min(0),    // Main area
            Constraint::Length(3), // Footer
        ])
        .split(f.area());
    
    header::render_branding(f, chunks[0]);
    header::render_tabs(f, chunks[1], active_tab, current_branch);
    
    list::render(f, chunks[2], active_tab, prompts, selected_index);

    footer::render(
        f,
        chunks[3],
        mode,
        prompts.len(),
        selected_index,
        !suggestions.is_empty(),
    );

    if mode == "Editor" {
        editor::render(f, textarea, suggestions, suggestion_index);
    }

    if let Some(ref mut toaster) = toaster {
        f.render_widget(&*toaster, f.area());
    }
}
