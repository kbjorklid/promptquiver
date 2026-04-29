mod common;
use common::setup_app;
use crossterm::event::{KeyEvent, KeyCode, KeyModifiers, KeyEventKind, KeyEventState, Event};

#[tokio::test]
async fn test_repro_at_symbol_typing() {
    let (mut app, _, _, _) = setup_app();
    
    // Enter editor
    app.enter_editor("".to_string(), None);
    
    // Simulate typing '@' with Shift
    let event = Event::Key(KeyEvent {
        code: KeyCode::Char('@'),
        modifiers: KeyModifiers::SHIFT,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    });

    if let Event::Key(key) = event {
        app.editor.textarea.input(key);
    }

    // Verify get_current_autocomplete_query
    let query_state = app.editor.get_current_autocomplete_query();
    assert!(query_state.is_some());
    let (trigger, query) = query_state.unwrap();
    assert_eq!(trigger, "@");
    assert_eq!(query, "");

    // Check if it was typed
    assert_eq!(app.editor.textarea.lines()[0], "@");
}

#[tokio::test]
async fn test_at_symbol_altgr_typing() {
    let (mut app, _, _, _) = setup_app();
    
    // Enter editor
    app.enter_editor("".to_string(), None);
    
    // Simulate typing '@' with Ctrl+Alt (AltGr on Windows)
    let event = Event::Key(KeyEvent {
        code: KeyCode::Char('@'),
        modifiers: KeyModifiers::CONTROL | KeyModifiers::ALT,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    });

    // Use the logic from main.rs
    if let Event::Key(key) = event {
        if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat {
            // This represents the fallback logic in main.rs
            if !app.editor.textarea.input(event) {
                if let KeyCode::Char(c) = key.code {
                    app.editor.textarea.input(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()));
                }
            }
        }
    }

    // Check if it was typed
    assert_eq!(app.editor.textarea.lines()[0], "@", "AltGr character should be typed using fallback logic");
}

