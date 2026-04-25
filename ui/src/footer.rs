use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use ratatui::layout::Rect;

pub fn render(
    f: &mut Frame<'_>,
    area: Rect,
    mode: &str,
    prompts_len: usize,
    selected_index: usize,
    has_suggestions: bool,
) {
    let footer_text = if mode == "Editor" {
        if has_suggestions {
            " Up/Down: Select | Enter: Complete | Esc: Close ".to_string()
        } else {
            " Ctrl+s: Save | Esc: Cancel ".to_string()
        }
    } else {
        format!(
            " q: Quit | Tab: Next Tab | j/k: Nav | s: Stage | a: Add | e: Edit | Index: {}/{} ",
            if prompts_len == 0 { 0 } else { selected_index + 1 },
            prompts_len
        )
    };
    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, area);
}
