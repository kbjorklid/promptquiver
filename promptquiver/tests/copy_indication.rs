mod common;
use common::setup_app;
use contracts::{Prompt, PromptType, Storage};

#[tokio::test]
async fn test_copy_indication() {
    let (mut app, storage, _, _) = setup_app();
    
    let p1 = Prompt::new("P1".to_string(), PromptType::Prompt, None, None);
    let p2 = Prompt::new("P2".to_string(), PromptType::Prompt, None, None);
    storage.save_project_prompts(common::TEST_PATH, vec![p1.clone(), p2.clone()]).await.unwrap();

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
async fn test_copy_icon_rendering() {
    let (mut app, storage, _, _) = setup_app();
    
    let p1 = Prompt::new("P1".to_string(), PromptType::Prompt, None, Some("Name1".to_string()));
    storage.save_project_prompts(common::TEST_PATH, vec![p1]).await.unwrap();

    app.load_prompts().await.unwrap();
    app.copy_selected().await.unwrap();

    let backend = ratatui::backend::TestBackend::new(40, 10);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();

    terminal.draw(|f| {
        ui::render(f, ui::RenderState {
            active_tab: app.nav.active_tab,
            prompts: &app.nav.prompts,
            selected_index: app.nav.selected_index,
            list_state: &mut app.nav.list_state,
            settings_slash_list_state: &mut app.nav.settings_slash_list_state,
            theme_list_state: &mut app.nav.theme_list_state,
            mode: "List",
            textarea: &mut app.editor.textarea,
            title_textarea: &mut app.editor.title_textarea,
            title_focused: app.editor.title_focused,
            current_branch: None,
            current_path: "test",
            suggestions: &[],
            suggestion_index: 0,
            autocomplete_open: app.editor.autocomplete.open,
            autocomplete_list_state: &mut app.editor.autocomplete.list_state,
            search_query: "",
            global_search_query: "",
            settings: &app.settings,
        }, &mut None);
    }).unwrap();

    let buffer = terminal.backend().buffer();
    let mut row_text = String::new();
    // Prompt list usually starts at y=1 (header is 1 line)
    for x in 0..40 {
        row_text.push_str(buffer[(x, 2)].symbol());
    }
    
    // Should contain the copy icon
    assert!(row_text.contains("📋"), "Rendered list should contain copy icon 📋. Found: {}", row_text);
}

