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
async fn test_autocomplete_file() {
    let (mut app, _, _, _) = setup_app();
    
    // Setup channels like in main.rs
    let (file_search_tx, mut file_search_rx) = tokio::sync::mpsc::channel::<(String, String)>(10);
    let (file_result_tx, mut file_result_rx) = tokio::sync::mpsc::channel::<Vec<contracts::Prompt>>(10);
    app.file_search_tx = Some(file_search_tx);

    // Create a dummy file for testing
    let temp_dir = std::env::current_dir().unwrap();
    let temp_file = temp_dir.join("test_file_for_autocomplete.txt");
    std::fs::write(&temp_file, "content").unwrap();
    app.current_path = temp_dir.to_string_lossy().to_string();

    app.enter_editor("Check @test".to_string(), None);
    app.textarea.move_cursor(ratatui_textarea::CursorMove::End);

    app.update_autocomplete().await.unwrap();
    
    // Simulate background searcher
    if let Some((_path, _query)) = file_search_rx.recv().await {
        let mut results = Vec::new();
        // We need a way to call walk_files here, but it's in main.rs.
        // For testing, we can just manually push the expected result.
        results.push(contracts::Prompt::new(
            temp_file.to_string_lossy().to_string(),
            contracts::PromptType::Note,
            None,
            Some("test_file_for_autocomplete.txt".to_string()),
        ));
        file_result_tx.send(results).await.unwrap();
    }

    // Receive result
    if let Ok(results) = file_result_rx.try_recv() {
        app.suggestions = results;
        app.autocomplete_open = true;
    }
    
    assert!(app.autocomplete_open);
    assert!(app.suggestions.iter().any(|s| s.name.as_deref() == Some("test_file_for_autocomplete.txt")));

    app.select_suggestion();
    let line = &app.textarea.lines()[0];
    assert!(line.contains("test_file_for_autocomplete.txt"));
    assert!(!line.contains("@test"));

    std::fs::remove_file(temp_file).unwrap();
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
