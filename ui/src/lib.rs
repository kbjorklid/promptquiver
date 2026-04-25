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

    if mode == "Editor" || mode == "Confirm Discard" {
        if active_tab == Tab::Settings {
            settings::render(f, chunks[1], settings, selected_index, Some(textarea));
        } else {
            editor::render(f, chunks[1], textarea, suggestions, suggestion_index);

            if mode == "Confirm Discard" {
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
        match active_tab {
            Tab::Settings => {
                settings::render(f, chunks[1], settings, selected_index, None);
            }
            _ => {
                let display_query = if !global_search_query.is_empty() {
                    global_search_query
                } else {
                    search_query
                };
                list::render(f, chunks[1], active_tab, prompts, selected_index, mode, display_query);
                
                let selected_prompt = prompts.get(selected_index);
                list::render_preview(f, chunks[2], selected_prompt);
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
