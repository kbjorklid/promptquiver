mod common;
use common::setup_app;
use contracts::{Tab, Storage};

#[tokio::test]
async fn test_settings_navigation_and_tab_focus() {
    let (mut app, _, _, _) = setup_app();

    app.set_tab(Tab::Settings);
    app.load_prompts().await.unwrap();

    assert_eq!(app.selected_index, 0);
    app.move_down();
    assert_eq!(app.selected_index, 1);
    
    for _ in 0..10 {
        app.move_down();
    }
    assert_eq!(app.selected_index, 8);

    app.move_up();
    assert_eq!(app.selected_index, 7);
}

#[tokio::test]
async fn test_edit_slash_commands_inline() {
    let (mut app, storage, _, _) = setup_app();

    app.settings.slash_commands = vec!["test".to_string()];
    storage.save_settings(app.settings.clone()).await.unwrap();

    app.set_tab(Tab::Settings);
    app.load_prompts().await.unwrap();

    let tabs_len = Tab::all().len();
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
    
    if app.active_tab == contracts::Tab::Settings {
        app.exit_editor();
    }

    assert_eq!(app.mode, app::app::Mode::List);
}
