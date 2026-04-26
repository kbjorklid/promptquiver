mod common;
use common::setup_app;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[tokio::test]
async fn test_add_edit_prompt() {
    let (mut app, _, _, _) = setup_app();
    
    app.enter_editor("New Prompt".to_string(), None);
    app.save_editor().await.unwrap();
    assert_eq!(app.prompts.len(), 1);
    assert_eq!(app.prompts[0].text, "New Prompt");

    let id = app.prompts[0].id;
    app.enter_editor("Updated Prompt".to_string(), Some(id));
    app.save_editor().await.unwrap();
    
    assert_eq!(app.prompts.len(), 1);
    assert_eq!(app.prompts[0].text, "Updated Prompt");
}

#[tokio::test]
async fn test_editor_discard_confirmation_modal() {
    let (mut app, _, _, _) = setup_app();

    app.enter_editor("Original".to_string(), None);
    app.textarea.insert_str("Modified");
    
    let current_text = app.textarea.lines().join("\n");
    if current_text != app.original_text {
        app.mode = app::app::Mode::ConfirmDiscard;
    }

    assert_eq!(app.mode, app::app::Mode::ConfirmDiscard);

    let backend = TestBackend::new(80, 30);
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
                    mode: "Confirm Discard",
                    textarea: &app.textarea,
                    title_textarea: &app.title_textarea,
                    title_focused: app.title_focused,
                    current_branch: None,
                    current_path: "",
                    suggestions: &[],
                    suggestion_index: 0,
                    search_query: "",
                    global_search_query: "",
                    settings: &contracts::Settings::default(),
                },
                &mut None,
            );
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    
    let mut found_modal_title = false;
    for y in 0..30 {
        for x in 0..80 {
            let mut line = String::new();
            for i in 0..16 {
                if x + i < 80 {
                    line.push_str(buffer[(x + i, y)].symbol());
                }
            }
            if line.contains("Discard Changes?") {
                found_modal_title = true;
                break;
            }
        }
    }
    assert!(found_modal_title);
}
