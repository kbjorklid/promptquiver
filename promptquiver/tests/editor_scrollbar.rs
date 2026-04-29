mod common;
use common::setup_app;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[tokio::test]
async fn test_editor_scrollbar_render() {
    let (mut app, _, _, _) = setup_app();

    let mut lines = Vec::new();
    for i in 0..50 {
        lines.push(format!("Line {}", i));
    }
    app.enter_editor(lines.join("\n"), None);

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            ui::render(
                f,
                ui::RenderState {
                    active_tab: app.nav.active_tab,
                    prompts: &app.nav.prompts,
                    selected_index: app.nav.selected_index,
                    list_state: &mut app.nav.list_state,
                    settings_slash_list_state: &mut app.nav.settings_slash_list_state,
                    theme_list_state: &mut app.nav.theme_list_state,
                    mode: "Editor",
                    textarea: &mut app.editor.textarea,
                    title_textarea: &mut app.editor.title_textarea,
                    title_focused: app.editor.title_focused,
                    current_branch: app.current_branch.as_deref(),
                    current_path: &app.nav.current_path,
                    suggestions: &app.editor.autocomplete.suggestions,
                    suggestion_index: app.editor.autocomplete.index,
                    autocomplete_open: app.editor.autocomplete.open,
                    autocomplete_list_state: &mut app.editor.autocomplete.list_state,
                    search_query: &app.nav.search_query,
                    global_search_query: &app.nav.global_search_query,
                    settings: &app.settings,
                },
                &mut None,
            );
        })
        .unwrap();
    
    let buffer = terminal.backend().buffer();
    
    let mut found_scrollbar = false;
    for y in 0..24 {
        let mut line_str = String::new();
        for x in 0..80 {
            let symbol = buffer[(x, y)].symbol();
            line_str.push_str(symbol);
            if symbol == "↑" || symbol == "↓" || symbol == "█" || symbol == "║" {
                found_scrollbar = true;
            }
        }
        println!("{}", line_str);
    }
    
    // We expect a scrollbar to be rendered.
    assert!(found_scrollbar, "Scrollbar should be visible in the editor");
}

