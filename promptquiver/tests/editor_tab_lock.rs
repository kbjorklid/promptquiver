mod common;
use common::setup_app;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

/// Helper that renders the app and returns the fg color of the cell at (x, y).
fn cell_fg(
    terminal: &mut Terminal<TestBackend>,
    app: &mut promptquiver::app::App<'_>,
    x: u16,
    y: u16,
) -> ratatui::style::Color {
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
    terminal.backend().buffer()[(x, y)].fg
}

/// When in editor mode the inactive tabs should use the muted palette color so that
/// the user can tell tab-switching is unavailable.
#[tokio::test]
async fn test_tabs_muted_in_editor_mode() {
    let (mut app, _, _, _) = setup_app();
    // Use a wide terminal so that multiple tabs are visible
    let backend = TestBackend::new(120, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    let palette = ui::utils::get_palette(app.settings.theme_name.as_deref());

    // In List mode the second tab area should use the normal foreground color
    app.mode = ui::Mode::List;
    // x=50 is well inside the tab bar area (past the 20-char branding), on the inactive tabs
    let list_fg = cell_fg(&mut terminal, &mut app, 50, 0);
    assert_eq!(list_fg, palette.fg, "In List mode, inactive tab text should use palette.fg");

    // In Editor mode the same cell should be muted
    app.mode = ui::Mode::Editor;
    let editor_fg = cell_fg(&mut terminal, &mut app, 50, 0);
    assert_eq!(
        editor_fg, palette.muted,
        "In Editor mode, tab text should use palette.muted to indicate tabs are locked"
    );
}
