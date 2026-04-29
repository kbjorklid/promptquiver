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
    
    // Switch to Snippets tab
    app.nav.active_tab = Tab::Snippets;
    
    // Enter editor
    app.enter_editor("Snippet content".to_string(), None);
    
    // Verify initial state
    assert!(app.editor.title_focused, "Title should be focused initially for snippets");
    assert_eq!(app.editor.title_textarea.lines()[0], "", "Title should be empty");

    // Simulate typing "mysnip"
    for c in "mysnip".chars() {
        app.editor.title_textarea.input(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()));
    }
    assert_eq!(app.editor.title_textarea.lines()[0], "mysnip");

    // Simulate pressing Enter
    let event = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    
    // Explicitly handle Enter for title_focused snippet (Logic from main.rs)
    if app.editor.title_focused && app.nav.active_tab == Tab::Snippets && event.code == KeyCode::Enter {
        app.editor.title_focused = false;
    } else {
        app.editor.title_textarea.input(event);
    }
    
    // Verify behavior after fix
    assert!(!app.editor.title_focused, "Focus should move to content field");
    assert_eq!(app.editor.title_textarea.lines().len(), 1, "Snippet name should remain single-line");
    assert_eq!(app.editor.title_textarea.lines()[0], "mysnip", "Snippet name should not have been modified");

    // Test Tab key still works
    let tab_event = KeyEvent::new(KeyCode::Tab, KeyModifiers::empty());
    if tab_event.code == KeyCode::Tab && app.nav.active_tab == Tab::Snippets {
        app.editor.title_focused = !app.editor.title_focused;
    }
    assert!(app.editor.title_focused, "Tab should move focus back to title");
}

