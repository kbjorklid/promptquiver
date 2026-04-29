use anyhow::Result;
use promptquiver::app::App;
use promptquiver::tui::{self, Tui};
use contracts::{Clipboard, Git, Storage};
use infra::{FileSystemStorage, RealClipboard, RealGit};
use ratatui::Terminal;
use ratatui_toaster::{ToastEngineBuilder, ToastType};
use std::{io, sync::Arc, time::Duration};
use crossterm::event::{Event, KeyEventKind};

macro_rules! handle_error {
    ($app:expr, $res:expr) => {
        if let Err(e) = $res {
            $app.notify(format!("Error: {}", e), ToastType::Error);
        }
    };
}

#[tokio::main]
async fn main() -> Result<()> {
    // Infrastructure
    let fs_storage = Arc::new(FileSystemStorage::new(None));
    let storage: Arc<dyn Storage> = fs_storage.clone();
    let clipboard: Arc<dyn Clipboard> = Arc::new(RealClipboard::new());
    let git: Arc<dyn Git> = Arc::new(RealGit::new());
    let service: Arc<dyn contracts::AppService> = Arc::new(infra::RealAppService::new(storage.clone(), clipboard.clone()));

    // App State
    let mut app = App::new(storage.clone(), clipboard, git.clone(), service.clone());
    handle_error!(app, app.load_prompts().await);

    // Background Git Poller
    let (branch_tx, mut branch_rx) = tokio::sync::mpsc::channel(1);
    let git_clone = git.clone();
    tokio::spawn(async move {
        let path = std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        loop {
            if let Ok(branch) = git_clone.get_current_branch(&path).await {
                let _ = branch_tx.send(branch).await;
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    // Background File Searcher
    let (file_search_tx, mut file_search_rx) = tokio::sync::mpsc::channel::<(String, String)>(10);
    let (file_result_tx, mut file_result_rx) = tokio::sync::mpsc::channel::<(String, Vec<contracts::Prompt>)>(10);
    app.file_search_tx = Some(file_search_tx);

    let service_clone = service.clone();
    tokio::spawn(async move {
        while let Some((path, query)) = file_search_rx.recv().await {
            if let Ok(results) = service_clone.search_files(&path, &query).await {
                let _ = file_result_tx.send((query, results)).await;
            }
        }
    });

    // Background File Watcher
    let (file_event_tx, mut file_event_rx) = tokio::sync::mpsc::channel(10);
    let watch_path = fs_storage.get_base_dir();
    
    use notify::{Watcher, RecursiveMode, Config};
    let mut watcher = notify::RecommendedWatcher::new(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            if event.kind.is_modify() || event.kind.is_create() {
                // Check if it's a toml file
                if event.paths.iter().any(|p| p.extension().map_or(false, |ext| ext == "toml")) {
                    let _ = file_event_tx.blocking_send(());
                }
            }
        }
    }, Config::default())?;

    watcher.watch(&watch_path, RecursiveMode::Recursive)?;

    // Terminal
    let backend = ratatui::backend::CrosstermBackend::new(io::stdout());
    let terminal = Terminal::new(backend)?;
    let mut tui = Tui::new(terminal);

    tui.enter()?;

    // Initialize toaster with terminal area
    app.toaster = Some(
        ToastEngineBuilder::new(tui.terminal.size()?.into())
            .default_duration(Duration::from_secs(3))
            .build()
    );

    while !app.should_quit {
        // Handle background updates
        if let Ok(branch) = branch_rx.try_recv() {
            app.current_branch = branch;
        }

        if let Ok(_) = file_event_rx.try_recv() {
            // Reload prompts if we are not in the middle of editing
            if app.mode == ui::Mode::List {
                handle_error!(app, app.load_prompts().await);
            }
        }

        while let Ok((query, results)) = file_result_rx.try_recv() {
            // Only update if the query matches current cursor state
            if let Some((trigger, current_query)) = app.editor.get_current_autocomplete_query() {
                if trigger == "@" && current_query == query {
                    if !results.is_empty() {
                        app.editor.autocomplete.suggestions = results;
                        app.editor.autocomplete.open = true;
                        app.editor.autocomplete.index = 0;
                    } else {
                        app.editor.autocomplete.open = false;
                        app.editor.autocomplete.suggestions.clear();
                    }
                }
            }
        }

        tui.terminal.draw(|f| {
            ui::render(
                f,
                ui::RenderState {
                    nav: &mut app.nav,
                    editor: &mut app.editor,
                    mode: app.mode,
                    settings: &app.settings,
                    current_branch: app.current_branch.as_deref(),
                },
                &mut app.toaster,
            );
        })?;

        if let Some(event) = tui::next_event(Duration::from_millis(16))? {
            if let Event::Key(key) = event {
                if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat {
                    promptquiver::handlers::handle_key_event(&mut app, key).await;
                }
            }
        }

        app.tick();
    }

    tui.exit()?;

    Ok(())
}
