mod common;
use common::setup_app;
use contracts::{Storage, Clipboard};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[tokio::test]
async fn test_basic_render() {
    let (mut app, _, _, _) = setup_app();

    let backend = TestBackend::new(40, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let mode_str = match app.mode {
                promptquiver::app::Mode::List => "List",
                promptquiver::app::Mode::Editor => "Editor",
                promptquiver::app::Mode::Move => "Move",
                promptquiver::app::Mode::Search => "Search",
                promptquiver::app::Mode::GlobalSearch => "Global Search",
                promptquiver::app::Mode::ConfirmDiscard => "Confirm Discard",
                promptquiver::app::Mode::ThemePicker => "Theme Picker",
            };
            ui::render(
                f,
                ui::RenderState {
                    active_tab: app.active_tab,
                    prompts: &app.prompts,
                    selected_index: app.selected_index,
                    list_state: &mut app.list_state,
                    settings_slash_list_state: &mut app.settings_slash_list_state,
                    theme_list_state: &mut app.theme_list_state,
                    mode: mode_str,
                    textarea: &mut app.textarea,
                    title_textarea: &mut app.title_textarea,
                    title_focused: app.title_focused,
                    current_branch: app.current_branch.as_deref(),
                    current_path: &app.current_path,
                    suggestions: &app.suggestions,
                    suggestion_index: app.suggestion_index,
                    autocomplete_list_state: &mut app.autocomplete_list_state,
                    search_query: &app.search_query,
                    global_search_query: &app.global_search_query,
                    settings: &app.settings,
                },
                &mut None,
            );
        })
        .unwrap();

    let buffer = terminal.backend().buffer();

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
async fn test_staging() {
    let (mut app, storage, clipboard, _) = setup_app();
    
    let p1 = contracts::Prompt::new("P1".to_string(), contracts::PromptType::Prompt, None, None);
    let p2 = contracts::Prompt::new("P2".to_string(), contracts::PromptType::Prompt, None, None);
    storage.save_project_prompts(common::TEST_PATH, vec![p1, p2]).await.unwrap();

    app.load_prompts().await.unwrap();

    app.stage_selected().await.unwrap();
    assert!(app.prompts[0].staged);
    assert_eq!(clipboard.paste().await.unwrap(), "P1");

    app.move_down();
    app.stage_selected().await.unwrap();
    
    assert_eq!(app.prompts.len(), 1);
    assert_eq!(app.prompts[0].text, "P2");
    assert!(app.prompts[0].staged);
    assert_eq!(clipboard.paste().await.unwrap(), "P2");

    let archive = storage.get_project_archive(common::TEST_PATH).await.unwrap();
    assert_eq!(archive.len(), 1);
    assert_eq!(archive[0].text, "P1");
}

#[tokio::test]
async fn test_unstaging() {
    let (mut app, storage, _, _) = setup_app();
    
    let mut p1 = contracts::Prompt::new("P1".to_string(), contracts::PromptType::Prompt, None, None);
    p1.staged = true;
    storage.save_project_prompts(common::TEST_PATH, vec![p1]).await.unwrap();

    app.load_prompts().await.unwrap();
    assert!(app.prompts[0].staged);

    // Unstage
    app.stage_selected().await.unwrap();
    assert!(!app.prompts[0].staged, "Should be unstaged in memory");

    // Verify persistence
    let stored = storage.get_project_prompts(common::TEST_PATH).await.unwrap();
    assert!(!stored[0].staged, "Should be unstaged in storage");
}

#[tokio::test]
async fn test_archive_delete() {
    let (mut app, storage, _, _) = setup_app();
    
    let p1 = contracts::Prompt::new("P1".to_string(), contracts::PromptType::Prompt, None, None);
    storage.save_project_prompts(common::TEST_PATH, vec![p1]).await.unwrap();

    app.load_prompts().await.unwrap();

    app.archive_selected().await.unwrap();
    assert_eq!(app.prompts.len(), 0);

    app.set_tab(contracts::Tab::Archive);
    app.load_prompts().await.unwrap();
    assert_eq!(app.prompts.len(), 1);

    app.archive_selected().await.unwrap();
    assert_eq!(app.prompts.len(), 0);
    
    let archive = storage.get_project_archive(common::TEST_PATH).await.unwrap();
    assert_eq!(archive.len(), 0);
}
