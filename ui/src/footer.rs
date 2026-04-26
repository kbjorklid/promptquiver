use ratatui::widgets::Paragraph;
use ratatui::Frame;
use ratatui::layout::Rect;

pub fn render(
    f: &mut Frame<'_>,
    area: Rect,
    mode: &str,
    _prompts_len: usize,
    _selected_index: usize,
    has_suggestions: bool,
) {
    let footer_text = if mode == "Editor" {
        if has_suggestions {
            " Up/Down: Select | Enter: Complete | Esc: Close ".to_string()
        } else {
            " Ctrl+s: Save | Esc: Cancel ".to_string()
        }
    } else if mode == "Confirm Discard" {
        " y: Discard | n: Cancel ".to_string()
    } else {
        " q: Quit | Tab/Arrows/hl: Tabs | j/k: Nav | s: Stage | a: Add | e: Edit ".to_string()
    };
    let footer = Paragraph::new(footer_text);
    f.render_widget(footer, area);
}
