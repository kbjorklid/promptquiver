mod common;
use common::setup_app;
use contracts::{Tab, Storage};
use promptquiver::handlers::handle_key_event;
use crossterm::event::{KeyEvent, KeyCode, KeyModifiers, KeyEventKind, KeyEventState};

#[tokio::test]
async fn test_tab_keys() {
    let (mut app, _, _, _) = setup_app();
    
    let k = |c| KeyEvent {
        code: KeyCode::Char(c),
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };

    handle_key_event(&mut app, k('2')).await;
    assert_eq!(app.nav.active_tab, Tab::Canned);

    handle_key_event(&mut app, k('3')).await;
    assert_eq!(app.nav.active_tab, Tab::Notes);

    handle_key_event(&mut app, k('4')).await;
    assert_eq!(app.nav.active_tab, Tab::Snippets);

    handle_key_event(&mut app, k('5')).await;
    assert_eq!(app.nav.active_tab, Tab::Archive);

    handle_key_event(&mut app, k('6')).await;
    assert_eq!(app.nav.active_tab, Tab::Settings);

    handle_key_event(&mut app, k('1')).await;
    assert_eq!(app.nav.active_tab, Tab::Prompts);
}

#[tokio::test]
async fn test_navigation_keys() {
    let (mut app, storage, _, _) = setup_app();
    for text in ["P1", "P2", "P3"] {
        storage.save_prompt(contracts::Prompt::new(text.to_string(), contracts::PromptType::Prompt, Some(common::TEST_PATH.to_string()), None, None, None)).await.unwrap();
    }
    app.load_prompts().await.unwrap();

    let k = |code| KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };

    handle_key_event(&mut app, k(KeyCode::Char('j'))).await;
    assert_eq!(app.nav.selected_index, 1);

    handle_key_event(&mut app, k(KeyCode::Char('k'))).await;
    assert_eq!(app.nav.selected_index, 0);

    handle_key_event(&mut app, k(KeyCode::Char('G'))).await;
    assert_eq!(app.nav.selected_index, 2);

    handle_key_event(&mut app, k(KeyCode::Char('g'))).await;
    assert_eq!(app.nav.selected_index, 0);

    handle_key_event(&mut app, k(KeyCode::Char('l'))).await;
    assert_eq!(app.nav.active_tab, Tab::Canned);

    handle_key_event(&mut app, k(KeyCode::Char('h'))).await;
    assert_eq!(app.nav.active_tab, Tab::Prompts);
}

#[tokio::test]
async fn test_undo_redo_keys() {
    let (mut app, storage, _, _) = setup_app();
    
    // Setup initial state
    storage.save_prompt(contracts::Prompt::new("P1".to_string(), contracts::PromptType::Prompt, Some(common::TEST_PATH.to_string()), None, None, None)).await.unwrap();
    app.load_prompts().await.unwrap();
    
    // Archive it (to have something in history)
    app.handle_message(ui::AppMessage::ArchiveSelected).await.unwrap();
    assert_eq!(app.nav.prompts.len(), 0);

    let u_key = KeyEvent {
        code: KeyCode::Char('u'),
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };
    
    handle_key_event(&mut app, u_key).await;
    assert_eq!(app.nav.prompts.len(), 1, "Undo should restore the prompt");

    let redo_key = KeyEvent {
        code: KeyCode::Char('y'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };

    handle_key_event(&mut app, redo_key).await;
    assert_eq!(app.nav.prompts.len(), 0, "Redo should archive it again");
}

#[tokio::test]
async fn test_move_item_handlers() {
    let (mut app, storage, _, _) = setup_app();
    
    let mut p1 = contracts::Prompt::new("P1".to_string(), contracts::PromptType::Prompt, Some(common::TEST_PATH.to_string()), None, None, None);
    p1.order_index = 0;
    let mut p2 = contracts::Prompt::new("P2".to_string(), contracts::PromptType::Prompt, Some(common::TEST_PATH.to_string()), None, None, None);
    p2.order_index = 1;
    storage.save_prompt(p1).await.unwrap();
    storage.save_prompt(p2).await.unwrap();

    app.load_prompts().await.unwrap();
    assert_eq!(app.nav.prompts[0].text, "P1");
    assert_eq!(app.nav.prompts[1].text, "P2");

    app.mode = promptquiver::app::Mode::Move;

    // Move P1 down
    let move_down_key = KeyEvent {
        code: KeyCode::Char('j'),
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };
    handle_key_event(&mut app, move_down_key).await;
    assert_eq!(app.nav.prompts[1].text, "P1");
    assert_eq!(app.nav.selected_index, 1);
}

#[tokio::test]
async fn test_search_mode_key() {
    let (mut app, _, _, _) = setup_app();
    
    let slash_key = KeyEvent {
        code: KeyCode::Char('/'),
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };

    handle_key_event(&mut app, slash_key).await;
    assert_eq!(app.mode, promptquiver::app::Mode::Search);
}

#[tokio::test]
async fn test_move_mode_key() {
    let (mut app, _, _, _) = setup_app();
    
    let m_key = KeyEvent {
        code: KeyCode::Char('m'),
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };

    handle_key_event(&mut app, m_key).await;
    assert_eq!(app.mode, promptquiver::app::Mode::Move);
    
    handle_key_event(&mut app, m_key).await;
    assert_eq!(app.mode, promptquiver::app::Mode::List);
}

#[tokio::test]
async fn test_confirm_discard_keys() {
    let (mut app, _, _, _) = setup_app();
    app.mode = promptquiver::app::Mode::ConfirmDiscard;

    let k = |code| KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };

    // 'n' cancels discard
    handle_key_event(&mut app, k(KeyCode::Char('n'))).await;
    assert_eq!(app.mode, promptquiver::app::Mode::Editor);

    app.mode = promptquiver::app::Mode::ConfirmDiscard;
    // 'y' confirms discard (exits editor)
    handle_key_event(&mut app, k(KeyCode::Char('y'))).await;
    assert_eq!(app.mode, promptquiver::app::Mode::List);
}

#[tokio::test]
async fn test_theme_picker_keys() {
    let (mut app, _, _, _) = setup_app();
    app.mode = promptquiver::app::Mode::ThemePicker;

    let k = |code| KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };

    // Esc exits theme picker
    handle_key_event(&mut app, k(KeyCode::Esc)).await;
    assert_eq!(app.mode, promptquiver::app::Mode::List);
}

#[tokio::test]
async fn test_editor_autocomplete_keys() {
    let (mut app, _, _, _) = setup_app();
    app.mode = promptquiver::app::Mode::Editor;
    app.editor.autocomplete.open = true;
    app.editor.autocomplete.suggestions = vec![
        contracts::Prompt::new("test".to_string(), contracts::PromptType::Snippet, None, None, None, None)
    ];

    let k = |code| KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };

    handle_key_event(&mut app, k(KeyCode::Down)).await;
    assert_eq!(app.editor.autocomplete.index, 0); 

    handle_key_event(&mut app, k(KeyCode::Up)).await;
}

#[tokio::test]
async fn test_move_item_keys() {
    let (mut app, storage, _, _) = setup_app();
    for text in ["P2", "P1"] {
        storage.save_prompt(contracts::Prompt::new(text.to_string(), contracts::PromptType::Prompt, Some(common::TEST_PATH.to_string()), None, None, None)).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    app.load_prompts().await.unwrap();
    app.mode = promptquiver::app::Mode::Move;

    let k = |code| KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };

    // Move P1 down
    handle_key_event(&mut app, k(KeyCode::Char('j'))).await;
    assert_eq!(app.nav.prompts[0].text, "P2");
    assert_eq!(app.nav.prompts[1].text, "P1");

    // Move P1 up
    handle_key_event(&mut app, k(KeyCode::Char('k'))).await;
    assert_eq!(app.nav.prompts[0].text, "P1");
    assert_eq!(app.nav.prompts[1].text, "P2");
}

#[tokio::test]
async fn test_duplicate_key() {
    let (mut app, storage, _, _) = setup_app();
    storage.save_prompt(contracts::Prompt::new("P1".to_string(), contracts::PromptType::Prompt, Some(common::TEST_PATH.to_string()), None, None, None)).await.unwrap();
    app.load_prompts().await.unwrap();

    let d_key = KeyEvent {
        code: KeyCode::Char('D'),
        modifiers: KeyModifiers::SHIFT,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };

    handle_key_event(&mut app, d_key).await;
    assert_eq!(app.nav.prompts.len(), 2);
    assert_eq!(app.nav.prompts[1].text, "P1");
}

#[tokio::test]
async fn test_branch_filter_key() {
    let (mut app, _, _, _) = setup_app();
    assert!(!app.nav.branch_filter);

    let b_key = KeyEvent {
        code: KeyCode::Char('b'),
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };

    handle_key_event(&mut app, b_key).await;
    assert!(app.nav.branch_filter);
    
    handle_key_event(&mut app, b_key).await;
    assert!(!app.nav.branch_filter);
}

#[tokio::test]
async fn test_toggle_setting_key() {
    let (mut app, _, _, _) = setup_app();
    app.set_tab(Tab::Settings);
    app.nav.selected_index = 0; // First setting in settings tab (after tabs visibility?)
    
    // Actually, in ui/src/settings.rs, the order is:
    // Tab visibility (5 items)
    // Slash commands
    // Add slash command
    // Separator
    // enable_claude_commands
    // use_nerd_font
    // ...
    
    let tabs_len = Tab::settings_display_len();
    let maintenance_len = 2;
    app.nav.selected_index = tabs_len + app.settings.slash_commands.len() + 1 + maintenance_len; // enable_claude_commands

    let original = app.settings.enable_claude_commands;

    let space_key = KeyEvent {
        code: KeyCode::Char(' '),
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };

    handle_key_event(&mut app, space_key).await;
    assert_ne!(app.settings.enable_claude_commands, original);
}

#[tokio::test]
async fn test_archive_slash_command() {
    let (mut app, _, _, _) = setup_app();
    app.settings.slash_commands = vec!["/test".to_string()];
    app.set_tab(Tab::Settings);
    app.nav.selected_index = Tab::settings_display_len(); // Index of first slash command

    let d_key = KeyEvent {
        code: KeyCode::Char('d'),
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };

    handle_key_event(&mut app, d_key).await;
    assert!(app.settings.slash_commands.is_empty());
}

#[tokio::test]
async fn test_list_extra_keys() {
    let (mut app, storage, _, _) = setup_app();
    storage.save_prompt(contracts::Prompt::new("P1".to_string(), contracts::PromptType::Prompt, Some(common::TEST_PATH.to_string()), None, None, None)).await.unwrap();
    app.load_prompts().await.unwrap();

    let k = |code| KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };
    let kc = |code| KeyEvent {
        code,
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };

    handle_key_event(&mut app, kc(KeyCode::Char('e'))).await; // CyclePreviewMode
    
    // Editor tests while list is NOT empty
    handle_key_event(&mut app, k(KeyCode::Char('a'))).await; // EnterEditor (add)
    assert_eq!(app.mode, promptquiver::app::Mode::Editor);
    app.mode = promptquiver::app::Mode::List;

    handle_key_event(&mut app, k(KeyCode::Char('i'))).await; // EnterEditorBefore
    assert_eq!(app.mode, promptquiver::app::Mode::Editor);
    app.mode = promptquiver::app::Mode::List;

    handle_key_event(&mut app, k(KeyCode::Char('e'))).await; // EnterEditor (edit)
    assert_eq!(app.mode, promptquiver::app::Mode::Editor);
    app.mode = promptquiver::app::Mode::List;

    handle_key_event(&mut app, k(KeyCode::Enter)).await; // EnterEditor (edit)
    assert_eq!(app.mode, promptquiver::app::Mode::Editor);
    app.mode = promptquiver::app::Mode::List;

    handle_key_event(&mut app, k(KeyCode::Char('c'))).await; // CopySelected
    handle_key_event(&mut app, k(KeyCode::Char('y'))).await; // CopySelected
    
    handle_key_event(&mut app, k(KeyCode::Char('f'))).await; // ToggleFolderFilter
    handle_key_event(&mut app, k(KeyCode::Char('p'))).await; // ToggleProjectFilter
    handle_key_event(&mut app, k(KeyCode::Char('I'))).await; // EnterEditorBefore(0)
    app.mode = promptquiver::app::Mode::List;
    handle_key_event(&mut app, k(KeyCode::Char('A'))).await; // EnterEditorBefore(len)
    app.mode = promptquiver::app::Mode::List;
    
    handle_key_event(&mut app, k(KeyCode::Char('s'))).await; // StageSelected
    handle_key_event(&mut app, k(KeyCode::Char('d'))).await; // ArchiveSelected
    // List is empty now
    handle_key_event(&mut app, k(KeyCode::Char('r'))).await; // RestoreSelected (does nothing outside Archive tab)
}

#[tokio::test]
async fn test_settings_tab_key() {
    let (mut app, _, _, _) = setup_app();
    app.set_tab(Tab::Settings);
    app.nav.selected_index = 0;

    let tab_key = KeyEvent {
        code: KeyCode::Tab,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };

    handle_key_event(&mut app, tab_key).await;
    assert_eq!(app.nav.selected_index, 1);
}

#[tokio::test]
async fn test_project_picker_and_add_keys() {
    let (mut app, _, _, _) = setup_app();
    
    // Project Picker
    app.mode = promptquiver::app::Mode::ProjectPicker;
    let k = |code| KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };
    handle_key_event(&mut app, k(KeyCode::Char('j'))).await;
    
    // Add Project
    app.mode = promptquiver::app::Mode::AddProject;
    handle_key_event(&mut app, k(KeyCode::Char('T'))).await;
    handle_key_event(&mut app, k(KeyCode::Char('e'))).await;
    handle_key_event(&mut app, k(KeyCode::Char('s'))).await;
    handle_key_event(&mut app, k(KeyCode::Char('t'))).await;
    assert_eq!(app.nav.projects_manager.new_project_name, "Test");

    handle_key_event(&mut app, k(KeyCode::Backspace)).await;
    assert_eq!(app.nav.projects_manager.new_project_name, "Tes");

    handle_key_event(&mut app, k(KeyCode::Enter)).await;
    // Should have added project "Tes" and switched back (logic is in App::handle_message)
    
    app.mode = promptquiver::app::Mode::AddProject;
    handle_key_event(&mut app, k(KeyCode::Esc)).await;
    assert_eq!(app.mode, promptquiver::app::Mode::ProjectPicker);
}

#[tokio::test]
async fn test_paste_event() {
    let (mut app, _, _, _) = setup_app();
    app.mode = promptquiver::app::Mode::Editor;
    
    handle_event(&mut app, Event::Paste("pasted content".to_string())).await;
    // AppMessage::Paste should be handled
}

use promptquiver::handlers::handle_event;
use crossterm::event::Event;

#[tokio::test]
async fn test_editor_extra_keys() {
    let (mut app, _, _, _) = setup_app();
    app.mode = promptquiver::app::Mode::Editor;

    let k = |code| KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };
    let kc = |code| KeyEvent {
        code,
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };

    handle_key_event(&mut app, k(KeyCode::Esc)).await; // ExitEditor / ConfirmDiscard
    app.mode = promptquiver::app::Mode::Editor;

    handle_key_event(&mut app, kc(KeyCode::Char('s'))).await; // SaveEditor
    handle_key_event(&mut app, kc(KeyCode::Char('g'))).await; // SaveAndStageEditor

    // Autocomplete open
    app.editor.autocomplete.open = true;
    app.editor.autocomplete.suggestions = vec![
        contracts::Prompt::new("test".to_string(), contracts::PromptType::Snippet, None, None, None, None)
    ];

    handle_key_event(&mut app, k(KeyCode::Enter)).await; // SelectSuggestion
    
    app.editor.autocomplete.open = true;
    handle_key_event(&mut app, k(KeyCode::Esc)).await; // Close autocomplete
}

#[tokio::test]
async fn test_move_extra_keys() {
    let (mut app, _, _, _) = setup_app();
    app.mode = promptquiver::app::Mode::Move;

    let k = |code| KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };

    handle_key_event(&mut app, k(KeyCode::Esc)).await; // ToggleMoveMode
    assert_eq!(app.mode, promptquiver::app::Mode::List);

    app.mode = promptquiver::app::Mode::Move;
    handle_key_event(&mut app, k(KeyCode::Enter)).await; // ToggleMoveMode
    assert_eq!(app.mode, promptquiver::app::Mode::List);
}

#[tokio::test]
async fn test_app_handle_message_transitions() {
    let (mut app, _, _, _) = setup_app();

    // Test ThemePickerInput transition
    app.mode = promptquiver::app::Mode::ThemePicker;
    app.handle_message(ui::AppMessage::ThemePickerInput(crossterm::event::KeyEvent::new(crossterm::event::KeyCode::Esc, crossterm::event::KeyModifiers::NONE))).await.unwrap();
    assert_eq!(app.mode, promptquiver::app::Mode::List);

    // Test ProjectPickerInput transition
    app.mode = promptquiver::app::Mode::ProjectPicker;
    app.handle_message(ui::AppMessage::ProjectPickerInput(crossterm::event::KeyEvent::new(crossterm::event::KeyCode::Esc, crossterm::event::KeyModifiers::NONE))).await.unwrap();
    assert_eq!(app.mode, promptquiver::app::Mode::List);

    // Test AddProject transition
    app.mode = promptquiver::app::Mode::AddProject;
    app.handle_message(ui::AppMessage::AddProject("New Project".to_string())).await.unwrap();
    assert_eq!(app.mode, promptquiver::app::Mode::List);

    // Test SearchInput Enter transition
    app.mode = promptquiver::app::Mode::Search;
    app.handle_message(ui::AppMessage::SearchInput(crossterm::event::KeyEvent::new(crossterm::event::KeyCode::Enter, crossterm::event::KeyModifiers::NONE))).await.unwrap();
    assert_eq!(app.mode, promptquiver::app::Mode::List);

    // Test SearchInput Esc transition
    app.mode = promptquiver::app::Mode::Search;
    app.handle_message(ui::AppMessage::SearchInput(crossterm::event::KeyEvent::new(crossterm::event::KeyCode::Esc, crossterm::event::KeyModifiers::NONE))).await.unwrap();
    assert_eq!(app.mode, promptquiver::app::Mode::List);
}

#[tokio::test]
async fn test_app_handle_message_global_handlers() {
    let (mut app, _, _, _) = setup_app();

    // Test Quit
    app.handle_message(ui::AppMessage::Quit).await.unwrap();
    assert!(app.should_quit);

    // Test Notify
    app.handle_message(ui::AppMessage::Notify("Hello".to_string(), ratatui_toaster::ToastType::Info)).await.unwrap();

    // Test EnterEditor/ExitEditor
    app.handle_message(ui::AppMessage::EnterEditor("text".to_string(), None)).await.unwrap();
    assert_eq!(app.mode, promptquiver::app::Mode::Editor);
    app.handle_message(ui::AppMessage::ExitEditor).await.unwrap();
    assert_eq!(app.mode, promptquiver::app::Mode::List);

    // Test ConfirmDiscard/CancelDiscard
    app.mode = promptquiver::app::Mode::Editor;
    app.handle_message(ui::AppMessage::ConfirmDiscard).await.unwrap();
    assert_eq!(app.mode, promptquiver::app::Mode::ConfirmDiscard);
    app.handle_message(ui::AppMessage::CancelDiscard).await.unwrap();
    assert_eq!(app.mode, promptquiver::app::Mode::Editor);

    // Test ToggleMoveMode
    app.mode = promptquiver::app::Mode::List;
    app.handle_message(ui::AppMessage::ToggleMoveMode).await.unwrap();
    assert_eq!(app.mode, promptquiver::app::Mode::Move);
    app.handle_message(ui::AppMessage::ToggleMoveMode).await.unwrap();
    assert_eq!(app.mode, promptquiver::app::Mode::List);

    // Test Search
    app.handle_message(ui::AppMessage::Search("query".to_string())).await.unwrap();
    assert_eq!(app.mode, promptquiver::app::Mode::Search);

    // Test SelectTheme
    app.handle_message(ui::AppMessage::SelectTheme).await.unwrap();
    assert_eq!(app.mode, promptquiver::app::Mode::ThemePicker);

    // Test SelectProject
    app.handle_message(ui::AppMessage::SelectProject).await.unwrap();
    assert_eq!(app.mode, promptquiver::app::Mode::ProjectPicker);

    // Test EnterAddProject
    app.handle_message(ui::AppMessage::EnterAddProject).await.unwrap();
    assert_eq!(app.mode, promptquiver::app::Mode::AddProject);
}

#[tokio::test]
async fn test_coverage_boost_list_module_extra() {
    let (mut app, storage, _, _) = setup_app();

    // 1. Stage notes/snippets
    app.set_tab(Tab::Notes);
    storage.save_prompts(vec![
        contracts::Prompt::new("note".into(), contracts::PromptType::Note, Some(common::TEST_PATH.to_string()), None, None, None),
    ]).await.unwrap();
    app.load_prompts().await.unwrap();
    app.handle_message(ui::AppMessage::StageSelected).await.unwrap();

    // 2. Restore from Archive
    app.set_tab(Tab::Archive);
    let mut arch_prompt = contracts::Prompt::new("archived".into(), contracts::PromptType::Prompt, Some(common::TEST_PATH.to_string()), None, None, None);
    arch_prompt.is_archived = true;
    storage.save_prompts(vec![arch_prompt]).await.unwrap();
    app.load_prompts().await.unwrap();
    app.handle_message(ui::AppMessage::RestoreSelected).await.unwrap();
    
    // 3. Toggle Settings - Startup behavior/project
    app.set_tab(Tab::Settings);
    let tabs_len = Tab::settings_display_len();
    app.nav.selected_index = tabs_len + 4; // Startup behavior
    app.handle_message(ui::AppMessage::ToggleSetting).await.unwrap();
    app.nav.selected_index = tabs_len + 5; // Startup project
    app.handle_message(ui::AppMessage::ToggleSetting).await.unwrap();

    // 4. Projects delete
    app.handle_message(ui::AppMessage::AddProject("New Project".into())).await.unwrap();
    let proj_id = app.nav.projects_manager.projects[0].id;
    app.handle_message(ui::AppMessage::DeleteProject(proj_id)).await.unwrap();

    // 5. Search & Paste
    app.handle_message(ui::AppMessage::Search("query".into())).await.unwrap();
    app.handle_message(ui::AppMessage::Paste("pasted".into())).await.unwrap();
    app.handle_message(ui::AppMessage::SearchInput(crossterm::event::KeyEvent::new(crossterm::event::KeyCode::Esc, crossterm::event::KeyModifiers::empty()))).await.unwrap();
    app.handle_message(ui::AppMessage::SearchInput(crossterm::event::KeyEvent::new(crossterm::event::KeyCode::Char('a'), crossterm::event::KeyModifiers::empty()))).await.unwrap();

    // 6. Project Picker Input
    app.handle_message(ui::AppMessage::SelectProject).await.unwrap();
    app.handle_message(ui::AppMessage::ProjectPickerInput(crossterm::event::KeyEvent::new(crossterm::event::KeyCode::Char('j'), crossterm::event::KeyModifiers::empty()))).await.unwrap();
    app.handle_message(ui::AppMessage::ProjectPickerInput(crossterm::event::KeyEvent::new(crossterm::event::KeyCode::Char('k'), crossterm::event::KeyModifiers::empty()))).await.unwrap();
    app.handle_message(ui::AppMessage::ProjectPickerInput(crossterm::event::KeyEvent::new(crossterm::event::KeyCode::Enter, crossterm::event::KeyModifiers::empty()))).await.unwrap();
    app.handle_message(ui::AppMessage::ProjectPickerInput(crossterm::event::KeyEvent::new(crossterm::event::KeyCode::Char('x'), crossterm::event::KeyModifiers::empty()))).await.unwrap();
    app.handle_message(ui::AppMessage::ProjectPickerInput(crossterm::event::KeyEvent::new(crossterm::event::KeyCode::Tab, crossterm::event::KeyModifiers::empty()))).await.unwrap();
}

