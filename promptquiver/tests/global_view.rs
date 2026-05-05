use contracts::{Prompt, PromptType, Storage};
use promptquiver::app::{App, AppMessage};
use infra::{InMemoryStorage, MockClipboard, MockGit, RealAppService};
use std::sync::Arc;

#[tokio::test]
async fn test_global_view_toggle() {
    let storage = Arc::new(InMemoryStorage::new());
    let clipboard = Arc::new(MockClipboard::new());
    let git = Arc::new(MockGit::new(None));
    let service = Arc::new(RealAppService::new(storage.clone(), clipboard.clone()));

    let mut app = App::new(storage.clone(), clipboard, git, service);

    let project_a = "/path/a";
    let project_b = "/path/b";

    // 1. Setup prompts in different folders
    // Create P2 first so P1 is newer and at index 0
    let prompt_b = Prompt::new("Prompt B".to_string(), PromptType::Prompt, Some(project_b.to_string()), None, None, None);
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    let prompt_a = Prompt::new("Prompt A".to_string(), PromptType::Prompt, Some(project_a.to_string()), None, None, None);
    
    storage.save_prompt(prompt_b).await.unwrap();
    storage.save_prompt(prompt_a).await.unwrap();

    // 2. Set current path to project A and enable folder filter (it's off by default now)
    app.nav.current_path = project_a.to_string();
    app.nav.folder_filter = true;
    app.load_prompts().await.unwrap();

    // 3. Verify only Prompt A is visible initially
    assert_eq!(app.nav.prompts.len(), 1);
    assert_eq!(app.nav.prompts[0].text, "Prompt A");

    // 4. Toggle Global View (Folder Filter)
    app.handle_message(AppMessage::ToggleFolderFilter).await.unwrap();

    // 5. Verify both prompts are visible
    assert_eq!(app.nav.prompts.len(), 2, "Both prompts should be visible in Global View");
    
    let texts: Vec<String> = app.nav.prompts.iter().map(|p| p.text.clone()).collect();
    assert!(texts.contains(&"Prompt A".to_string()));
    assert!(texts.contains(&"Prompt B".to_string()));

    // 6. Toggle back
    app.handle_message(AppMessage::ToggleFolderFilter).await.unwrap();
    assert_eq!(app.nav.prompts.len(), 1);
    assert_eq!(app.nav.prompts[0].text, "Prompt A");
}
