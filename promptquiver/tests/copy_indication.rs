mod common;
use common::setup_app;
use contracts::{Prompt, PromptType, Storage, Clipboard};

#[tokio::test]
async fn test_copy_indication() {
    let (mut app, storage, _, _) = setup_app();
    
    let p1 = Prompt::new("P1".to_string(), PromptType::Prompt, Some(common::TEST_PATH.to_string()), None, None, None);
    let p2 = Prompt::new("P2".to_string(), PromptType::Prompt, Some(common::TEST_PATH.to_string()), None, None, None);
    storage.save_prompt(p1.clone()).await.unwrap();
    storage.save_prompt(p2.clone()).await.unwrap();

    app.load_prompts().await.unwrap();
    assert_eq!(app.nav.prompts.len(), 2);

    // 1. Copy first prompt
    app.copy_selected().await.unwrap();
    assert!(app.nav.prompts[0].last_copied, "First prompt should be marked as last_copied");
    assert!(!app.nav.prompts[1].last_copied, "Second prompt should NOT be marked as last_copied");

    // 2. Copy second prompt
    app.move_down();
    app.copy_selected().await.unwrap();
    assert!(!app.nav.prompts[0].last_copied, "First prompt should NO LONGER be marked as last_copied");
    assert!(app.nav.prompts[1].last_copied, "Second prompt should NOW be marked as last_copied");

    // 3. Stage a prompt should clear last_copied
    app.stage_selected().await.unwrap();
    // Re-load because stage_selected reloads
    app.load_prompts().await.unwrap();
    
    for p in &app.nav.prompts {
        assert!(!p.last_copied, "Staging should clear last_copied icon from all prompts");
    }
}

#[tokio::test]
async fn test_copy_via_y_key() {
    let (mut app, storage, clipboard, _) = setup_app();
    
    let p1 = contracts::Prompt::new("Copy this text".to_string(), contracts::PromptType::Prompt, Some(common::TEST_PATH.to_string()), None, None, None);
    storage.save_prompt(p1).await.unwrap();

    app.load_prompts().await.unwrap();
    
    // Set some old content in clipboard
    clipboard.copy("Old content".to_string()).await.unwrap();

    // Simulate pressing 'y'
    let key = crossterm::event::KeyEvent::new(crossterm::event::KeyCode::Char('y'), crossterm::event::KeyModifiers::NONE);
    promptquiver::handlers::handle_key_event(&mut app, key).await;

    // Verify clipboard
    let current_clipboard = clipboard.paste().await.unwrap();
    assert_eq!(current_clipboard, "Copy this text", "Clipboard should contain the prompt text after pressing 'y'");
}

#[tokio::test]
async fn test_copy_via_y_key_canned_tab() {
    let (mut app, storage, clipboard, _) = setup_app();
    
    let p1 = contracts::Prompt::new("Canned prompt text".to_string(), contracts::PromptType::Prompt, None, None, None, None);
    storage.save_prompt(p1).await.unwrap();

    app.set_tab(contracts::Tab::Canned);
    app.load_prompts().await.unwrap();
    
    // Set some old content in clipboard
    clipboard.copy("Old content".to_string()).await.unwrap();

    // Simulate pressing 'y'
    let key = crossterm::event::KeyEvent::new(crossterm::event::KeyCode::Char('y'), crossterm::event::KeyModifiers::NONE);
    promptquiver::handlers::handle_key_event(&mut app, key).await;

    // Verify clipboard
    let current_clipboard = clipboard.paste().await.unwrap();
    assert_eq!(current_clipboard, "Canned prompt text", "Clipboard should contain the canned prompt text after pressing 'y'");
}

#[tokio::test]
async fn test_copy_icon_rendering() {
    let (mut app, storage, _, _) = setup_app();
    
    let p1 = Prompt::new("P1".to_string(), PromptType::Prompt, Some(common::TEST_PATH.to_string()), None, Some("Name1".to_string()), None);
    storage.save_prompt(p1).await.unwrap();

    app.load_prompts().await.unwrap();
    app.copy_selected().await.unwrap();

    let backend = ratatui::backend::TestBackend::new(40, 10);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();

    terminal.draw(|f| {
        ui::render(f, ui::RenderState {
            nav: &mut app.nav,
            editor: &mut app.editor,
            mode: app.mode,
            settings: &app.settings,
            current_branch: app.current_branch.as_deref(),
            show_help: app.show_help,
            help_scroll: app.help_scroll,
        }, &mut None);
    }).unwrap();

    let buffer = terminal.backend().buffer();
    let mut row_text = String::new();
    // Prompt list usually starts at y=1 (header is 1 line)
    for x in 0..40 {
        row_text.push_str(buffer[(x, 2)].symbol());
    }
    
    // Should contain the copy icon
    assert!(row_text.contains("📋"), "Rendered list should contain copy icon 📋. Found: {row_text}");
}

