mod common;
use common::setup_app;
use promptquiver::app::{AppMessage, Mode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[tokio::test]
async fn test_editor_discard_confirmation_flow() {
    let (mut app, _storage, _clipboard, _git) = setup_app();

    // 1. Enter editor
    app.handle_message(AppMessage::EnterEditor(String::new(), None)).await.unwrap();
    assert_eq!(app.mode, Mode::Editor);

    // 2. Type some content (make it dirty)
    let key = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::empty());
    app.handle_message(AppMessage::EditorInput(key)).await.unwrap();
    assert!(app.editor.is_dirty());

    // 3. Press Esc
    let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    promptquiver::handlers::handle_events(&mut app, vec![crossterm::event::Event::Key(esc)]).await;

    // Verify it asks for confirmation
    assert_eq!(app.mode, Mode::ConfirmDiscard, "Should ask for confirmation when dirty");

    // 4. Press 'n' to cancel
    let n_key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::empty());
    promptquiver::handlers::handle_events(&mut app, vec![crossterm::event::Event::Key(n_key)]).await;
    assert_eq!(app.mode, Mode::Editor, "Should stay in editor after cancelling discard");
    assert!(app.editor.is_dirty());

    // 5. Press Esc then 'y' to discard
    promptquiver::handlers::handle_events(&mut app, vec![crossterm::event::Event::Key(esc)]).await;
    assert_eq!(app.mode, Mode::ConfirmDiscard);
    let y_key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty());
    promptquiver::handlers::handle_events(&mut app, vec![crossterm::event::Event::Key(y_key)]).await;
    assert_eq!(app.mode, Mode::List, "Should exit editor after confirming discard");
    assert!(!app.editor.is_dirty());

    // 6. Enter editor again and exit without changes
    app.handle_message(AppMessage::EnterEditor(String::new(), None)).await.unwrap();
    assert_eq!(app.mode, Mode::Editor);
    promptquiver::handlers::handle_events(&mut app, vec![crossterm::event::Event::Key(esc)]).await;
    assert_eq!(app.mode, Mode::List, "Should exit immediately if not dirty");
}
