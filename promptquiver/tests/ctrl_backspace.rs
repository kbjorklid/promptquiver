mod common;
use common::setup_app;
use crossterm::event::{KeyEvent, KeyCode, KeyModifiers, KeyEventKind, KeyEventState};

#[tokio::test]
async fn test_ctrl_backspace_deletes_word() {
    let (mut app, _, _, _) = setup_app();
    
    // Enter editor with some text
    app.enter_editor("hello world".to_string(), None);
    app.editor.textarea.move_cursor(ratatui_textarea::CursorMove::End);
    
    // Initial state: "hello world" |
    assert_eq!(app.editor.textarea.lines()[0], "hello world");
    
    // Simulate Ctrl+Backspace (as KeyCode::Backspace + CONTROL)
    let event = KeyEvent {
        code: KeyCode::Backspace,
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };

    // Use real application logic
    app.handle_message(ui::AppMessage::EditorInput(event)).await.unwrap();

    // If it works, it should be "hello "
    // If it doesn't work, it will still be "hello world"
    assert_eq!(app.editor.textarea.lines()[0], "hello ", "Ctrl+Backspace should delete the word 'world'");
}

#[tokio::test]
async fn test_ctrl_backspace_char_7f_deletes_word() {
    let (mut app, _, _, _) = setup_app();
    
    // Enter editor with some text
    app.enter_editor("hello world".to_string(), None);
    app.editor.textarea.move_cursor(ratatui_textarea::CursorMove::End);
    
    // Simulate Ctrl+Backspace (as KeyCode::Char('\u{7f}'))
    // Some Windows terminals send this for Ctrl+Backspace
    let event = KeyEvent {
        code: KeyCode::Char('\u{7f}'),
        modifiers: KeyModifiers::empty(), // Sometimes it comes without modifiers if the char itself is 7F
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    };

    // Use real application logic
    app.handle_message(ui::AppMessage::EditorInput(event)).await.unwrap();

    assert_eq!(app.editor.textarea.lines()[0], "hello ", "KeyCode::Char('\\u{{7f}}') should delete the word 'world'");
}

