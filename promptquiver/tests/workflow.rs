mod common;
use common::setup_app;
use contracts::{Storage, Clipboard, PromptFilter, Tab};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[tokio::test]
async fn test_basic_render() {
    let (mut app, _, _, _) = setup_app();

    let backend = TestBackend::new(40, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            ui::render(
                f,
                ui::RenderState {
                    nav: &mut app.nav,
                    editor: &mut app.editor,
                    mode: app.mode,
                    settings: &app.settings,
                    current_branch: app.current_branch.as_deref(),
                },
                &mut None,
            );

        })
        .unwrap();

    let buffer = terminal.backend().buffer();

    let mut found_title = false;
    for x in 0..40 {
        let s = buffer[(x, 0)].symbol();
        if s.contains('P') {
            let mut title = String::new();
            for i in 0..13 {
                if x + i < 40 {
                    title.push_str(buffer[(x + i, 0)].symbol());
                }
            }
            if title.contains("PROMPT QUIVER") {
                found_title = true;
                break;
            }
        }
    }
    assert!(found_title, "Title 'PROMPT QUIVER' not found in buffer");
}

#[tokio::test]
async fn test_quit_event() {
    let (mut app, _, _, _) = setup_app();
    assert!(!app.should_quit);

    app.quit();
    assert!(app.should_quit);
}

#[tokio::test]
async fn test_staging() {
    let (mut app, storage, clipboard, _) = setup_app();
    
    let p2 = contracts::Prompt::new("P2".to_string(), contracts::PromptType::Prompt, Some(common::TEST_PATH.to_string()), None, None, None);
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    let p1 = contracts::Prompt::new("P1".to_string(), contracts::PromptType::Prompt, Some(common::TEST_PATH.to_string()), None, None, None);
    storage.save_prompt(p2).await.unwrap();
    storage.save_prompt(p1).await.unwrap();

    app.load_prompts().await.unwrap();

    app.stage_selected().await.unwrap();
    assert!(app.nav.prompts[0].staged);
    assert_eq!(clipboard.paste().await.unwrap(), "P1");

    app.move_down();
    app.stage_selected().await.unwrap();
    
    assert_eq!(app.nav.prompts.len(), 1);
    assert_eq!(app.nav.prompts[0].text, "P2");
    assert!(app.nav.prompts[0].staged);
    assert_eq!(clipboard.paste().await.unwrap(), "P2");

    let archive = storage.get_prompts(PromptFilter { folder: Some(common::TEST_PATH.to_string()), tab: Some(Tab::Archive), ..Default::default() }).await.unwrap();
    assert_eq!(archive.len(), 1);
    assert_eq!(archive[0].text, "P1");
}

#[tokio::test]
async fn test_unstaging() {
    let (mut app, storage, _, _) = setup_app();
    
    let mut p1 = contracts::Prompt::new("P1".to_string(), contracts::PromptType::Prompt, Some(common::TEST_PATH.to_string()), None, None, None);
    p1.staged = true;
    storage.save_prompt(p1).await.unwrap();

    app.load_prompts().await.unwrap();
    assert!(app.nav.prompts[0].staged);

    // Unstage
    app.stage_selected().await.unwrap();
    assert!(!app.nav.prompts[0].staged, "Should be unstaged in memory");

    // Verify persistence
    let stored = storage.get_prompts(PromptFilter { folder: Some(common::TEST_PATH.to_string()), tab: Some(Tab::Prompts), ..Default::default() }).await.unwrap();
    assert!(!stored[0].staged, "Should be unstaged in storage");
}

#[tokio::test]
async fn test_archive_restore() {
    let (mut app, storage, _, _) = setup_app();
    
    let p1 = contracts::Prompt::new("P1".to_string(), contracts::PromptType::Prompt, Some(common::TEST_PATH.to_string()), None, None, None);
    storage.save_prompt(p1).await.unwrap();

    app.load_prompts().await.unwrap();
    assert_eq!(app.nav.prompts.len(), 1);

    // Archive
    app.archive_selected().await.unwrap();
    assert_eq!(app.nav.prompts.len(), 0);

    // Go to Archive tab
    app.set_tab(contracts::Tab::Archive);
    app.load_prompts().await.unwrap();
    assert_eq!(app.nav.prompts.len(), 1);
    assert_eq!(app.nav.prompts[0].text, "P1");

    // Restore
    app.restore_selected().await.unwrap();
    assert_eq!(app.nav.prompts.len(), 0);

    // Go back to Prompts tab
    app.set_tab(contracts::Tab::Prompts);
    app.load_prompts().await.unwrap();
    assert_eq!(app.nav.prompts.len(), 1);
    assert_eq!(app.nav.prompts[0].text, "P1");
}

#[tokio::test]
async fn test_archive_delete() {
    let (mut app, storage, _, _) = setup_app();
    
    let p1 = contracts::Prompt::new("P1".to_string(), contracts::PromptType::Prompt, Some(common::TEST_PATH.to_string()), None, None, None);
    storage.save_prompt(p1).await.unwrap();

    app.load_prompts().await.unwrap();

    app.archive_selected().await.unwrap();
    assert_eq!(app.nav.prompts.len(), 0);

    app.set_tab(contracts::Tab::Archive);
    app.load_prompts().await.unwrap();
    assert_eq!(app.nav.prompts.len(), 1);

    app.archive_selected().await.unwrap();
    assert_eq!(app.nav.prompts.len(), 0);
    
    let archive = storage.get_prompts(PromptFilter { folder: Some(common::TEST_PATH.to_string()), tab: Some(Tab::Archive), ..Default::default() }).await.unwrap();
    assert_eq!(archive.len(), 0);
}
