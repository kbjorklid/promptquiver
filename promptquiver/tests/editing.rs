mod common;
use common::setup_app;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[tokio::test]
async fn test_add_edit_prompt() {
    let (mut app, _, _, _) = setup_app();
    
    app.enter_editor("New Prompt".to_string(), None);
    app.save_editor().await.unwrap();
    assert_eq!(app.nav.prompts.len(), 1);
    assert_eq!(app.nav.prompts[0].text, "New Prompt");

    let id = app.nav.prompts[0].id;
    app.enter_editor("Updated Prompt".to_string(), Some(id));
    app.save_editor().await.unwrap();
    
    assert_eq!(app.nav.prompts.len(), 1);
    assert_eq!(app.nav.prompts[0].text, "Updated Prompt");
}

#[tokio::test]
async fn test_editor_discard_confirmation_modal() {
    let (mut app, _, _, _) = setup_app();

    app.enter_editor("Original".to_string(), None);
    app.editor.textarea.insert_str("Modified");
    
    let current_text = app.editor.textarea.lines().join("\n");
    if current_text != app.editor.original_text {
        app.mode = promptquiver::app::Mode::ConfirmDiscard;
    }

    assert_eq!(app.mode, promptquiver::app::Mode::ConfirmDiscard);

    let backend = TestBackend::new(80, 30);
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
    
    let mut found_modal_title = false;
    for y in 0..30 {
        for x in 0..80 {
            let mut line = String::new();
            for i in 0..16 {
                if x + i < 80 {
                    line.push_str(buffer[(x + i, y)].symbol());
                }
            }
            if line.contains("Discard Changes?") {
                found_modal_title = true;
                break;
            }
        }
    }
    assert!(found_modal_title);
}

#[tokio::test]
async fn test_snippet_name_enter_moves_focus() {
    let (mut app, _, _, _) = setup_app();
    use crossterm::event::{KeyEvent, KeyCode, KeyModifiers};
    use contracts::Tab;
    use ui::types::AppMessage;
    
    // Switch to Snippets tab
    app.handle_message(AppMessage::SetTab(Tab::Snippets)).await.unwrap();
    
    // Enter editor
    app.handle_message(AppMessage::EnterEditor(String::new(), None)).await.unwrap();
    
    // Verify initial state
    assert!(app.editor.title_focused, "Title should be focused initially for snippets");

    // Simulate typing "mysnip"
    for c in "mysnip".chars() {
        app.handle_message(AppMessage::EditorInput(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()))).await.unwrap();
    }
    assert_eq!(app.editor.title_textarea.lines()[0], "mysnip");

    // Simulate pressing Enter
    app.handle_message(AppMessage::EditorInput(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()))).await.unwrap();
    
    // Verify focus moved
    assert!(!app.editor.title_focused, "Focus should move to content field");
    assert_eq!(app.editor.title_textarea.lines().len(), 1, "Snippet name should remain single-line");
    assert_eq!(app.editor.title_textarea.lines()[0], "mysnip", "Snippet name should not have been modified");

    // Test Tab key still works
    app.handle_message(AppMessage::EditorInput(KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()))).await.unwrap();
    assert!(app.editor.title_focused, "Tab should move focus back to title");
}

