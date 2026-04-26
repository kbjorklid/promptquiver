use ratatui::widgets::Paragraph;
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
    } else if mode == "Confirm Discard" {
        " y: Discard | n: Cancel ".to_string()
    } else {
        format!(
            " q: Quit | Tab/Arrows/hl: Tabs | j/k: Nav | s: Stage | a: Add | e: Edit | {}/{} ",
            if prompts_len == 0 { 0 } else { selected_index + 1 },
            prompts_len
        )
    };
    let footer = Paragraph::new(footer_text);
    f.render_widget(footer, area);
}
