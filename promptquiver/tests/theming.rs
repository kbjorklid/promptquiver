use contracts::Tab;
use promptquiver::app::{App, Mode};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

mod common;

#[tokio::test]
async fn test_theming_selection() {
    let (mut app, _storage, _clipboard, _git) = common::setup_app();
    let backend = TestBackend::new(100, 50);
    let mut terminal = Terminal::new(backend).unwrap();

    // 1. Go to Settings tab
    app.set_tab(Tab::Settings);
    app.load_prompts().await.unwrap();

    // 2. Select Theme item in Advanced section
    // Tab Visibility (5 items) + Slash Commands (0 initial + 1 Add New) + Maintenance (2) + Claude (1) + Nerd (1) + Theme (1)
    // selected_index for Theme should be 5 + 1 + 2 + 2 = 10
    app.nav.selected_index = 10;

    // 3. Open Theme Picker
    app.mode = Mode::ThemePicker;

    // Render once to check
    terminal
        .draw(|f| {
            ui::render(f, app_to_render_state(&mut app), &mut None);
        })
        .unwrap();

    // 4. Select a theme (e.g., the second one)
    app.nav.theme_list_state.select(Some(1));

    // Simulate Enter to select theme
    let themes = ratatui_themes::ThemeName::all();
    let theme_name = format!("{:?}", themes[1]);
    app.settings.theme_name = Some(theme_name.clone());
    app.mode = Mode::List;

    // 5. Verify theme is updated in settings
    assert_eq!(app.settings.theme_name, Some(theme_name));

    // 6. Render and verify background color (e.g. for Nord theme)
    // Note: Colors depend on the theme palette.
    terminal
        .draw(|f| {
            ui::render(f, app_to_render_state(&mut app), &mut None);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let palette = ui::utils::get_palette(app.settings.theme_name.as_deref());

    // Check if some cell has the theme's background color
    // Usually cells in the list area
    let sample_cell = &buffer[(10, 10)];
    assert_eq!(sample_cell.bg, palette.bg);
}

#[tokio::test]
async fn test_theme_picker_opening_and_dismissal() {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use promptquiver::handlers::handle_key_event;

    let (mut app, _, _, _) = common::setup_app();

    app.set_tab(Tab::Settings);
    app.load_prompts().await.unwrap();

    // 1. Verify opening theme picker via Enter
    let tabs_len = Tab::settings_display_len();
    let slash_len = app.settings.slash_commands.len();
    let maintenance_len = 2;
    let theme_idx = tabs_len + slash_len + 1 + maintenance_len + 2; // theme item index: 5 + slash + 1 (Add New) + 2 (Maintenance) + 2 (claude, nerd) = 10 + slash
    app.nav.selected_index = theme_idx;

    let enter_key = KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };

    handle_key_event(&mut app, enter_key).await;
    assert_eq!(app.mode, Mode::ThemePicker, "Enter on theme setting should open theme picker");

    // 2. Verify dismissal via Enter inside theme picker
    handle_key_event(&mut app, enter_key).await;
    assert_eq!(app.mode, Mode::List, "Enter in Theme Picker should return to List mode");

    // 3. Verify opening again and dismissal via Esc
    app.nav.selected_index = theme_idx;
    handle_key_event(&mut app, enter_key).await;
    assert_eq!(app.mode, Mode::ThemePicker);

    let esc_key = KeyEvent {
        code: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    };
    handle_key_event(&mut app, esc_key).await;
    assert_eq!(app.mode, Mode::List, "Esc in Theme Picker should return to List mode");
}

#[tokio::test]
async fn test_theme_preview_preserved_on_reload() {
    use contracts::Storage;
    let (mut app, storage, _, _) = common::setup_app();

    // Set initial theme
    app.settings.theme_name = Some("Default".to_string());
    storage.save_settings(app.settings.clone()).await.unwrap();

    // Enter theme picker
    app.handle_message(ui::AppMessage::SelectTheme).await.unwrap();
    assert_eq!(app.mode, Mode::ThemePicker);

    // Simulate moving to a new theme (preview)
    app.settings.theme_name = Some("Nord".to_string());

    // Simulate background reload
    app.handle_message(ui::AppMessage::ReloadPrompts).await.unwrap();

    // Theme should still be Nord (the preview)
    assert_eq!(app.settings.theme_name, Some("Nord".to_string()));

    // Exit theme picker (Esc) - should revert to original
    app.handle_message(ui::AppMessage::ThemePickerInput(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::empty(),
    )))
    .await
    .unwrap();

    assert_eq!(app.settings.theme_name, Some("Default".to_string()));
}

#[tokio::test]
async fn test_theme_change_persistence() {
    use contracts::Storage;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ui::AppMessage;
    let (mut app, storage, _, _) = common::setup_app();

    // 1. Initial theme should be None
    assert_eq!(app.settings.theme_name, None);

    // 2. Open Theme Picker
    app.handle_message(AppMessage::SelectTheme).await.unwrap();
    assert_eq!(app.mode, Mode::ThemePicker);

    // 3. Move to the second theme (index 1)
    let down_key = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
    app.handle_message(AppMessage::ThemePickerInput(down_key)).await.unwrap();

    let themes = ratatui_themes::ThemeName::all();
    let second_theme = format!("{:?}", themes[1]);
    assert_eq!(app.settings.theme_name, Some(second_theme.clone()));

    // 4. Press Enter to confirm selection
    let enter_key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    app.handle_message(AppMessage::ThemePickerInput(enter_key)).await.unwrap();

    assert_eq!(app.mode, Mode::List);
    assert_eq!(app.settings.theme_name, Some(second_theme.clone()));

    // 5. Verify if it's saved in storage
    let saved_settings = storage.get_settings().await.unwrap();
    assert_eq!(saved_settings.theme_name, Some(second_theme));
}

fn app_to_render_state<'a>(app: &'a mut App<'static>) -> ui::RenderState<'a, 'static> {
    ui::RenderState {
        nav: &mut app.nav,
        editor: &mut app.editor,
        mode: app.mode,
        settings: &app.settings,
        current_branch: app.current_branch.as_deref(),
        show_help: app.show_help,
        help_scroll: app.help_scroll,
        ai_pending: None,
    }
}
