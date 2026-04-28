mod common;
use common::setup_app;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[tokio::test]
async fn test_dynamic_shortcut_hints() {
    let (mut app, _, _, _) = setup_app();

    let backend = TestBackend::new(120, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    // Helper to render and get footer text
    let mut get_footer = |app: &mut promptquiver::app::App<'_>| {
        terminal.draw(|f| {
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
                    autocomplete_open: app.autocomplete_open,
                    autocomplete_list_state: &mut app.autocomplete_list_state,
                    search_query: &app.search_query,
                    global_search_query: &app.global_search_query,
                    settings: &app.settings,
                },
                &mut None,
            );
        }).unwrap();

        let buffer = terminal.backend().buffer();
        // With height 20:
        // Header: 0,1,2 (3 lines)
        // Content: 3..16 (14 lines, Min 5 satisfied)
        // Footer: 17, 18 (2 lines)
        // Statusline: 19 (1 line)
        
        let mut footer_text = String::new();
        for y in 17..19 {
            for x in 0..120 {
                footer_text.push_str(buffer[(x, y)].symbol());
            }
            footer_text.push('\n');
        }
        footer_text.trim().to_string()
    };

    // 1. Test List Mode
    let footer = get_footer(&mut app);
    assert!(footer.contains("u: Undo"), "List mode should show 'u: Undo'");
    assert!(footer.contains("a/i: Add"), "List mode should show 'a/i: Add'");
    assert!(footer.contains("d/D: Del/Dupe"), "List mode should show 'd/D: Del/Dupe'");
    assert!(footer.contains("Ctrl+f: Global Search"), "List mode should show 'Ctrl+f: Global Search'");

    // 2. Test Move Mode
    app.mode = promptquiver::app::Mode::Move;
    let footer = get_footer(&mut app);
    assert!(footer.contains("j/k: Move"), "Move mode should show 'j/k: Move'");
    assert!(footer.contains("Esc/m/Ent: Back"), "Move mode should show 'Esc/m/Ent: Back'");

    // 3. Test Editor Mode
    app.mode = promptquiver::app::Mode::Editor;
    let footer = get_footer(&mut app);
    assert!(footer.contains("Ctrl+s: Save"), "Editor mode should show 'Ctrl+s: Save'");
    assert!(footer.contains("Ctrl+g: Save & Stage"), "Editor mode should show 'Ctrl+g: Save & Stage'");

    // 4. Test Archive Tab (List Mode)
    app.mode = promptquiver::app::Mode::List;
    app.active_tab = contracts::Tab::Archive;
    let footer = get_footer(&mut app);
    assert!(footer.contains("r: Restore"), "Archive tab should show 'r: Restore'");
}
