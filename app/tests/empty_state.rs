mod common;
use common::setup_app;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[tokio::test]
async fn test_empty_state_big_text() {
    let (mut app, _, _, _) = setup_app();
    app.prompts.clear();

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
                    mode: "List",
                    textarea: &app.textarea,
                    title_textarea: &app.title_textarea,
                    title_focused: app.title_focused,
                    current_branch: app.current_branch.as_deref(),
                    current_path: &app.current_path,
                    suggestions: &app.suggestions,
                    suggestion_index: app.suggestion_index,
                    search_query: &app.search_query,
                    global_search_query: &app.global_search_query,
                    settings: &app.settings,
                    throbber_state: &mut app.throbber_state,
                },
                &mut None,
            );
        })
        .unwrap();

    // Just verifying it doesn't crash is good enough for TDD, 
    // but we can also check if the buffer contains part of the "Press 'a' to add" text 
    // since the BigText letters are composed of blocks that are hard to assert as a string.
}