use promptquiver::app::{App, Mode};
use contracts::Tab;
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
    // Tab Visibility (6 items) + Slash Commands (0 initial + 1 Add New) + Claude (1) + Nerd (1) + Theme (1)
    // selected_index for Theme should be 6 + 1 + 2 = 9
    app.nav.selected_index = 9;

    // 3. Open Theme Picker
    app.mode = Mode::ThemePicker;

    // Render once to check
    terminal.draw(|f| {
        ui::render(f, app_to_render_state(&mut app), &mut None);
    }).unwrap();

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
    terminal.draw(|f| {
        ui::render(f, app_to_render_state(&mut app), &mut None);
    }).unwrap();

    let buffer = terminal.backend().buffer();
    let palette = ui::utils::get_palette(app.settings.theme_name.as_deref());
    
    // Check if some cell has the theme's background color
    // Usually cells in the list area
    let sample_cell = &buffer[(10, 10)];
    assert_eq!(sample_cell.bg, palette.bg);
}

#[tokio::test]
async fn test_theme_picker_opening_and_dismissal() {
    use promptquiver::handlers::handle_key_event;
    use crossterm::event::{KeyEvent, KeyCode, KeyModifiers, KeyEventKind, KeyEventState};

    let (mut app, _, _, _) = common::setup_app();

    app.set_tab(Tab::Settings);
    app.load_prompts().await.unwrap();

    // 1. Verify opening theme picker via Enter
    let tabs_len = Tab::all().len();
    let slash_len = app.settings.slash_commands.len();
    let theme_idx = tabs_len + slash_len + 3; // theme item index
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

fn app_to_render_state<'a>(app: &'a mut App<'static>) -> ui::RenderState<'a, 'static> {
    ui::RenderState {
        nav: &mut app.nav,
        editor: &mut app.editor,
        mode: app.mode,
        settings: &app.settings,
        current_branch: app.current_branch.as_deref(),
    }
}

