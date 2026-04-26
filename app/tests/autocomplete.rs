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
    assert!(line.contains("@test_file_for_autocomplete.txt"));
    assert!(!line.contains("Check @test ")); // Ensure it didn't just append
    assert!(!line.ends_with("@test")); // Ensure @test was replaced

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
                    list_state: &mut app.list_state,
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

#[tokio::test]
async fn test_autocomplete_closes_on_trigger_removal() {
    let (mut app, _, _, _) = setup_app();
    
    app.enter_editor("".to_string(), None);
    
    // Type @
    app.textarea.input(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('@'),
        crossterm::event::KeyModifiers::empty(),
    ));
    // Simulate finding a file (manually since background searcher is complex to setup here)
    app.suggestions = vec![contracts::Prompt::new("test".to_string(), contracts::PromptType::Note, None, Some("test".to_string()))];
    app.autocomplete_open = true;

    assert!(app.autocomplete_open);
    assert!(!app.suggestions.is_empty());

    // Remove @
    app.textarea.input(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Backspace,
        crossterm::event::KeyModifiers::empty(),
    ));
    app.update_autocomplete().await.unwrap();
    
    // It should be closed
    assert!(!app.autocomplete_open, "Autocomplete should be closed after removing trigger");
    // AND suggestions should be empty so it's not rendered
    assert!(app.suggestions.is_empty(), "Suggestions should be cleared after removing trigger");
}

#[tokio::test]
async fn test_autocomplete_positioning_below_cursor() {
    let (mut app, _, _, _) = setup_app();
    app.settings.slash_commands = vec!["test".to_string()];

    app.enter_editor("line1\nline2".to_string(), None);
    // Move to end of line 2
    app.textarea.move_cursor(ratatui_textarea::CursorMove::Down);
    app.textarea.move_cursor(ratatui_textarea::CursorMove::End);
    
    // Type /
    app.textarea.input(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('/'),
        crossterm::event::KeyModifiers::empty(),
    ));
    app.update_autocomplete().await.unwrap();

    let backend = TestBackend::new(80, 30);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();

    terminal.draw(|f| {
        ui::render(f, ui::RenderState {
            active_tab: app.active_tab,
            prompts: &app.prompts,
            selected_index: app.selected_index,
            list_state: &mut app.list_state,
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
        }, &mut None);
    }).unwrap();
    
    let buffer = terminal.backend().buffer();
    
    // Header: y=0,1,2
    // Editor Start: y=3
    // line1: y=4
    // line2/: y=5
    // Popup Title: y=6
    
    let mut found_popup_at_expected_y = false;
    for x in 0..80 {
        let mut title = String::new();
        for i in 0..12 {
            if x + i < 80 {
                title.push_str(buffer[(x + i, 6)].symbol());
            }
        }
        if title.contains(" Commands ") {
            found_popup_at_expected_y = true;
            break;
        }
    }
    
    assert!(found_popup_at_expected_y, "Popup should be rendered at y=6 (below line 2)");
}

#[tokio::test]
async fn test_autocomplete_positioning_above_cursor() {
    let (mut app, _, _, _) = setup_app();
    app.settings.slash_commands = vec![
        "test1".to_string(), 
        "test2".to_string(), 
        "test3".to_string(), 
        "test4".to_string(),
        "test5".to_string()
    ];

    // Create enough lines to push cursor to the bottom
    let lines = (0..25).map(|i| format!("line{}", i)).collect::<Vec<_>>().join("\n");
    app.enter_editor(lines, None);
    
    // Move to last line (line24)
    for _ in 0..24 {
        app.textarea.move_cursor(ratatui_textarea::CursorMove::Down);
    }
    app.textarea.move_cursor(ratatui_textarea::CursorMove::End);
    
    // Type /
    app.textarea.input(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('/'),
        crossterm::event::KeyModifiers::empty(),
    ));
    app.update_autocomplete().await.unwrap();

    let backend = TestBackend::new(80, 20);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();

    terminal.draw(|f| {
        ui::render(f, ui::RenderState {
            active_tab: app.active_tab,
            prompts: &app.prompts,
            selected_index: app.selected_index,
            list_state: &mut app.list_state,
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
        }, &mut None);
    }).unwrap();
    
    let buffer = terminal.backend().buffer();
    
    // Find where the cursor is (line24/)
    let mut cursor_y = 0;
    for y in 0..20 {
        let mut line = String::new();
        for x in 0..15 {
            line.push_str(buffer[(x, y)].symbol());
        }
        if line.contains("line24/") {
            cursor_y = y;
        }
    }
    assert!(cursor_y > 0, "Could not find cursor at line24/");

    // Popup should be ABOVE cursor_y
    let mut found_popup_above = false;
    for y in 0..cursor_y {
        for x in 0..80 {
            let mut title = String::new();
            for i in 0..12 {
                if x + i < 80 {
                    title.push_str(buffer[(x + i, y)].symbol());
                }
            }
            if title.contains(" Commands ") {
                found_popup_above = true;
                break;
            }
        }
    }
    
    assert!(found_popup_above, "Popup should be rendered above cursor when at bottom");
}
