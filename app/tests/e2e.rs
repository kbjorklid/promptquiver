use app::app::App;
use contracts::{Clipboard, Storage};
use infra::{InMemoryStorage, MockClipboard, MockGit};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::sync::Arc;

const TEST_PATH: &str = "test_project";

fn setup_app() -> (App<'static>, Arc<InMemoryStorage>, Arc<MockClipboard>, Arc<MockGit>) {
    let storage = Arc::new(InMemoryStorage::new());
    let clipboard = Arc::new(MockClipboard::new());
    let git = Arc::new(MockGit::new(None));
    let mut app = App::new(storage.clone(), clipboard.clone(), git.clone());
    app.current_path = TEST_PATH.to_string();
    (app, storage, clipboard, git)
}

#[tokio::test]
async fn test_basic_render() {
    let (app, _, _, _) = setup_app();

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
                ui::RenderState {
                    active_tab: app.active_tab,
                    prompts: &app.prompts,
                    selected_index: app.selected_index,
                    mode: mode_str,
                    textarea: &app.textarea,
                    title_textarea: &app.title_textarea,
                    title_focused: app.title_focused,
                    current_branch: app.current_branch.as_deref(),
                    suggestions: &app.suggestions,
                    suggestion_index: app.suggestion_index,
                    search_query: &app.search_query,
                    global_search_query: &app.global_search_query,
                    settings: &app.settings,
                },
                &mut None,
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
}

#[tokio::test]
async fn test_quit_event() {
    let (mut app, _, _, _) = setup_app();
    assert!(!app.should_quit);

    app.quit();
    assert!(app.should_quit);
}

#[tokio::test]
async fn test_tab_navigation() {
    let (mut app, _, _, _) = setup_app();
    assert_eq!(app.active_tab, contracts::Tab::Prompts);

    app.next_tab();
    assert_eq!(app.active_tab, contracts::Tab::Canned);

    app.next_tab();
    assert_eq!(app.active_tab, contracts::Tab::Notes);

    app.prev_tab();
    assert_eq!(app.active_tab, contracts::Tab::Canned);
}

#[tokio::test]
async fn test_list_navigation() {
    let (mut app, storage, _, _) = setup_app();

    // Seed storage with some prompts
    let prompts = vec![
        contracts::Prompt::new("Prompt 1".to_string(), contracts::PromptType::Prompt, None, None),
        contracts::Prompt::new("Prompt 2".to_string(), contracts::PromptType::Prompt, None, None),
    ];
    storage.save_project_prompts(TEST_PATH, prompts).await.unwrap();

    app.load_prompts().await.unwrap();

    assert_eq!(app.prompts.len(), 2);
    assert_eq!(app.selected_index, 0);

    app.move_down();
    assert_eq!(app.selected_index, 1);

    app.move_up();
    assert_eq!(app.selected_index, 0);
}

#[tokio::test]
async fn test_tab_specific_content() {
    let (mut app, storage, _, _) = setup_app();
    
    // Prompts
    storage.save_project_prompts(TEST_PATH, vec![
        contracts::Prompt::new("P1".to_string(), contracts::PromptType::Prompt, None, None)
    ]).await.unwrap();
    
    // Notes
    storage.save_project_notes(TEST_PATH, vec![
        contracts::Prompt::new("N1".to_string(), contracts::PromptType::Note, None, None)
    ]).await.unwrap();

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
    let (mut app, storage, clipboard, _) = setup_app();
    
    // Seed with two prompts
    let p1 = contracts::Prompt::new("P1".to_string(), contracts::PromptType::Prompt, None, None);
    let p2 = contracts::Prompt::new("P2".to_string(), contracts::PromptType::Prompt, None, None);
    storage.save_project_prompts(TEST_PATH, vec![p1, p2]).await.unwrap();

    app.load_prompts().await.unwrap();

    // Stage P1
    app.stage_selected().await.unwrap();
    assert!(app.prompts[0].staged);
    assert_eq!(clipboard.paste().await.unwrap(), "P1");

    // Move to P2 and stage it
    app.move_down();
    app.stage_selected().await.unwrap();
    
    // P2 should be staged, P1 should be archived
    assert_eq!(app.prompts.len(), 1);
    assert_eq!(app.prompts[0].text, "P2");
    assert!(app.prompts[0].staged);
    assert_eq!(clipboard.paste().await.unwrap(), "P2");

    // Check Archive
    let archive = storage.get_project_archive(TEST_PATH).await.unwrap();
    assert_eq!(archive.len(), 1);
    assert_eq!(archive[0].text, "P1");
}

#[tokio::test]
async fn test_add_edit_prompt() {
    let (mut app, _, _, _) = setup_app();
    
    // Add new prompt
    app.enter_editor("New Prompt".to_string(), None);
    app.save_editor().await.unwrap();
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
    let (mut app, storage, _, _) = setup_app();
    
    // Seed with a prompt
    let p1 = contracts::Prompt::new("P1".to_string(), contracts::PromptType::Prompt, None, None);
    storage.save_project_prompts(TEST_PATH, vec![p1]).await.unwrap();

    app.load_prompts().await.unwrap();

    // Archive it
    app.archive_selected().await.unwrap();
    assert_eq!(app.prompts.len(), 0);

    // Check Archive tab
    app.set_tab(contracts::Tab::Archive);
    app.load_prompts().await.unwrap();
    assert_eq!(app.prompts.len(), 1);

    // Delete permanently from Archive
    app.archive_selected().await.unwrap();
    assert_eq!(app.prompts.len(), 0);
    
    let archive = storage.get_project_archive(TEST_PATH).await.unwrap();
    assert_eq!(archive.len(), 0);
}

#[tokio::test]
async fn test_autocomplete() {
    let (mut app, storage, _, _) = setup_app();

    // Seed with a snippet
    let s1 = contracts::Prompt::new("test_snippet".to_string(), contracts::PromptType::Snippet, None, Some("ts".to_string()));
    storage.save_global_snippets(vec![s1]).await.unwrap();
    
    app.enter_editor("Hello ".to_string(), None);
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

    // Select it
    app.select_suggestion();
    assert_eq!(app.textarea.lines()[0], "Hello $$ts");
}

#[tokio::test]
async fn test_autocomplete_slash_command_title() {
    let (mut app, _, _, _) = setup_app();
    app.settings.slash_commands = vec!["/test".to_string()];
    app.storage.save_settings(app.settings.clone()).await.unwrap();

    app.enter_editor(" ".to_string(), None);
    app.textarea.move_cursor(ratatui_textarea::CursorMove::End);

    // Type '/'
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
async fn test_editor_discard_confirmation_modal() {
    let (mut app, _, _, _) = setup_app();

    // 1. Enter editor and modify text
    app.enter_editor("Original".to_string(), None);
    app.textarea.insert_str("Modified");
    
    // Simulate Esc
    let current_text = app.textarea.lines().join("\n");
    if current_text != app.original_text {
        app.mode = app::app::Mode::ConfirmDiscard;
    }

    assert_eq!(app.mode, app::app::Mode::ConfirmDiscard);

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
                    mode: "Confirm Discard",
                    textarea: &app.textarea,
                    title_textarea: &app.title_textarea,
                    title_focused: app.title_focused,
                    current_branch: None,
                    suggestions: &[],
                    suggestion_index: 0,
                    search_query: "",
                    global_search_query: "",
                    settings: &contracts::Settings::default(),
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
async fn test_settings_navigation_and_tab_focus() {
    let (mut app, _, _, _) = setup_app();

    // Switch to Settings tab
    app.set_tab(contracts::Tab::Settings);
    app.load_prompts().await.unwrap();

    assert_eq!(app.selected_index, 0);
    app.move_down();
    assert_eq!(app.selected_index, 1);
    
    // Total items: 8 (6 tabs + 2 extra)
    for _ in 0..10 {
        app.move_down();
    }
    assert_eq!(app.selected_index, 7);

    app.move_up();
    assert_eq!(app.selected_index, 6);
}

#[tokio::test]
async fn test_edit_slash_commands_inline() {
    let (mut app, storage, _, _) = setup_app();

    app.settings.slash_commands = vec!["test".to_string()];
    storage.save_settings(app.settings.clone()).await.unwrap();

    app.set_tab(contracts::Tab::Settings);
    app.load_prompts().await.unwrap();

    let tabs_len = contracts::Tab::all().len();
    app.selected_index = tabs_len; // First Slash Command ("test")

    app.edit_setting();
    assert_eq!(app.mode, app::app::Mode::Editor);
    assert_eq!(app.textarea.lines().join("\n"), "test");

    app.textarea = ratatui_textarea::TextArea::from(vec!["updated".to_string()]);
    app.save_editor().await.unwrap();

    assert_eq!(app.mode, app::app::Mode::List);
    let updated_settings = storage.get_settings().await.unwrap();
    assert_eq!(updated_settings.slash_commands, vec!["updated".to_string()]);

    // Test Adding new
    app.selected_index = tabs_len + 1; // "Add New"
    app.edit_setting();
    app.textarea = ratatui_textarea::TextArea::from(vec!["new".to_string()]);
    app.save_editor().await.unwrap();

    let updated_settings = storage.get_settings().await.unwrap();
    assert_eq!(updated_settings.slash_commands, vec!["updated".to_string(), "new".to_string()]);
}

#[tokio::test]
async fn test_settings_auto_discard_on_esc() {
    let (mut app, _, _, _) = setup_app();

    app.set_tab(contracts::Tab::Settings);
    app.load_prompts().await.unwrap();

    let tabs_len = contracts::Tab::all().len();
    app.selected_index = tabs_len; // First Slash Command (if any, or Add New)

    app.edit_setting();
    app.textarea.insert_str("modified");
    
    // In a real app, main.rs handles the Esc key. 
    // Here we simulate what main.rs does:
    if app.active_tab == contracts::Tab::Settings {
        app.exit_editor();
    }

    assert_eq!(app.mode, app::app::Mode::List);
}
