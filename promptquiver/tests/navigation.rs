mod common;
use common::setup_app;
use contracts::{Tab, Storage};

#[tokio::test]
async fn test_tab_navigation() {
    let (mut app, _, _, _) = setup_app();
    assert_eq!(app.nav.active_tab, Tab::Prompts);

    app.next_tab();
    assert_eq!(app.nav.active_tab, Tab::Canned);

    app.next_tab();
    assert_eq!(app.nav.active_tab, Tab::Notes);

    app.prev_tab();
    assert_eq!(app.nav.active_tab, Tab::Canned);
}

#[tokio::test]
async fn test_list_navigation() {
    let (mut app, storage, _, _) = setup_app();

    let prompts = vec![
        contracts::Prompt::new("Prompt 1".to_string(), contracts::PromptType::Prompt, None, None),
        contracts::Prompt::new("Prompt 2".to_string(), contracts::PromptType::Prompt, None, None),
    ];
    storage.save_project_prompts(common::TEST_PATH, prompts).await.unwrap();

    app.load_prompts().await.unwrap();

    assert_eq!(app.nav.prompts.len(), 2);
    assert_eq!(app.nav.selected_index, 0);

    app.move_down();
    assert_eq!(app.nav.selected_index, 1);

    app.move_up();
    assert_eq!(app.nav.selected_index, 0);
}

#[tokio::test]
async fn test_tab_specific_content() {
    let (mut app, storage, _, _) = setup_app();
    
    storage.save_project_prompts(common::TEST_PATH, vec![
        contracts::Prompt::new("P1".to_string(), contracts::PromptType::Prompt, None, None)
    ]).await.unwrap();
    
    storage.save_project_notes(common::TEST_PATH, vec![
        contracts::Prompt::new("N1".to_string(), contracts::PromptType::Note, None, None)
    ]).await.unwrap();

    app.load_prompts().await.unwrap();
    assert_eq!(app.nav.prompts.len(), 1);
    assert_eq!(app.nav.prompts[0].text, "P1");

    app.set_tab(Tab::Notes);
    app.load_prompts().await.unwrap();
    assert_eq!(app.nav.prompts.len(), 1);
    assert_eq!(app.nav.prompts[0].text, "N1");
}

