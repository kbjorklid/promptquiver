mod common;
use common::setup_app;
use ui::{AppMessage, Mode};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

#[tokio::test]
async fn test_new_project_footer_update() {
    let (mut app, _storage, _clipboard, _git) = setup_app();
    let backend = TestBackend::new(100, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    // 1. Enter Project Picker
    app.handle_message(AppMessage::SelectProject).await.unwrap();
    assert_eq!(app.mode, Mode::ProjectPicker);

    // 2. Enter Add Project
    app.handle_message(AppMessage::EnterAddProject).await.unwrap();
    assert_eq!(app.mode, Mode::AddProject);

    // 3. Add Project "New Project"
    app.handle_message(AppMessage::AddProject("New Project".into())).await.unwrap();
    assert_eq!(app.mode, Mode::List);

    // 4. Render and check status line
    terminal.draw(|f| {
        ui::render(f, ui::RenderState {
            nav: &mut app.nav,
            editor: &mut app.editor,
            mode: app.mode,
            settings: &app.settings,
            current_branch: app.current_branch.as_deref(),
            show_help: app.show_help,
            help_scroll: app.help_scroll,
        }, &mut app.toaster);
    }).unwrap();

    let buffer = terminal.backend().buffer();
    let mut status_line = String::new();
    // Status line is at the bottom (y=19)
    for x in 0..100 {
        status_line.push_str(buffer[(x, 19)].symbol());
    }

    assert!(status_line.contains("New Project"), "Status line should contain 'New Project'");
}

#[tokio::test]
async fn test_delete_project_updates_list() {
    let (mut app, _storage, _clipboard, _git) = setup_app();
    
    // 1. Add a project
    app.handle_message(AppMessage::AddProject("To Delete".into())).await.unwrap();
    let project_id = app.nav.projects_manager.active_project_id.unwrap();

    // 2. Open picker
    app.handle_message(AppMessage::SelectProject).await.unwrap();
    assert!(app.nav.projects_manager.projects.iter().any(|p| p.id == project_id));

    // 3. Delete it
    app.handle_message(AppMessage::DeleteProject(project_id)).await.unwrap();

    // 4. Verify it's gone from the internal list
    assert!(!app.nav.projects_manager.projects.iter().any(|p| p.id == project_id));
    }

