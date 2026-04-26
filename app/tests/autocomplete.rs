mod common;
use common::setup_app;
use contracts::Storage;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[tokio::test]
async fn test_autocomplete() {
    let (mut app, storage, _, _) = setup_app();

    let s1 = contracts::Prompt::new("test_snippet".to_string(), contracts::PromptType::Snippet, None, Some("ts".to_string()));
    storage.save_global_snippets(vec![s1]).await.unwrap();
    
    app.enter_editor("Hello ".to_string(), None);
    app.textarea.move_cursor(ratatui_textarea::CursorMove::End);

    for c in "$$t".chars() {
        app.textarea.input(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char(c),
            crossterm::event::KeyModifiers::empty(),
        ));
    }
    app.update_autocomplete().await.unwrap();
    
    assert!(app.autocomplete_open);
    assert_eq!(app.suggestions.len(), 1);

    app.select_suggestion();
    assert_eq!(app.textarea.lines()[0], "Hello $$ts");
}

#[tokio::test]
async fn test_autocomplete_slash_command_title() {
    let (mut app, _, _, _) = setup_app();
    app.settings.slash_commands = vec!["test".to_string()];
    app.storage.save_settings(app.settings.clone()).await.unwrap();

    app.enter_editor(" ".to_string(), None);
    app.textarea.move_cursor(ratatui_textarea::CursorMove::End);

    app.textarea.input(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('/'),
        crossterm::event::KeyModifiers::empty(),
    ));
    app.update_autocomplete().await.unwrap();

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
                    mode: "Editor",
                    textarea: &app.textarea,
                    title_textarea: &app.title_textarea,
                    title_focused: app.title_focused,
                    current_branch: None,
                    current_path: "",
                    suggestions: &app.suggestions,
                    suggestion_index: app.suggestion_index,
                    search_query: "",
                    global_search_query: "",
                    settings: &app.settings,
                },
                &mut None,
            );
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    
    let mut found_commands_title = false;
    for y in 0..30 {
        for x in 0..80 {
            let mut line = String::new();
            for i in 0..12 {
                if x + i < 80 {
                    line.push_str(buffer[(x + i, y)].symbol());
                }
            }
            if line.contains(" Commands ") {
                found_commands_title = true;
                break;
            }
        }
    }
    assert!(found_commands_title);
}
