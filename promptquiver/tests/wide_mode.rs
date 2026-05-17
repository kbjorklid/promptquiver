mod common;
use contracts::{PreviewMode, Tab};
use promptquiver::app::App;
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use ui::utils::{get_palette, get_zebra_color};

fn app_to_render_state<'a>(app: &'a mut App<'static>) -> ui::RenderState<'a, 'static> {
    ui::RenderState {
        nav: &mut app.nav,
        editor: &mut app.editor,
        mode: app.mode,
        settings: &app.settings,
        current_branch: app.current_branch.as_deref(),
        show_help: app.show_help,
        help_scroll: app.help_scroll,
    }
}

#[tokio::test]
async fn test_wide_mode_active_item_line1_bg_matches_row() {
    let (mut app, _, _, _) = common::setup_app();

    app.enter_editor("A".to_string(), None);
    app.save_editor().await.unwrap();
    app.enter_editor("B".to_string(), None);
    app.save_editor().await.unwrap();

    assert_eq!(app.nav.prompts.len(), 2);

    app.settings.show_wide_view = true;
    app.settings.preview_mode = PreviewMode::Hidden;

    // Select index 0 (even) so the bar row sits on an even-indexed item.
    // Before the fix, line1's trailing area inherits palette.bg from the List
    // widget instead of zebra_bg from the item's row style.
    app.nav.selected_index = 0;

    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            ui::render(f, app_to_render_state(&mut app), &mut None);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let palette = get_palette(app.settings.theme_name.as_deref());
    let zebra_bg = get_zebra_color(palette.bg);

    // Layout (100 cols x 30 rows, no preview):
    //   y=0: header
    //   y=1: list top border
    //   y=2: item 0 line1  (even index, SELECTED => bar row)
    //   y=3: item 0 line2
    let trailing_x = 80u16;
    let active_line1_y = 2u16;

    let trailing_cell = &buffer[(trailing_x, active_line1_y)];
    assert_eq!(
        trailing_cell.bg, zebra_bg,
        "trailing area of line1 for an even-indexed active (bar) item must carry the \
         zebra background, not the default list background"
    );
}

#[tokio::test]
async fn test_wide_mode_zebra_striping_fills_full_row() {
    let (mut app, _, _, _) = common::setup_app();

    // Add two prompts (short text so trailing area is well past content)
    app.enter_editor("A".to_string(), None);
    app.save_editor().await.unwrap();
    app.enter_editor("B".to_string(), None);
    app.save_editor().await.unwrap();

    // Newest prompt is at index 0, previous at index 1
    assert_eq!(app.nav.prompts.len(), 2);

    // Enable wide view and hide preview for a predictable layout
    app.settings.show_wide_view = true;
    app.settings.preview_mode = PreviewMode::Hidden;

    // Select index 1 so index 0 renders with zebra background (not selection highlight)
    app.nav.selected_index = 1;

    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            ui::render(f, app_to_render_state(&mut app), &mut None);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let palette = get_palette(app.settings.theme_name.as_deref());
    let zebra_bg = get_zebra_color(palette.bg);

    // Layout (100 cols x 30 rows, no search, no preview):
    //   y=0:  header (1 row)
    //   y=1:  list top border
    //   y=2:  item 0 line1  (even index => zebra_bg)
    //   y=3:  item 0 line2  (even index => zebra_bg)
    //   y=4:  item 1 line1  (odd index  => palette.bg, also selected)
    //   y=5:  item 1 line2
    //
    // x=80 is well past the "  B" / "  A" text, so we are in the unfilled trailing area.

    let trailing_x = 80u16;
    let item0_line1_y = 2u16;

    let trailing_cell = &buffer[(trailing_x, item0_line1_y)];
    assert_eq!(
        trailing_cell.bg, zebra_bg,
        "trailing area of line1 for an even-indexed item must carry the zebra background \
         in wide mode (not the default list background)"
    );
}

#[tokio::test]
async fn test_wide_mode_applies_to_archive_tab() {
    let (mut app, _, _, _) = common::setup_app();

    app.enter_editor("A".to_string(), None);
    app.save_editor().await.unwrap();
    assert_eq!(app.nav.prompts.len(), 1);

    app.archive_selected().await.unwrap();
    assert_eq!(app.nav.prompts.len(), 0);

    app.set_tab(Tab::Archive);
    app.load_prompts().await.unwrap();
    assert_eq!(app.nav.prompts.len(), 1);

    app.settings.show_wide_view = true;
    app.settings.preview_mode = PreviewMode::Hidden;
    app.nav.selected_index = 0;

    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            ui::render(f, app_to_render_state(&mut app), &mut None);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let palette = get_palette(app.settings.theme_name.as_deref());
    let zebra_bg = get_zebra_color(palette.bg);

    // In wide mode, item 0 occupies two rows:
    //   y=2: line1 (title row, selected => bar row)
    //   y=3: line2 (metadata row)
    // The metadata row must have the zebra background, not the default list background.
    // Without the fix (show_wide only for Tab::Prompts), line2 would not exist and
    // y=3 would be the next item's title row with palette.bg.
    let trailing_x = 80u16;
    let item0_line2_y = 3u16;

    let line2_cell = &buffer[(trailing_x, item0_line2_y)];
    assert_eq!(
        line2_cell.bg, zebra_bg,
        "Archive tab wide view: line2 (metadata row) of item 0 must carry the zebra \
         background — wide mode must apply to Archive tab, not just Prompts tab"
    );
}
