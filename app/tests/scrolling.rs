mod common;
use common::setup_app;
use contracts::Storage;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[tokio::test]
async fn test_list_scrolling() {
    let (mut app, storage, _, _) = setup_app();

    // Create 50 prompts to ensure they don't all fit in the view
    let mut prompts = Vec::new();
    for i in 1..=50 {
        prompts.push(contracts::Prompt::new(
            format!("Prompt {i:02}"),
            contracts::PromptType::Prompt,
            None,
            None,
        ));
    }
    storage.save_project_prompts(common::TEST_PATH, prompts).await.unwrap();

    app.load_prompts().await.unwrap();
    assert_eq!(app.prompts.len(), 50);

    // Set a small terminal height
    let backend = TestBackend::new(80, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    // Move to the last prompt
    app.move_to_bottom();
    assert_eq!(app.selected_index, 49);

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

    let buffer = terminal.backend().buffer();
    
    // Check if "Prompt 50" is visible.
    // In a 10-line terminal:
    // 3 lines for header
    // 2 lines for footer
    // 1 line for statusline
    // That leaves 4 lines for content if preview is hidden or if terminal is too small for preview.
    // In ui/src/lib.rs: available_for_main = 10 - 6 = 4.
    // available_for_main < 10, so preview is NOT shown.
    // content_chunk height should be 4.
    
    let mut found_last_prompt = false;
    for y in 0..10 {
        let mut line = String::new();
        for x in 0..80 {
            line.push_str(buffer[(x, y)].symbol());
        }
        if line.contains("Prompt 50") {
            found_last_prompt = true;
            break;
        }
    }
    
    assert!(found_last_prompt, "Last prompt 'Prompt 50' should be visible when selected");
}
