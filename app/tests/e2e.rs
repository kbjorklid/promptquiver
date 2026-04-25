use app::app::App;
use contracts::{Clipboard, Storage};
use infra::{InMemoryStorage, MockClipboard, MockGit};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::sync::Arc;

#[tokio::test]
async fn test_basic_render() {
    let storage = Arc::new(InMemoryStorage::new());
    let clipboard = Arc::new(MockClipboard::new());
    let git = Arc::new(MockGit::new(None));
    let mut app = App::new(storage, clipboard, git);

    let backend = TestBackend::new(40, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let mode_str = match app.mode {
                app::app::Mode::List => "List",
                app::app::Mode::Editor => "Editor",
                app::app::Mode::Move => "Move",
                app::app::Mode::Search => "Search",
                app::app::Mode::GlobalSearch => "Global Search",
                app::app::Mode::ConfirmDiscard => "Confirm Discard",
            };
            ui::render(
                f,
                app.active_tab,
                &app.prompts,
                app.selected_index,
                mode_str,
                &app.textarea,
                app.current_branch.as_deref(),
                &app.suggestions,
                app.suggestion_index,
                &mut app.toaster,
                &app.search_query,
                &app.global_search_query,
                &app.settings,
            );
        })
        .unwrap();

    let buffer = terminal.backend().buffer();

    // Check for "PROMPT QUIVER" title
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

    // Check for "Prompts" (active tab)
    let mut found_prompts = false;
    for y in 0..10 {
        for x in 0..40 {
            let mut line = String::new();
            for i in 0..7 {
                if x + i < 40 {
                    line.push_str(buffer[(x + i, y)].symbol());
                }
            }
            if line.contains("Prompts") {
                found_prompts = true;
                break;
            }
        }
        if found_prompts {
            break;
        }
    }
    assert!(found_prompts, "Active tab 'Prompts' not found in buffer");
}


#[tokio::test]
async fn test_quit_event() {
    let storage = Arc::new(InMemoryStorage::new());
    let clipboard = Arc::new(MockClipboard::new());
    let git = Arc::new(MockGit::new(None));
    let mut app = App::new(storage, clipboard, git);

    assert!(!app.should_quit);

    // Simulate 'q' key press logic (this is what main loop does)
    let event = crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('q'),
        crossterm::event::KeyModifiers::empty(),
    ));

    if let crossterm::event::Event::Key(key) = event {
        if key.code == crossterm::event::KeyCode::Char('q') {
            app.quit();
        }
    }

    assert!(app.should_quit);
}

#[tokio::test]
async fn test_tab_navigation() {
    let storage = Arc::new(InMemoryStorage::new());
    let clipboard = Arc::new(MockClipboard::new());
    let git = Arc::new(MockGit::new(None));
    let mut app = App::new(storage, clipboard, git);

    assert_eq!(app.active_tab, contracts::Tab::Prompts);

    // Simulate Tab key press
    app.next_tab();
    assert_eq!(app.active_tab, contracts::Tab::Canned);

    app.next_tab();
    assert_eq!(app.active_tab, contracts::Tab::Notes);

    app.prev_tab();
    assert_eq!(app.active_tab, contracts::Tab::Canned);
}

#[tokio::test]
async fn test_list_navigation() {
    let storage = Arc::new(InMemoryStorage::new());
    let clipboard = Arc::new(MockClipboard::new());
    let git = Arc::new(MockGit::new(None));

    // Seed storage with some prompts
    let prompts = vec![
        contracts::Prompt::new("Prompt 1".to_string(), contracts::PromptType::Prompt, None, None),
        contracts::Prompt::new("Prompt 2".to_string(), contracts::PromptType::Prompt, None, None),
    ];
    let path = std::env::current_dir().unwrap().to_string_lossy().into_owned();
    storage.save_project_prompts(&path, prompts).await.unwrap();

    let mut app = App::new(storage, clipboard, git);
    app.load_prompts().await.unwrap();

    assert_eq!(app.prompts.len(), 2);
    assert_eq!(app.selected_index, 0);

    app.move_down();
    assert_eq!(app.selected_index, 1);

    app.move_down(); // Should not go further
    assert_eq!(app.selected_index, 1);

    app.move_up();
    assert_eq!(app.selected_index, 0);

    app.move_up(); // Should not go further
    assert_eq!(app.selected_index, 0);
}

#[tokio::test]
async fn test_tab_specific_content() {
    let storage = Arc::new(InMemoryStorage::new());
    let clipboard = Arc::new(MockClipboard::new());
    let git = Arc::new(MockGit::new(None));

    let path = std::env::current_dir().unwrap().to_string_lossy().into_owned();
    
    // Prompts
    storage.save_project_prompts(&path, vec![
        contracts::Prompt::new("P1".to_string(), contracts::PromptType::Prompt, None, None)
    ]).await.unwrap();
    
    // Notes
    storage.save_project_notes(&path, vec![
        contracts::Prompt::new("N1".to_string(), contracts::PromptType::Note, None, None)
    ]).await.unwrap();

    let mut app = App::new(storage, clipboard, git);
    
    // Initial (Prompts)
    app.load_prompts().await.unwrap();
    assert_eq!(app.prompts.len(), 1);
    assert_eq!(app.prompts[0].text, "P1");

    // Switch to Notes
    app.set_tab(contracts::Tab::Notes);
    app.load_prompts().await.unwrap();
    assert_eq!(app.prompts.len(), 1);
    assert_eq!(app.prompts[0].text, "N1");
}

#[tokio::test]
async fn test_staging() {
    let storage = Arc::new(InMemoryStorage::new());
    let clipboard = Arc::new(MockClipboard::new());
    let git = Arc::new(MockGit::new(None));

    let path = std::env::current_dir().unwrap().to_string_lossy().into_owned();
    
    // Seed with two prompts
    let p1 = contracts::Prompt::new("P1".to_string(), contracts::PromptType::Prompt, None, None);
    let p2 = contracts::Prompt::new("P2".to_string(), contracts::PromptType::Prompt, None, None);
    storage.save_project_prompts(&path, vec![p1.clone(), p2.clone()]).await.unwrap();

    let mut app = App::new(storage.clone(), clipboard.clone(), git);
    app.load_prompts().await.unwrap();

    // Stage P1
    app.stage_selected().await.unwrap();
    assert!(app.prompts[0].staged);
    assert_eq!(clipboard.paste().await.unwrap(), "P1");

    // Move to P2 and stage it
    app.move_down();
    app.stage_selected().await.unwrap();
    
    // P2 should be staged, P1 should be archived (removed from prompts)
    assert_eq!(app.prompts.len(), 1);
    assert_eq!(app.prompts[0].text, "P2");
    assert!(app.prompts[0].staged);
    assert_eq!(clipboard.paste().await.unwrap(), "P2");

    // Check Archive
    let archive = storage.get_project_archive(&path).await.unwrap();
    assert_eq!(archive.len(), 1);
    assert_eq!(archive[0].text, "P1");
    assert!(!archive[0].staged);
}

#[tokio::test]
async fn test_add_edit_prompt() {
    let storage = Arc::new(InMemoryStorage::new());
    let clipboard = Arc::new(MockClipboard::new());
    let git = Arc::new(MockGit::new(None));

    let mut app = App::new(storage, clipboard, git);
    
    // Add new prompt
    app.enter_editor("New Prompt".to_string(), None);
    assert_eq!(app.mode, app::app::Mode::Editor);
    
    app.save_editor().await.unwrap();
    assert_eq!(app.mode, app::app::Mode::List);
    assert_eq!(app.prompts.len(), 1);
    assert_eq!(app.prompts[0].text, "New Prompt");

    // Edit existing
    let id = app.prompts[0].id;
    app.enter_editor("Updated Prompt".to_string(), Some(id));
    app.save_editor().await.unwrap();
    
    assert_eq!(app.prompts.len(), 1);
    assert_eq!(app.prompts[0].text, "Updated Prompt");
}

#[tokio::test]
async fn test_archive_delete() {
    let storage = Arc::new(InMemoryStorage::new());
    let clipboard = Arc::new(MockClipboard::new());
    let git = Arc::new(MockGit::new(None));

    let path = std::env::current_dir().unwrap().to_string_lossy().into_owned();
    
    // Seed with a prompt
    let p1 = contracts::Prompt::new("P1".to_string(), contracts::PromptType::Prompt, None, None);
    storage.save_project_prompts(&path, vec![p1.clone()]).await.unwrap();

    let mut app = App::new(storage.clone(), clipboard, git);
    app.load_prompts().await.unwrap();

    // Archive it
    app.archive_selected().await.unwrap();
    assert_eq!(app.prompts.len(), 0);

    // Check Archive tab
    app.set_tab(contracts::Tab::Archive);
    app.load_prompts().await.unwrap();
    assert_eq!(app.prompts.len(), 1);
    assert_eq!(app.prompts[0].text, "P1");

    // Delete permanently from Archive
    app.archive_selected().await.unwrap();
    assert_eq!(app.prompts.len(), 0);
    
    let archive = storage.get_project_archive(&path).await.unwrap();
    assert_eq!(archive.len(), 0);
}

#[tokio::test]
async fn test_autocomplete() {
    let storage = Arc::new(InMemoryStorage::new());
    let clipboard = Arc::new(MockClipboard::new());
    let git = Arc::new(MockGit::new(None));

    // Seed with a snippet
    let s1 = contracts::Prompt::new("test_snippet".to_string(), contracts::PromptType::Snippet, None, Some("ts".to_string()));
    storage.save_global_snippets(vec![s1]).await.unwrap();

    let mut app = App::new(storage, clipboard, git);
    
    app.enter_editor("Hello ".to_string(), None);
    
    // Move to end of "Hello "
    app.textarea.move_cursor(ratatui_textarea::CursorMove::End);

    // Type '$$t'
    for c in "$$t".chars() {
        app.textarea.input(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char(c),
            crossterm::event::KeyModifiers::empty(),
        ));
    }
    app.update_autocomplete().await.unwrap();
    
    assert!(app.autocomplete_open);
    assert_eq!(app.suggestions.len(), 1);
    assert_eq!(app.suggestions[0].name, Some("ts".to_string()));

    // Select it
    app.select_suggestion();
    assert!(!app.autocomplete_open);
    assert_eq!(app.textarea.lines()[0], "Hello $$ts");
}

#[tokio::test]
async fn test_editor_discard_confirmation_modal() {
    let storage = Arc::new(InMemoryStorage::new());
    let clipboard = Arc::new(MockClipboard::new());
    let git = Arc::new(MockGit::new(None));
    let mut app = App::new(storage, clipboard, git);

    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    // 1. Enter editor and modify text
    app.enter_editor("Original".to_string(), None);
    app.textarea.insert_str("Modified");
    
    // 2. Simulate Esc key
    let event = crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::empty(),
    ));
    
    // Handle Esc in Editor mode
    if let crossterm::event::Event::Key(key) = event {
        if key.code == crossterm::event::KeyCode::Esc {
            let current_text = app.textarea.lines().join("\n");
            if current_text != app.original_text {
                app.mode = app::app::Mode::ConfirmDiscard;
            }
        }
    }

    assert_eq!(app.mode, app::app::Mode::ConfirmDiscard);

    // 3. Render and verify that the modal is present AND editor content is still there
    terminal
        .draw(|f| {
            ui::render(
                f,
                app.active_tab,
                &app.prompts,
                app.selected_index,
                "Confirm Discard",
                &app.textarea,
                None,
                &[],
                0,
                &mut None,
                "",
                "",
                &contracts::Settings::default(),
            );
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    
    // Verify "Discard Changes?" title exists (modal)
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
    assert!(found_modal_title, "Modal title 'Discard Changes?' not found");

    // Verify editor content "Original" is visible
    let mut found_original = false;
    let mut found_modified = false;
    for y in 0..30 {
        let mut line = String::new();
        for x in 0..80 {
            line.push_str(buffer[(x, y)].symbol());
        }
        if line.contains("Original") {
            found_original = true;
        }
        if line.contains("Modified") {
            found_modified = true;
        }
    }
    assert!(found_original, "Editor content 'Original' not found in buffer");
    assert!(found_modified, "Editor content 'Modified' not found in buffer");
}







