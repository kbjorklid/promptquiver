mod common;
use common::setup_app;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

#[tokio::test]
async fn test_editor_scrollbar_render() {
    let (mut app, _, _, _) = setup_app();

    let mut lines = Vec::new();
    for i in 0..50 {
        lines.push(format!("Line {i}"));
    }
    app.enter_editor(lines.join("\n"), None);

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

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
        println!("{line_str}");
    }

    // We expect a scrollbar to be rendered.
    assert!(found_scrollbar, "Scrollbar should be visible in the editor");
}
