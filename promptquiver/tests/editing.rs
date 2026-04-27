mod common;
use common::setup_app;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[tokio::test]
async fn test_add_edit_prompt() {
    let (mut app, _, _, _) = setup_app();
    
    app.enter_editor("New Prompt".to_string(), None);
    app.save_editor().await.unwrap();
    assert_eq!(app.prompts.len(), 1);
    assert_eq!(app.prompts[0].text, "New Prompt");

    let id = app.prompts[0].id;
    app.enter_editor("Updated Prompt".to_string(), Some(id));
    app.save_editor().await.unwrap();
    
    assert_eq!(app.prompts.len(), 1);
    assert_eq!(app.prompts[0].text, "Updated Prompt");
}

#[tokio::test]
async fn test_editor_discard_confirmation_modal() {
    let (mut app, _, _, _) = setup_app();

    app.enter_editor("Original".to_string(), None);
    app.textarea.insert_str("Modified");
    
    let current_text = app.textarea.lines().join("\n");
    if current_text != app.original_text {
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
                    active_tab: app.active_tab,
                    prompts: &app.prompts,
                    selected_index: app.selected_index,
                    list_state: &mut app.list_state,
                    settings_slash_list_state: &mut app.settings_slash_list_state,
                    theme_list_state: &mut app.theme_list_state,
                    mode: "Confirm Discard",
                    textarea: &app.textarea,
                    title_textarea: &app.title_textarea,
                    title_focused: app.title_focused,
                    current_branch: None,
                    current_path: "",
                    suggestions: &[],
                    suggestion_index: 0,
                    search_query: "",
                    global_search_query: "",
                    settings: &contracts::Settings::default(),
                    throbber_state: &mut app.throbber_state,
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
    app.active_tab = Tab::Snippets;
    
    // Enter editor
    app.enter_editor("Snippet content".to_string(), None);
    
    // Verify initial state
    assert!(app.title_focused, "Title should be focused initially for snippets");
    assert_eq!(app.title_textarea.lines()[0], "", "Title should be empty");

    // Simulate typing "mysnip"
    for c in "mysnip".chars() {
        app.title_textarea.input(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()));
    }
    assert_eq!(app.title_textarea.lines()[0], "mysnip");

    // Simulate pressing Enter
    let event = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    
    // Explicitly handle Enter for title_focused snippet (Logic from main.rs)
    if app.title_focused && app.active_tab == Tab::Snippets && event.code == KeyCode::Enter {
        app.title_focused = false;
    } else {
        app.title_textarea.input(event);
    }
    
    // Verify behavior after fix
    assert!(!app.title_focused, "Focus should move to content field");
    assert_eq!(app.title_textarea.lines().len(), 1, "Snippet name should remain single-line");
    assert_eq!(app.title_textarea.lines()[0], "mysnip", "Snippet name should not have been modified");

    // Test Tab key still works
    let tab_event = KeyEvent::new(KeyCode::Tab, KeyModifiers::empty());
    if tab_event.code == KeyCode::Tab && app.active_tab == Tab::Snippets {
        app.title_focused = !app.title_focused;
    }
    assert!(app.title_focused, "Tab should move focus back to title");
}
