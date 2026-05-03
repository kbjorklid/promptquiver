use anyhow::Result;
use promptquiver::app::App;
use promptquiver::tui::{self, Tui};
use contracts::{Clipboard, Git, Storage};
use infra::{RealClipboard, RealGit, SqliteStorage};
use ratatui::Terminal;
use ratatui_toaster::{ToastEngineBuilder, ToastType};
use std::{io, sync::Arc, time::Duration};

macro_rules! handle_error {
    ($app:expr, $res:expr) => {
        if let Err(e) = $res {
            $app.notify(format!("Error: {e}"), ToastType::Error);
        }
    };
}

type AppInfra = (
    Arc<dyn Storage>,
    Arc<dyn Clipboard>,
    Arc<dyn Git>,
    Arc<dyn contracts::AppService>,
);

#[tokio::main]
async fn main() -> Result<()> {
    let (storage, clipboard, git, service) = setup_infra();
    let mut app = App::new(storage.clone(), clipboard, git.clone(), service.clone());
    handle_error!(app, app.init_project().await);

    let mut branch_rx = setup_git_poller(git.clone());
    let mut file_result_rx = setup_file_searcher(&mut app, service.clone());
    let mut db_sync_rx = setup_db_poller(storage.clone());

    let mut tui = Tui::new(Terminal::new(ratatui::backend::CrosstermBackend::new(io::stdout()))?);
    tui.enter()?;

    app.toaster = Some(
        ToastEngineBuilder::new(tui.terminal.size()?.into())
            .default_duration(Duration::from_secs(3))
            .build()
    );

    run_app_loop(&mut app, &mut tui, &mut branch_rx, &mut file_result_rx, &mut db_sync_rx).await?;
    tui.exit()?;
    Ok(())
}

fn setup_infra() -> AppInfra {
    let db_dir = directories::ProjectDirs::from("", "", "promptquiver")
        .map_or_else(|| std::path::PathBuf::from("."), |d| d.data_dir().to_path_buf());
    if !db_dir.exists() {
        let _ = std::fs::create_dir_all(&db_dir);
    }
    let db_path = db_dir.join("promptquiver.db");

    let storage: Arc<dyn Storage> = Arc::new(SqliteStorage::new(db_path));
    let clipboard: Arc<dyn Clipboard> = Arc::new(RealClipboard::new());
    let git: Arc<dyn Git> = Arc::new(RealGit::new());
    let service: Arc<dyn contracts::AppService> = Arc::new(infra::RealAppService::new(storage.clone(), clipboard.clone()));
    (storage, clipboard, git, service)
}

fn setup_git_poller(git: Arc<dyn Git>) -> tokio::sync::mpsc::Receiver<Option<String>> {
    let (branch_tx, branch_rx) = tokio::sync::mpsc::channel(1);
    tokio::spawn(async move {
        let path = std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        loop {
            if let Ok(branch) = git.get_current_branch(&path).await {
                let _ = branch_tx.send(branch).await;
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
    branch_rx
}

fn setup_file_searcher(app: &mut App<'_>, service: Arc<dyn contracts::AppService>) -> tokio::sync::mpsc::Receiver<(String, Vec<contracts::Prompt>)> {
    let (file_search_tx, mut file_search_rx) = tokio::sync::mpsc::channel::<(String, String)>(10);
    let (file_result_tx, file_result_rx) = tokio::sync::mpsc::channel::<(String, Vec<contracts::Prompt>)>(10);
    app.file_search_tx = Some(file_search_tx);

    tokio::spawn(async move {
        while let Some((path, query)) = file_search_rx.recv().await {
            if let Ok(results) = service.search_files(&path, &query).await {
                let _ = file_result_tx.send((query, results)).await;
            }
        }
    });
    file_result_rx
}

fn setup_db_poller(storage: Arc<dyn Storage>) -> tokio::sync::mpsc::Receiver<()> {
    let (db_sync_tx, db_sync_rx) = tokio::sync::mpsc::channel(1);
    tokio::spawn(async move {
        let mut last_version = storage.get_data_version().await.unwrap_or(0);
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if let Ok(current_version) = storage.get_data_version().await {
                if current_version != last_version {
                    last_version = current_version;
                    let _ = db_sync_tx.send(()).await;
                }
            }
        }
    });
    db_sync_rx
}

async fn run_app_loop(
    app: &mut App<'_>,
    tui: &mut Tui<ratatui::backend::CrosstermBackend<io::Stdout>>,
    branch_rx: &mut tokio::sync::mpsc::Receiver<Option<String>>,
    file_result_rx: &mut tokio::sync::mpsc::Receiver<(String, Vec<contracts::Prompt>)>,
    db_sync_rx: &mut tokio::sync::mpsc::Receiver<()>,
) -> Result<()> {
    while !app.should_quit {
        if let Ok(branch) = branch_rx.try_recv() {
            app.current_branch = branch;
        }

        if db_sync_rx.try_recv().is_ok() {
            handle_error!(app, app.handle_message(ui::AppMessage::ReloadPrompts).await);
        }

        while let Ok((query, results)) = file_result_rx.try_recv() {
            if let Some((trigger, current_query)) = app.editor.get_current_autocomplete_query() {
                if trigger == "@" && current_query == query {
                    if results.is_empty() {
                        app.editor.autocomplete.open = false;
                        app.editor.autocomplete.suggestions.clear();
                    } else {
                        app.editor.autocomplete.suggestions = results;
                        app.editor.autocomplete.open = true;
                        app.editor.autocomplete.index = 0;
                    }
                }
            }
        }

        tui.terminal.draw(|f| {
            let state = ui::RenderState {
                nav: &mut app.nav,
                editor: &mut app.editor,
                mode: app.mode,
                settings: &app.settings,
                current_branch: app.current_branch.as_deref(),
            };
            ui::render(f, state, &mut app.toaster);
        })?;

        let mut events = Vec::new();
        while let Some(event) = tui::next_event(Duration::from_millis(0))? {
            events.push(event);
        }

        if events.is_empty() {
            if let Some(event) = tui::next_event(Duration::from_millis(16))? {
                events.push(event);
                while let Some(e) = tui::next_event(Duration::from_millis(0))? {
                    events.push(e);
                }
            }
        }

        if !events.is_empty() {
            promptquiver::handlers::handle_events(app, events).await;
        }

        app.tick();
    }
    Ok(())
}

