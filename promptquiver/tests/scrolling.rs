mod common;
use common::setup_app;
use contracts::Storage;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

#[tokio::test]
async fn test_list_scrolling() {
    let (mut app, storage, _, _) = setup_app();

    // Create 50 prompts to ensure they don't all fit in the view
    // Create in reverse order so Prompt 50 is oldest and Prompt 01 is newest
    for i in (1..=50).rev() {
        let p = contracts::Prompt::new(
            format!("Prompt {i:02}"),
            contracts::PromptType::Prompt,
            Some(common::TEST_PATH.to_string()),
            None,
            None,
            None,
        );
        storage.save_prompt(p).await.unwrap();
    }

    app.load_prompts().await.unwrap();
    assert_eq!(app.nav.prompts.len(), 50);

    // Set a small terminal height
    let backend = TestBackend::new(80, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    // Move to the last prompt
    app.move_to_bottom();
    assert_eq!(app.nav.selected_index, 49);

    terminal
        .draw(|f| {
            ui::render(
                f,
                ui::RenderState {
                    nav: &mut app.nav,
                    editor: &mut app.editor,
                    mode: app.mode,
                    settings: &app.settings,
                    current_branch: app.current_branch.as_deref(),
                    show_help: app.show_help,
                    help_scroll: app.help_scroll,
                    ai_pending_titles: None,
                    ai_download_progress: None,
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

#[tokio::test]
async fn test_settings_scrolling() {
    let (mut app, _storage, _clipboard, _git) = setup_app();
    app.nav.active_tab = contracts::Tab::Settings;
    app.nav.selected_index = 0;

    let backend = TestBackend::new(80, 10); // Small height to force scrolling
    let mut terminal = ratatui::Terminal::new(backend).unwrap();

    // Initial render
    terminal
        .draw(|f| {
            let state = ui::RenderState {
                nav: &mut app.nav,
                editor: &mut app.editor,
                mode: app.mode,
                settings: &app.settings,
                current_branch: app.current_branch.as_deref(),
                show_help: app.show_help,
                help_scroll: app.help_scroll,
                    ai_pending_titles: None,
                    ai_download_progress: None,
            };
            ui::render(f, state, &mut app.toaster);
        })
        .unwrap();

    assert_eq!(app.nav.settings_scroll_offset, 0);

    // Move to the bottom of settings
    // total_settings = 5 (tabs) + 0 (slash) + 1 (Add New) + 3 (advanced) = 9
    // Indices: 0-4 (tabs), 5 (Add New), 6-8 (advanced)
    app.nav.selected_index = 8; // Last advanced setting

    // Render again to update scroll offset
    terminal
        .draw(|f| {
            let state = ui::RenderState {
                nav: &mut app.nav,
                editor: &mut app.editor,
                mode: app.mode,
                settings: &app.settings,
                current_branch: app.current_branch.as_deref(),
                show_help: app.show_help,
                help_scroll: app.help_scroll,
                    ai_pending_titles: None,
                    ai_download_progress: None,
            };
            ui::render(f, state, &mut app.toaster);
        })
        .unwrap();

    // With height 10, and total settings 9 + some borders, it should have scrolled
    assert!(
        app.nav.settings_scroll_offset > 0,
        "Scroll offset should be greater than 0, got {}",
        app.nav.settings_scroll_offset
    );

    // Move back to top
    app.nav.selected_index = 0;
    terminal
        .draw(|f| {
            let state = ui::RenderState {
                nav: &mut app.nav,
                editor: &mut app.editor,
                mode: app.mode,
                settings: &app.settings,
                current_branch: app.current_branch.as_deref(),
                show_help: app.show_help,
                help_scroll: app.help_scroll,
                    ai_pending_titles: None,
                    ai_download_progress: None,
            };
            ui::render(f, state, &mut app.toaster);
        })
        .unwrap();

    assert_eq!(app.nav.settings_scroll_offset, 0);
}
