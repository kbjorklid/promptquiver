mod common;
use common::setup_app;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ui::types::AppMessage;

#[tokio::test]
async fn test_positional_insertion() {
    let (mut app, _, _, _) = setup_app();

    // 1. Add first item
    app.enter_editor("Item 1".to_string(), None);
    app.save_editor().await.unwrap();
    assert_eq!(app.nav.prompts.len(), 1);

    // 2. Add second item (default 'a' currently adds at top)
    app.enter_editor("Item 2".to_string(), None);
    app.save_editor().await.unwrap();
    assert_eq!(app.nav.prompts.len(), 2);
    // Current behavior: newest is at index 0
    assert_eq!(app.nav.prompts[0].text, "Item 2");
    assert_eq!(app.nav.prompts[1].text, "Item 1");

    // Now let's try 'i' which uses EnterEditorBefore
    // Select Item 1 (index 1)
    app.nav.selected_index = 1;
    app.handle_message(AppMessage::EnterEditorBefore("Item 1.5".to_string(), 1)).await.unwrap();
    app.save_editor().await.unwrap();

    // It SHOULD be at index 1: [Item 2, Item 1.5, Item 1]
    assert_eq!(app.nav.prompts.len(), 3);
    assert_eq!(app.nav.prompts[1].text, "Item 1.5", "Item should be inserted at index 1");
    assert_eq!(app.nav.prompts[0].text, "Item 2");
    assert_eq!(app.nav.prompts[2].text, "Item 1");
}

#[tokio::test]
async fn test_positional_keys() {
    let (mut app, _, _, _) = setup_app();

    // Add some items first
    app.enter_editor("Item 1".to_string(), None);
    app.save_editor().await.unwrap();
    app.enter_editor("Item 2".to_string(), None);
    app.save_editor().await.unwrap();
    // Initially [Item 2, Item 1] (newest at top)

    let k = |code| KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };
    let shift = |code| KeyEvent {
        code,
        modifiers: KeyModifiers::SHIFT,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };

    // 1. Test 'i' (above selected)
    // Select Item 1 (index 1)
    app.nav.selected_index = 1;
    promptquiver::handlers::handle_key_event(&mut app, k(KeyCode::Char('i'))).await;
    app.editor.textarea.insert_str("Item 1.25");
    app.save_editor().await.unwrap();
    // [Item 2, Item 1.25, Item 1]
    assert_eq!(app.nav.prompts[1].text, "Item 1.25");

    // 2. Test 'a' (below selected)
    // Select Item 2 (index 0)
    app.nav.selected_index = 0;
    promptquiver::handlers::handle_key_event(&mut app, k(KeyCode::Char('a'))).await;
    app.editor.textarea.insert_str("Item 2.5");
    app.save_editor().await.unwrap();
    // [Item 2, Item 2.5, Item 1.25, Item 1]
    assert_eq!(app.nav.prompts[1].text, "Item 2.5");

    // 3. Test 'I' (first item)
    app.nav.selected_index = 3; // Select last item
    promptquiver::handlers::handle_key_event(&mut app, shift(KeyCode::Char('I'))).await;
    app.editor.textarea.insert_str("Item First");
    app.save_editor().await.unwrap();
    // [Item First, Item 2, Item 2.5, Item 1.25, Item 1]
    assert_eq!(app.nav.prompts[0].text, "Item First");

    // 4. Test 'A' (last item)
    app.nav.selected_index = 0; // Select first item
    promptquiver::handlers::handle_key_event(&mut app, shift(KeyCode::Char('A'))).await;
    app.editor.textarea.insert_str("Item Last");
    app.save_editor().await.unwrap();
    // [Item First, Item 2, Item 2.5, Item 1.25, Item 1, Item Last]
    assert_eq!(app.nav.prompts.last().unwrap().text, "Item Last");
}
