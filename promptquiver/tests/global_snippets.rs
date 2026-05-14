use contracts::{PromptType, Storage, Tab};
use promptquiver::app::AppMessage;
use uuid::Uuid;

mod common;

#[tokio::test]
async fn test_snippets_are_global_across_projects() {
    let (mut app, storage, _clipboard, _git) = common::setup_app();

    // 1. Setup two projects
    let p1_id = Uuid::new_v4();
    storage
        .save_project(contracts::Project {
            id: p1_id,
            title: "Project 1".to_string(),
            created_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    let p2_id = Uuid::new_v4();
    storage
        .save_project(contracts::Project {
            id: p2_id,
            title: "Project 2".to_string(),
            created_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    // 2. Select Project 1
    app.handle_message(AppMessage::SetProject(Some(p1_id))).await.unwrap();
    assert_eq!(app.nav.projects_manager.active_project_id, Some(p1_id));

    // 3. Create a Snippet while Project 1 is active
    app.handle_message(AppMessage::SetTab(Tab::Snippets)).await.unwrap();
    app.handle_message(AppMessage::EnterEditor("Snippet Text".to_string(), None)).await.unwrap();
    app.editor.title_textarea.insert_str("mysnip");
    app.handle_message(AppMessage::SaveEditor).await.unwrap();

    // 4. Switch to Project 2
    app.handle_message(AppMessage::SetProject(Some(p2_id))).await.unwrap();
    app.handle_message(AppMessage::ReloadPrompts).await.unwrap();

    // 5. Verify Snippet is STILL visible (This should fail if it's associated with P1)
    app.handle_message(AppMessage::SetTab(Tab::Snippets)).await.unwrap();
    assert_eq!(app.nav.prompts.len(), 1, "Snippet should be visible in Project 2");
    assert_eq!(app.nav.prompts[0].name.as_deref(), Some("mysnip"));

    // 6. Check storage directly to ensure it has NO project association
    let prompts = storage.get_prompts(contracts::PromptFilter::default()).await.unwrap();
    let snippet =
        prompts.iter().find(|p| p.r#type == PromptType::Snippet).expect("Snippet not found");
    assert!(snippet.project_id.is_none(), "Snippet should not be associated with a project");
}

#[tokio::test]
async fn test_canned_prompts_are_global_across_branches() {
    let (mut app, storage, _clipboard, _git) = common::setup_app();

    // 1. Set branch to 'main' and enable branch filter
    app.current_branch = Some("main".to_string());
    app.nav.branch_filter = true;
    app.handle_message(AppMessage::ReloadPrompts).await.unwrap();

    // 2. Create a Canned Prompt
    app.handle_message(AppMessage::SetTab(Tab::Canned)).await.unwrap();
    app.handle_message(AppMessage::EnterEditor("Canned Text".to_string(), None)).await.unwrap();
    app.handle_message(AppMessage::SaveEditor).await.unwrap();

    // 3. Switch branch to 'feature'
    app.current_branch = Some("feature".to_string());
    app.handle_message(AppMessage::ReloadPrompts).await.unwrap();

    // 4. Verify Canned Prompt is STILL visible
    app.handle_message(AppMessage::SetTab(Tab::Canned)).await.unwrap();
    assert_eq!(app.nav.prompts.len(), 1, "Canned prompt should be visible on feature branch");

    // 5. Check storage directly
    let prompts = storage.get_prompts(contracts::PromptFilter::default()).await.unwrap();
    let canned = prompts
        .iter()
        .find(|p| p.folder.is_none() && p.r#type == PromptType::Prompt)
        .expect("Canned prompt not found");
    assert!(canned.branch.is_none(), "Canned prompt should not be associated with a branch");
}

#[tokio::test]
async fn test_canned_prompts_are_global_across_projects() {
    let (mut app, storage, _clipboard, _git) = common::setup_app();

    // 1. Setup two projects
    let p1_id = Uuid::new_v4();
    storage
        .save_project(contracts::Project {
            id: p1_id,
            title: "Project 1".to_string(),
            created_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    let p2_id = Uuid::new_v4();
    storage
        .save_project(contracts::Project {
            id: p2_id,
            title: "Project 2".to_string(),
            created_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    // 2. Select Project 1 and enable project filter
    app.handle_message(AppMessage::SetProject(Some(p1_id))).await.unwrap();
    app.nav.project_filter = true;
    app.handle_message(AppMessage::ReloadPrompts).await.unwrap();

    // 3. Create a Canned Prompt while Project 1 is active
    app.handle_message(AppMessage::SetTab(Tab::Canned)).await.unwrap();
    app.handle_message(AppMessage::EnterEditor("Canned Project Text".to_string(), None))
        .await
        .unwrap();
    app.handle_message(AppMessage::SaveEditor).await.unwrap();

    // 4. Switch to Project 2
    app.handle_message(AppMessage::SetProject(Some(p2_id))).await.unwrap();
    app.handle_message(AppMessage::ReloadPrompts).await.unwrap();

    // 5. Verify Canned Prompt is STILL visible
    app.handle_message(AppMessage::SetTab(Tab::Canned)).await.unwrap();
    assert_eq!(app.nav.prompts.len(), 1, "Canned prompt should be visible in Project 2");

    // 6. Check storage directly
    let prompts = storage.get_prompts(contracts::PromptFilter::default()).await.unwrap();
    let canned =
        prompts.iter().find(|p| p.text == "Canned Project Text").expect("Canned prompt not found");
    assert!(canned.project_id.is_none(), "Canned prompt should not be associated with a project");
}
