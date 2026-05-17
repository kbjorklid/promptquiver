mod common;
use common::{setup_app, TEST_PATH};
use contracts::{PromptFilter, PromptType, Storage, Tab};
use promptquiver::app::AppMessage;

#[tokio::test]
async fn test_apply_metadata_edit_updates_folder() {
    let (mut app, storage, _, _) = setup_app();

    let prompt = contracts::Prompt::new(
        "test prompt".to_string(),
        PromptType::Prompt,
        Some("old_folder".to_string()),
        Some("old_branch".to_string()),
        None,
        None,
    );
    storage.save_prompt(prompt.clone()).await.unwrap();
    app.load_prompts().await.unwrap();
    assert!(!app.nav.prompts.is_empty(), "prompt should be loaded");

    app.handle_message(AppMessage::ApplyMetadataEdit {
        use_current_folder: true,
        use_current_branch: false,
        project_id: None,
    })
    .await
    .unwrap();

    let filter = PromptFilter { tab: Some(Tab::Prompts), ..Default::default() };
    let prompts = storage.get_prompts(filter).await.unwrap();
    assert_eq!(
        prompts[0].folder.as_deref(),
        Some(TEST_PATH),
        "folder should be updated to current path"
    );
    assert_eq!(prompts[0].branch.as_deref(), Some("old_branch"), "branch should not change");
}

#[tokio::test]
async fn test_apply_metadata_edit_updates_branch() {
    let (mut app, storage, _, _) = setup_app();

    let prompt = contracts::Prompt::new(
        "test prompt".to_string(),
        PromptType::Prompt,
        Some("old_folder".to_string()),
        Some("old_branch".to_string()),
        None,
        None,
    );
    storage.save_prompt(prompt.clone()).await.unwrap();
    app.current_branch = Some("new_branch".to_string());
    app.load_prompts().await.unwrap();

    app.handle_message(AppMessage::ApplyMetadataEdit {
        use_current_folder: false,
        use_current_branch: true,
        project_id: None,
    })
    .await
    .unwrap();

    let filter = PromptFilter { tab: Some(Tab::Prompts), ..Default::default() };
    let prompts = storage.get_prompts(filter).await.unwrap();
    assert_eq!(
        prompts[0].branch.as_deref(),
        Some("new_branch"),
        "branch should be updated to current branch"
    );
    assert_eq!(prompts[0].folder.as_deref(), Some("old_folder"), "folder should not change");
}

#[tokio::test]
async fn test_apply_metadata_edit_updates_project() {
    let (mut app, storage, _, _) = setup_app();

    let project = contracts::Project {
        id: uuid::Uuid::new_v4(),
        title: "My Project".to_string(),
        created_at: chrono::Utc::now(),
    };
    storage.save_project(project.clone()).await.unwrap();
    app.nav.projects_manager.projects = vec![project.clone()];

    let prompt = contracts::Prompt::new(
        "test prompt".to_string(),
        PromptType::Prompt,
        Some("old_folder".to_string()),
        Some("old_branch".to_string()),
        None,
        None,
    );
    storage.save_prompt(prompt.clone()).await.unwrap();
    app.load_prompts().await.unwrap();

    app.handle_message(AppMessage::ApplyMetadataEdit {
        use_current_folder: false,
        use_current_branch: false,
        project_id: Some(project.id),
    })
    .await
    .unwrap();

    let filter = PromptFilter { tab: Some(Tab::Prompts), ..Default::default() };
    let prompts = storage.get_prompts(filter).await.unwrap();
    assert_eq!(prompts[0].project_id, Some(project.id), "project should be updated");
}

#[tokio::test]
async fn test_apply_metadata_edit_no_op_when_all_false() {
    let (mut app, storage, _, _) = setup_app();

    let prompt = contracts::Prompt::new(
        "test prompt".to_string(),
        PromptType::Prompt,
        Some("old_folder".to_string()),
        Some("old_branch".to_string()),
        None,
        None,
    );
    storage.save_prompt(prompt.clone()).await.unwrap();
    app.load_prompts().await.unwrap();

    app.handle_message(AppMessage::ApplyMetadataEdit {
        use_current_folder: false,
        use_current_branch: false,
        project_id: None,
    })
    .await
    .unwrap();

    let filter = PromptFilter { tab: Some(Tab::Prompts), ..Default::default() };
    let prompts = storage.get_prompts(filter).await.unwrap();
    assert_eq!(prompts[0].folder.as_deref(), Some("old_folder"), "folder should not change");
    assert_eq!(prompts[0].branch.as_deref(), Some("old_branch"), "branch should not change");
    assert_eq!(prompts[0].project_id, None, "project should stay None");
}

#[tokio::test]
async fn test_enter_metadata_editor_opens_mode() {
    let (mut app, storage, _, _) = setup_app();

    let prompt = contracts::Prompt::new(
        "test prompt".to_string(),
        PromptType::Prompt,
        Some("some_folder".to_string()),
        Some("some_branch".to_string()),
        None,
        None,
    );
    storage.save_prompt(prompt.clone()).await.unwrap();
    app.load_prompts().await.unwrap();

    app.handle_message(AppMessage::EnterMetadataEditor).await.unwrap();

    assert_eq!(app.mode, promptquiver::app::Mode::MetadataEditor);
}

#[tokio::test]
async fn test_enter_metadata_editor_noop_when_empty() {
    let (mut app, _, _, _) = setup_app();
    app.load_prompts().await.unwrap();
    assert!(app.nav.prompts.is_empty());

    app.handle_message(AppMessage::EnterMetadataEditor).await.unwrap();

    assert_eq!(app.mode, promptquiver::app::Mode::List, "should stay in List when no prompts");
}
