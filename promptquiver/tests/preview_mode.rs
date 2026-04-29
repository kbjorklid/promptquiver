mod common;
use common::setup_app;
use contracts::{Tab, PreviewMode, Storage};
use promptquiver::handlers::handle_key_event;
use crossterm::event::{KeyEvent, KeyCode, KeyModifiers, KeyEventKind, KeyEventState};

#[tokio::test]
async fn test_cycle_preview_mode() {
    let (mut app, storage, _, _) = setup_app();

    app.set_tab(Tab::Prompts);
    app.load_prompts().await.unwrap();

    assert_eq!(app.settings.preview_mode, PreviewMode::Bottom);

    let ctrl_e = KeyEvent {
        code: KeyCode::Char('e'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };

    // 1. Cycle to Side
    handle_key_event(&mut app, ctrl_e).await;
    assert_eq!(app.settings.preview_mode, PreviewMode::Side);
    let saved_settings = storage.get_settings().await.unwrap();
    assert_eq!(saved_settings.preview_mode, PreviewMode::Side);

    // 2. Cycle to Hidden
    handle_key_event(&mut app, ctrl_e).await;
    assert_eq!(app.settings.preview_mode, PreviewMode::Hidden);
    let saved_settings = storage.get_settings().await.unwrap();
    assert_eq!(saved_settings.preview_mode, PreviewMode::Hidden);

    // 3. Cycle back to Bottom
    handle_key_event(&mut app, ctrl_e).await;
    assert_eq!(app.settings.preview_mode, PreviewMode::Bottom);
    let saved_settings = storage.get_settings().await.unwrap();
    assert_eq!(saved_settings.preview_mode, PreviewMode::Bottom);
}
