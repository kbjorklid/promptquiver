mod common;
use common::setup_app;
use contracts::{Storage, Tab};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use promptquiver::handlers::handle_key_event;
use ui::list_module::ListModule;

const fn enter() -> KeyEvent {
    KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    }
}

#[tokio::test]
async fn test_settings_navigation_and_tab_focus() {
    let (mut app, _, _, _) = setup_app();

    app.set_tab(Tab::Settings);
    app.load_prompts().await.unwrap();

    assert_eq!(app.nav.selected_index, 0);
    app.move_down();
    assert_eq!(app.nav.selected_index, 1);

    for _ in 0..15 {
        app.move_down();
    }
    let total = app.nav.total_settings_count(&app.settings);
    assert_eq!(app.nav.selected_index, total - 1);

    app.move_up();
    assert_eq!(app.nav.selected_index, total - 2);
}

#[tokio::test]
async fn test_edit_slash_commands_inline() {
    let (mut app, storage, _, _) = setup_app();

    app.settings.slash_commands = vec!["test".to_string()];
    storage.save_settings(app.settings.clone()).await.unwrap();

    app.set_tab(Tab::Settings);
    app.load_prompts().await.unwrap();

    let tabs_len = Tab::settings_display_len();
    app.nav.selected_index = tabs_len; // First Slash Command ("test")

    app.edit_setting();
    assert_eq!(app.mode, promptquiver::app::Mode::Editor);
    assert_eq!(app.editor.textarea.lines().join("\n"), "test");

    app.editor.textarea = ratatui_textarea::TextArea::from(vec!["updated".to_string()]);
    app.save_editor().await.unwrap();

    assert_eq!(app.mode, promptquiver::app::Mode::List);
    let updated_settings = storage.get_settings().await.unwrap();
    assert_eq!(updated_settings.slash_commands, vec!["updated".to_string()]);

    // Test Adding new
    app.nav.selected_index = tabs_len + 1; // "Add New"
    app.edit_setting();
    app.editor.textarea = ratatui_textarea::TextArea::from(vec!["new".to_string()]);
    app.save_editor().await.unwrap();

    let updated_settings = storage.get_settings().await.unwrap();
    assert_eq!(updated_settings.slash_commands, vec!["updated".to_string(), "new".to_string()]);
}

#[tokio::test]
async fn test_save_slash_command_with_enter() {
    let (mut app, storage, _, _) = setup_app();

    app.set_tab(contracts::Tab::Settings);
    app.load_prompts().await.unwrap();

    let tabs_len = contracts::Tab::settings_display_len();
    app.nav.selected_index = tabs_len; // Add New

    app.edit_setting();
    app.editor.textarea.insert_str("enter_save");

    // Simulate Enter key
    let enter_key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyModifiers::empty(),
    );
    app.handle_message(promptquiver::app::AppMessage::EditorInput(enter_key)).await.unwrap();

    assert_eq!(app.mode, promptquiver::app::Mode::List);
    let updated_settings = storage.get_settings().await.unwrap();
    assert!(updated_settings.slash_commands.contains(&"enter_save".to_string()));
}

#[tokio::test]
async fn test_coverage_boost_settings_render() {
    let (mut app, _, _, _) = setup_app();
    let backend = ratatui::backend::TestBackend::new(80, 10); // Small height to trigger scrolling
    let mut terminal = ratatui::Terminal::new(backend).unwrap();

    app.set_tab(Tab::Settings);
    app.settings.startup_behavior = contracts::StartupBehavior::Specific;
    app.settings.slash_commands = vec!["one".into(), "two".into(), "three".into()];
    app.load_prompts().await.unwrap();

    // Scroll down
    for _ in 0..10 {
        app.move_down();
    }

    terminal
        .draw(|f| {
            ui::render(
                f,
                ui::RenderState {
                    nav: &mut app.nav,
                    editor: &mut app.editor,
                    mode: app.mode,
                    settings: &app.settings,
                    current_branch: None,
                    show_help: app.show_help,
                    help_scroll: app.help_scroll,
                },
                &mut None,
            );
        })
        .unwrap();

    // Edit slash command (textarea rendering)
    app.nav.selected_index = Tab::settings_display_len();
    app.mode = promptquiver::app::Mode::Editor;
    let mut ta = ratatui_textarea::TextArea::default();
    ta.insert_str("edit");
    app.editor.textarea = ta;

    terminal
        .draw(|f| {
            ui::render(
                f,
                ui::RenderState {
                    nav: &mut app.nav,
                    editor: &mut app.editor,
                    mode: app.mode,
                    settings: &app.settings,
                    current_branch: None,
                    show_help: app.show_help,
                    help_scroll: app.help_scroll,
                },
                &mut None,
            );
        })
        .unwrap();
}

#[tokio::test]
async fn test_slash_command_colon_and_leading_slash() {
    let (mut app, _, _, _) = setup_app();

    app.set_tab(Tab::Settings);
    app.load_prompts().await.unwrap();

    let tabs_len = Tab::settings_display_len();
    let slash_len = app.settings.slash_commands.len();
    app.nav.selected_index = tabs_len + slash_len; // "Add New Slash Command"

    // Case 1: Try to add with colon (should succeed now)
    app.edit_setting();
    app.editor.textarea = ratatui_textarea::TextArea::from(vec!["fix:bug".to_string()]);
    app.save_editor().await.unwrap();

    assert!(app.settings.slash_commands.contains(&"fix:bug".to_string()), "Colon should succeed");

    // Case 2: Try to add with leading slash (should be stripped and succeed)
    app.nav.selected_index = tabs_len + app.settings.slash_commands.len(); // Next "Add New"
    app.edit_setting();
    app.editor.textarea = ratatui_textarea::TextArea::from(vec!["/fix".to_string()]);
    app.save_editor().await.unwrap();
    assert!(
        app.settings.slash_commands.contains(&"fix".to_string()),
        "Leading slash should be stripped"
    );
    assert!(!app.settings.slash_commands.contains(&"/fix".to_string()));

    // Case 3: Verify autocomplete works with colon in query
    app.set_tab(Tab::Prompts);
    app.mode = promptquiver::app::Mode::Editor;
    app.editor.textarea = ratatui_textarea::TextArea::default();
    app.editor.textarea.insert_str("/fix:");
    app.update_autocomplete().await.unwrap();
    assert!(app.editor.autocomplete.open);
    assert!(app.editor.autocomplete.suggestions.iter().any(|p| p.text == "fix:bug"));
}

#[tokio::test]
async fn test_enter_on_nerd_font_row_toggles_nerd_font() {
    let (mut app, _, _, _) = setup_app();
    app.set_tab(Tab::Settings);
    app.load_prompts().await.unwrap();

    let slash_len = app.settings.slash_commands.len();
    let (_, advanced_start) = ListModule::settings_section_offsets(slash_len);
    app.nav.selected_index = advanced_start + 2; // Nerd Font Icons

    let original = app.settings.use_nerd_font;
    handle_key_event(&mut app, enter()).await;

    assert_ne!(app.settings.use_nerd_font, original, "Enter on Nerd Font row must toggle it");
    assert_ne!(app.mode, promptquiver::app::Mode::ThemePicker, "must NOT open theme picker");
}

#[tokio::test]
async fn test_enter_on_theme_row_opens_theme_picker() {
    let (mut app, _, _, _) = setup_app();
    app.set_tab(Tab::Settings);
    app.load_prompts().await.unwrap();

    let slash_len = app.settings.slash_commands.len();
    let (_, advanced_start) = ListModule::settings_section_offsets(slash_len);
    app.nav.selected_index = advanced_start + 3; // Theme

    handle_key_event(&mut app, enter()).await;

    assert_eq!(
        app.mode,
        promptquiver::app::Mode::ThemePicker,
        "Enter on Theme row must open theme picker"
    );
}

#[tokio::test]
async fn test_enter_on_startup_behavior_row_toggles_behavior() {
    let (mut app, _, _, _) = setup_app();
    app.set_tab(Tab::Settings);
    app.load_prompts().await.unwrap();

    let slash_len = app.settings.slash_commands.len();
    let (_, advanced_start) = ListModule::settings_section_offsets(slash_len);
    app.nav.selected_index = advanced_start + 4; // Startup Behavior

    let original = app.settings.startup_behavior;
    handle_key_event(&mut app, enter()).await;

    assert_ne!(
        app.settings.startup_behavior, original,
        "Enter on Startup Behavior row must toggle it"
    );
    assert_ne!(app.mode, promptquiver::app::Mode::ProjectPicker, "must NOT open project picker");
}

#[tokio::test]
async fn test_enter_on_startup_project_row_opens_project_picker() {
    let (mut app, _, _, _) = setup_app();
    app.set_tab(Tab::Settings);
    app.load_prompts().await.unwrap();

    app.settings.startup_behavior = contracts::StartupBehavior::Specific;

    let slash_len = app.settings.slash_commands.len();
    let (_, advanced_start) = ListModule::settings_section_offsets(slash_len);
    app.nav.selected_index = advanced_start + 5; // Startup Project (only visible when Specific)

    handle_key_event(&mut app, enter()).await;

    assert_eq!(
        app.mode,
        promptquiver::app::Mode::ProjectPicker,
        "Enter on Startup Project row must open project picker"
    );
    assert!(
        app.nav.projects_manager.selecting_startup_project,
        "must be selecting startup project, not active project"
    );
}
