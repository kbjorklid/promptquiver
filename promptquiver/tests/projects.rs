mod common;
use common::setup_app;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use ui::{AppMessage, Mode};

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
                    ai_pending: None,
                },
                &mut app.toaster,
            );
        })
        .unwrap();

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

#[tokio::test]
async fn test_rename_project_flow() {
    let (mut app, _storage, _clipboard, _git) = setup_app();

    // 1. Add a project
    app.handle_message(AppMessage::AddProject("Project Alpha".into())).await.unwrap();
    let project_id = app.nav.projects_manager.active_project_id.unwrap();

    // 2. Open picker
    app.handle_message(AppMessage::SelectProject).await.unwrap();

    // 3. Select the project in the list
    // Index 0 is Default, Index 1 is Project Alpha
    app.nav.projects_manager.project_list_state.select(Some(1));

    // 4. Press 'r' to rename
    let r_key = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE);
    app.handle_message(AppMessage::ProjectPickerInput(r_key)).await.unwrap();

    assert_eq!(app.mode, Mode::RenameProject);

    // 5. Clear and type new name "Project Beta"
    for _ in 0.."Project Alpha".len() {
        let bs_key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        app.handle_message(AppMessage::RenameProjectInput(bs_key)).await.unwrap();
    }
    for c in "Project Beta".chars() {
        let key = KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
        app.handle_message(AppMessage::RenameProjectInput(key)).await.unwrap();
    }

    // 6. Press Enter
    let enter_key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    app.handle_message(AppMessage::RenameProjectInput(enter_key)).await.unwrap();

    // 7. Verify rename
    assert_eq!(app.mode, Mode::List);
    let project = app.nav.projects_manager.projects.iter().find(|p| p.id == project_id).unwrap();
    assert_eq!(project.title, "Project Beta");
}

#[tokio::test]
async fn test_delete_project_shortcut() {
    let (mut app, _storage, _clipboard, _git) = setup_app();

    // 1. Add a project
    app.handle_message(AppMessage::AddProject("To Delete".into())).await.unwrap();
    let project_id = app.nav.projects_manager.active_project_id.unwrap();

    // 2. Open picker
    app.handle_message(AppMessage::SelectProject).await.unwrap();

    // 3. Select the project
    app.nav.projects_manager.project_list_state.select(Some(1));

    // 4. Press 'd' to delete
    let d_key = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
    app.handle_message(AppMessage::ProjectPickerInput(d_key)).await.unwrap();

    // 5. Verify it's gone
    assert!(!app.nav.projects_manager.projects.iter().any(|p| p.id == project_id));
}

#[tokio::test]
async fn test_project_picker_hints_rendered() {
    let (mut app, _storage, _clipboard, _git) = setup_app();
    let backend = TestBackend::new(100, 20);
    let mut terminal = Terminal::new(backend).unwrap();

    // 1. Enter Project Picker
    app.handle_message(AppMessage::SelectProject).await.unwrap();
    assert_eq!(app.mode, Mode::ProjectPicker);

    // 2. Render
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
                    ai_pending: None,
                },
                &mut app.toaster,
            );
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let mut rendered_text = String::new();
    for y in 0..20 {
        for x in 0..100 {
            rendered_text.push_str(buffer[(x, y)].symbol());
        }
        rendered_text.push('\n');
    }

    assert!(rendered_text.contains("(r) Rename"), "Should contain Rename hint");
    assert!(rendered_text.contains("(d/del) Delete"), "Should contain Delete hint");
}
