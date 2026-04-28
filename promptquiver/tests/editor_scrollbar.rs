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
                    active_tab: app.active_tab,
                    prompts: &app.prompts,
                    selected_index: app.selected_index,
                    list_state: &mut app.list_state,
                    settings_slash_list_state: &mut app.settings_slash_list_state,
                    theme_list_state: &mut app.theme_list_state,
                    mode: "Editor",
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
                    throbber_state: &mut app.throbber_state,
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
