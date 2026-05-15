mod common;
use common::setup_app;
use contracts::Storage;
use tokio::sync::mpsc;
use ui::types::AppMessage;
use uuid::Uuid;

/// Step 3: saving an untitled prompt queues an AI title request.
#[tokio::test]
async fn test_save_untitled_prompt_queues_ai_request() {
    let (mut app, _storage, _, _) = setup_app();

    // Enable AI auto-titling
    app.settings.ai_enabled = true;
    app.settings.ai_auto_title = true;
    app.settings.ai_model_path = Some("google/gemma-4-E2B-it".to_string());

    // Wire up a test channel to observe what gets queued
    let (tx, mut rx) = mpsc::channel::<(Uuid, String)>(10);
    app.ai_title_tx = Some(tx);

    // Enter editor with text that has no title line (no "-- Title\n\n" header)
    app.enter_editor("This is my prompt text without a title".to_string(), None);
    app.save_editor().await.unwrap();

    // Expect a request was queued
    let queued = rx.try_recv();
    assert!(queued.is_ok(), "expected an AI title request to be queued");
    let (id, text) = queued.unwrap();
    assert_eq!(id, app.nav.prompts[0].id);
    assert!(text.contains("This is my prompt text without a title"));
}

/// Saving a prompt that already has a title does NOT queue an AI request.
#[tokio::test]
async fn test_save_titled_prompt_skips_ai_request() {
    let (mut app, _, _, _) = setup_app();

    app.settings.ai_enabled = true;
    app.settings.ai_auto_title = true;
    app.settings.ai_model_path = Some("google/gemma-4-E2B-it".to_string());

    let (tx, mut rx) = mpsc::channel::<(Uuid, String)>(10);
    app.ai_title_tx = Some(tx);

    // Text with a title line: "-- My Title\n\n<body>"
    app.enter_editor("-- My Title\n\nSome prompt body".to_string(), None);
    app.save_editor().await.unwrap();

    // No request should be queued because the prompt has a title
    assert!(rx.try_recv().is_err(), "no AI request should be queued for a titled prompt");
}

/// Step 4: `TitleGenerated` updates storage if prompt still has no name.
#[tokio::test]
async fn test_title_generated_updates_storage() {
    let (mut app, storage, _, _) = setup_app();

    // Save an untitled prompt
    app.enter_editor("Untitled body text".to_string(), None);
    app.save_editor().await.unwrap();
    let id = app.nav.prompts[0].id;

    // Simulate AI returning a title
    app.handle_message(AppMessage::TitleGenerated(id, "Generated Title".to_string()))
        .await
        .unwrap();

    // Prompt in storage should now have a name
    let saved = storage.get_prompt(id).await.unwrap().unwrap();
    assert_eq!(saved.name.as_deref(), Some("Generated Title"));
}

/// AI disabled: saving an untitled prompt does NOT queue a request.
#[tokio::test]
async fn test_ai_disabled_no_queue() {
    let (mut app, _, _, _) = setup_app();

    app.settings.ai_enabled = false;
    app.settings.ai_auto_title = true;
    app.settings.ai_model_path = Some("google/gemma-4-E2B-it".to_string());

    let (tx, mut rx) = mpsc::channel::<(Uuid, String)>(10);
    app.ai_title_tx = Some(tx);

    app.enter_editor("Untitled prompt text".to_string(), None);
    app.save_editor().await.unwrap();

    assert!(rx.try_recv().is_err(), "ai_enabled=false must not queue a request");
}

/// Auto-title off: saving an untitled prompt does NOT queue a request.
#[tokio::test]
async fn test_ai_auto_title_off_no_queue() {
    let (mut app, _, _, _) = setup_app();

    app.settings.ai_enabled = true;
    app.settings.ai_auto_title = false;
    app.settings.ai_model_path = Some("google/gemma-4-E2B-it".to_string());

    let (tx, mut rx) = mpsc::channel::<(Uuid, String)>(10);
    app.ai_title_tx = Some(tx);

    app.enter_editor("Untitled prompt text".to_string(), None);
    app.save_editor().await.unwrap();

    assert!(rx.try_recv().is_err(), "ai_auto_title=false must not queue a request");
}

/// No model path: saving an untitled prompt does NOT queue a request.
#[tokio::test]
async fn test_ai_model_path_none_no_queue() {
    let (mut app, _, _, _) = setup_app();

    app.settings.ai_enabled = true;
    app.settings.ai_auto_title = true;
    app.settings.ai_model_path = None;

    let (tx, mut rx) = mpsc::channel::<(Uuid, String)>(10);
    app.ai_title_tx = Some(tx);

    app.enter_editor("Untitled prompt text".to_string(), None);
    app.save_editor().await.unwrap();

    assert!(rx.try_recv().is_err(), "no model path must not queue a request");
}

/// No AI channel wired: saving an untitled prompt does not panic and queues nothing.
#[tokio::test]
async fn test_no_ai_channel_no_panic() {
    let (mut app, _, _, _) = setup_app();

    app.settings.ai_enabled = true;
    app.settings.ai_auto_title = true;
    app.settings.ai_model_path = Some("google/gemma-4-E2B-it".to_string());
    // ai_title_tx intentionally left as None

    app.enter_editor("Untitled prompt text".to_string(), None);
    app.save_editor().await.unwrap(); // must not panic
}

/// Deduplication: re-saving an in-flight prompt does not queue a second request.
#[tokio::test]
async fn test_no_double_queue_while_ai_in_flight() {
    let (mut app, _, _, _) = setup_app();

    app.settings.ai_enabled = true;
    app.settings.ai_auto_title = true;
    app.settings.ai_model_path = Some("google/gemma-4-E2B-it".to_string());

    let (tx, mut rx) = mpsc::channel::<(Uuid, String)>(10);
    app.ai_title_tx = Some(tx);

    // First save — should queue
    app.enter_editor("Untitled prompt text".to_string(), None);
    app.save_editor().await.unwrap();
    assert!(rx.try_recv().is_ok(), "first save should queue a request");

    let saved_id = app.nav.prompts[0].id;

    // Re-edit the same prompt without consuming the AI result
    app.enter_editor("Untitled prompt text".to_string(), Some(saved_id));
    app.save_editor().await.unwrap();

    // Second save must not queue again while first is still in-flight
    assert!(rx.try_recv().is_err(), "second save must not double-queue");
}

/// `TitleGenerated` for a prompt that no longer exists is a silent no-op.
#[tokio::test]
async fn test_title_generated_missing_prompt_is_noop() {
    let (mut app, _, _, _) = setup_app();
    let ghost_id = Uuid::new_v4();

    // Should not error, should not panic
    app.handle_message(AppMessage::TitleGenerated(ghost_id, "Ghost Title".to_string()))
        .await
        .unwrap();

    // ai_generating_title_for should not retain the ghost ID
    assert!(!app.ai_generating_title_for.contains(&ghost_id));
}

/// `TitleGenerated` clears the in-flight marker even when the title is applied.
#[tokio::test]
async fn test_title_generated_clears_in_flight_marker() {
    let (mut app, _, _, _) = setup_app();

    app.settings.ai_enabled = true;
    app.settings.ai_auto_title = true;
    app.settings.ai_model_path = Some("google/gemma-4-E2B-it".to_string());

    let (tx, _rx) = mpsc::channel::<(Uuid, String)>(10);
    app.ai_title_tx = Some(tx);

    app.enter_editor("Untitled body".to_string(), None);
    app.save_editor().await.unwrap();
    let id = app.nav.prompts[0].id;

    assert!(app.ai_generating_title_for.contains(&id), "should be marked in-flight after save");

    app.handle_message(AppMessage::TitleGenerated(id, "Final Title".to_string()))
        .await
        .unwrap();

    assert!(!app.ai_generating_title_for.contains(&id), "in-flight marker should be cleared after title applied");
}

/// `TitleGenerated` is a no-op when the prompt already has a name.
#[tokio::test]
async fn test_title_generated_no_op_if_already_named() {
    let (mut app, storage, _, _) = setup_app();

    // Save a titled prompt
    app.enter_editor("-- Existing Title\n\nBody".to_string(), None);
    app.save_editor().await.unwrap();
    let id = app.nav.prompts[0].id;

    // Prompt already has a name from the title line
    let before = storage.get_prompt(id).await.unwrap().unwrap();
    assert!(before.name.is_some(), "should have name from title line");

    // AI sends a title — should be ignored
    app.handle_message(AppMessage::TitleGenerated(id, "AI Title".to_string()))
        .await
        .unwrap();

    let after = storage.get_prompt(id).await.unwrap().unwrap();
    assert_eq!(after.name, before.name, "name should not change");
}
