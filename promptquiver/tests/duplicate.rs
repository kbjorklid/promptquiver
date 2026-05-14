mod common;
use common::setup_app;
use contracts::{PromptFilter, Storage, Tab};

#[tokio::test]
async fn test_duplicate_prompt() {
    let (mut app, storage, _, _) = setup_app();

    let p1 = contracts::Prompt::new(
        "Original Prompt".to_string(),
        contracts::PromptType::Prompt,
        Some(common::TEST_PATH.to_string()),
        None,
        Some("Name".to_string()),
        None,
    );
    storage.save_prompt(p1.clone()).await.unwrap();

    app.load_prompts().await.unwrap();
    assert_eq!(app.nav.prompts.len(), 1);
    assert_eq!(app.nav.selected_index, 0);

    // Call duplicate (this method doesn't exist yet, so it will fail to compile)
    app.duplicate_selected().await.unwrap();

    // Verify in-memory state
    assert_eq!(app.nav.prompts.len(), 2);
    assert_eq!(app.nav.selected_index, 1);
    assert_eq!(app.nav.prompts[0].text, "Original Prompt");
    assert_eq!(app.nav.prompts[1].text, "Original Prompt");
    assert_eq!(app.nav.prompts[1].name, Some("Name".to_string()));
    assert_ne!(app.nav.prompts[0].id, app.nav.prompts[1].id);
    assert!(!app.nav.prompts[1].staged);
    assert!(!app.nav.prompts[1].last_copied);

    // Verify persistence
    let stored = storage
        .get_prompts(PromptFilter {
            folder: Some(common::TEST_PATH.to_string()),
            tab: Some(Tab::Prompts),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(stored.len(), 2);
}
