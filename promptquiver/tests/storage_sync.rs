mod common;
use common::setup_app;
use contracts::{Prompt, Storage};
use infra::SqliteStorage;
use std::sync::Arc;
use tempfile::tempdir;

#[tokio::test]
async fn test_order_persistence() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test_order.db");
    let storage = Arc::new(SqliteStorage::new(db_path));

    let (mut app, _, _, _) = setup_app();
    app.storage = storage.clone();
    let test_path = app.nav.current_project_path();

    // Create 3 prompts
    let p1 = Prompt::new(
        "P1".to_string(),
        contracts::PromptType::Prompt,
        Some(test_path.clone()),
        None,
        None,
        None,
    );
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    let p2 = Prompt::new(
        "P2".to_string(),
        contracts::PromptType::Prompt,
        Some(test_path.clone()),
        None,
        None,
        None,
    );
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    let p3 = Prompt::new(
        "P3".to_string(),
        contracts::PromptType::Prompt,
        Some(test_path.clone()),
        None,
        None,
        None,
    );

    storage.save_prompt(p1).await.unwrap();
    storage.save_prompt(p2).await.unwrap();
    storage.save_prompt(p3).await.unwrap();

    app.load_prompts().await.unwrap();
    // Default order should be P3, P2, P1 (created_at DESC because order_index is 0)
    assert_eq!(app.nav.prompts[0].text, "P3");
    assert_eq!(app.nav.prompts[1].text, "P2");
    assert_eq!(app.nav.prompts[2].text, "P1");

    // Move P3 down (Swap P3 and P2)
    app.handle_message(ui::AppMessage::MoveItemDown).await.unwrap();
    assert_eq!(app.nav.prompts[0].text, "P2");
    assert_eq!(app.nav.prompts[1].text, "P3");
    assert_eq!(app.nav.prompts[2].text, "P1");

    // Reload prompts - this simulates a background sync
    app.load_prompts().await.unwrap();

    // Verify order is preserved
    assert_eq!(app.nav.prompts[0].text, "P2", "Order should be preserved after reload");
    assert_eq!(app.nav.prompts[1].text, "P3");
    assert_eq!(app.nav.prompts[2].text, "P1");
}

#[tokio::test]
async fn test_sync_efficiency() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test_sync.db");
    let storage = Arc::new(SqliteStorage::new(db_path));

    let (mut app, _, _, _) = setup_app();
    app.storage = storage.clone();

    // Initial load
    app.load_prompts().await.unwrap();
    let v1 = storage.get_data_version().await.unwrap();

    // Consecutive loads with no change should NOT increment version
    app.load_prompts().await.unwrap();
    let v2 = storage.get_data_version().await.unwrap();
    assert_eq!(v1, v2, "Version should not increment on no-op load");

    // Batch save with changes should increment version exactly ONCE
    let prompts = vec![
        Prompt::new(
            "X".to_string(),
            contracts::PromptType::Prompt,
            Some("path".to_string()),
            None,
            None,
            None,
        ),
        Prompt::new(
            "Y".to_string(),
            contracts::PromptType::Prompt,
            Some("path".to_string()),
            None,
            None,
            None,
        ),
    ];
    storage.save_prompts(prompts).await.unwrap();
    let v3 = storage.get_data_version().await.unwrap();
    assert_eq!(v2 + 1, v3, "Version should increment once for a batch save");

    // Batch save with SAME data should NOT increment version
    let filter = contracts::PromptFilter { folder: Some("path".to_string()), ..Default::default() };
    let current_prompts = storage.get_prompts(filter).await.unwrap();
    storage.save_prompts(current_prompts).await.unwrap();
    let v4 = storage.get_data_version().await.unwrap();
    assert_eq!(v3, v4, "Version should not increment when batch saving identical data");
}
