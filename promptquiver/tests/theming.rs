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
    app.selected_index = 9;

    // 3. Open Theme Picker
    app.mode = Mode::ThemePicker;

    // Render once to check
    terminal.draw(|f| {
        ui::render(f, app_to_render_state(&mut app), &mut None);
    }).unwrap();

    // 4. Select a theme (e.g., the second one)
    app.theme_list_state.select(Some(1));
    
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

fn app_to_render_state<'a>(app: &'a mut App<'static>) -> ui::RenderState<'a, 'static> {
    let mode_str = match app.mode {
        Mode::List => "List",
        Mode::Editor => "Editor",
        Mode::Move => "Move",
        Mode::Search => "Search",
        Mode::GlobalSearch => "Global Search",
        Mode::ConfirmDiscard => "Confirm Discard",
        Mode::ThemePicker => "Theme Picker",
    };
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
    }
}
