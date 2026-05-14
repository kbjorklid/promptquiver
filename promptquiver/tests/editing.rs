mod common;
use common::setup_app;
use contracts::Tab;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use ui::types::AppMessage;

#[tokio::test]
async fn test_add_edit_prompt() {
    let (mut app, _, _, _) = setup_app();

    app.enter_editor("New Prompt".to_string(), None);
    app.save_editor().await.unwrap();
    assert_eq!(app.nav.prompts.len(), 1);
    assert_eq!(app.nav.prompts[0].text, "New Prompt");
    assert_eq!(app.nav.selected_index, 0);

    let id = app.nav.prompts[0].id;
    app.enter_editor("Updated Prompt".to_string(), Some(id));
    app.save_editor().await.unwrap();

    assert_eq!(app.nav.prompts.len(), 1);
    assert_eq!(app.nav.prompts[0].text, "Updated Prompt");
    assert_eq!(app.nav.selected_index, 0);
}

#[tokio::test]
async fn test_selection_focus_after_creation() {
    let (mut app, _, _, _) = setup_app();

    // Create first prompt
    app.enter_editor("Prompt 1".to_string(), None);
    app.save_editor().await.unwrap();

    // Create second prompt
    app.enter_editor("Prompt 2".to_string(), None);
    app.save_editor().await.unwrap();

    // Move selection first
    app.nav.selected_index = 1; // Select Prompt 1 (which is at index 1 now, index 0 is Prompt 2)

    app.enter_editor("Prompt 3".to_string(), None);
    app.save_editor().await.unwrap();

    assert_eq!(app.nav.prompts.len(), 3);
    assert_eq!(app.nav.prompts[0].text, "Prompt 3");
    assert_eq!(app.nav.selected_index, 0, "New prompt should be selected");
}

#[tokio::test]
async fn test_create_title_in_editor() {
    let (mut app, _, _, _) = setup_app();

    // 1. Test "Create Prompt"
    app.enter_editor(String::new(), None);

    let backend = TestBackend::new(80, 10);
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
                    show_help: app.show_help,
                    help_scroll: app.help_scroll,
                },
                &mut None,
            );
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let mut found_create = false;
    for y in 0..10 {
        for x in 0..60 {
            let mut line = String::new();
            for i in 0..15 {
                if x + i < 80 {
                    line.push_str(buffer[(x + i, y)].symbol());
                }
            }
            if line.contains("Create Prompt") {
                found_create = true;
                break;
            }
        }
    }
    assert!(found_create, "Title 'Create Prompt' not found in buffer");

    // 2. Test "Create Snippet"
    app.exit_editor();
    app.handle_message(AppMessage::SetTab(Tab::Snippets)).await.unwrap();
    app.enter_editor(String::new(), None);

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
                    show_help: app.show_help,
                    help_scroll: app.help_scroll,
                },
                &mut None,
            );
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let mut found_create_snippet = false;
    for y in 0..10 {
        for x in 0..60 {
            let mut line = String::new();
            for i in 0..16 {
                if x + i < 80 {
                    line.push_str(buffer[(x + i, y)].symbol());
                }
            }
            if line.contains("Create Snippet") {
                found_create_snippet = true;
                break;
            }
        }
    }
    assert!(found_create_snippet, "Title 'Create Snippet' not found in buffer");
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
                    show_help: app.show_help,
                    help_scroll: app.help_scroll,
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

    // Switch to Snippets tab
    app.handle_message(AppMessage::SetTab(Tab::Snippets)).await.unwrap();

    // Enter editor
    app.handle_message(AppMessage::EnterEditor(String::new(), None)).await.unwrap();

    // Verify initial state
    assert!(app.editor.title_focused, "Title should be focused initially for snippets");

    // Simulate typing "mysnip"
    for c in "mysnip".chars() {
        app.handle_message(AppMessage::EditorInput(KeyEvent::new(
            KeyCode::Char(c),
            KeyModifiers::empty(),
        )))
        .await
        .unwrap();
    }
    assert_eq!(app.editor.title_textarea.lines()[0], "mysnip");

    // Simulate pressing Enter
    app.handle_message(AppMessage::EditorInput(KeyEvent::new(
        KeyCode::Enter,
        KeyModifiers::empty(),
    )))
    .await
    .unwrap();

    // Verify focus moved
    assert!(!app.editor.title_focused, "Focus should move to content field");
    assert_eq!(
        app.editor.title_textarea.lines().len(),
        1,
        "Snippet name should remain single-line"
    );
    assert_eq!(
        app.editor.title_textarea.lines()[0],
        "mysnip",
        "Snippet name should not have been modified"
    );

    // Test Tab key still works
    app.handle_message(AppMessage::EditorInput(KeyEvent::new(KeyCode::Tab, KeyModifiers::empty())))
        .await
        .unwrap();
    assert!(app.editor.title_focused, "Tab should move focus back to title");
}

#[tokio::test]
async fn test_paste_in_editor() {
    let (mut app, _, _, _) = setup_app();

    // Enter editor
    app.enter_editor(String::new(), None);

    // Simulate a paste event
    let paste_content = "Pasted content".to_string();
    let event = Event::Paste(paste_content.clone());

    // Handle the event
    promptquiver::handlers::handle_event(&mut app, event).await;

    // Assert that the content was pasted
    assert_eq!(app.editor.textarea.lines()[0], paste_content);
}
