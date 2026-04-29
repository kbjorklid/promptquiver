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
    let storage: Arc<dyn Storage> = Arc::new(FileSystemStorage::new(None));
    let clipboard: Arc<dyn Clipboard> = Arc::new(RealClipboard::new());
    let git: Arc<dyn Git> = Arc::new(RealGit::new());
    let service: Arc<dyn contracts::AppService> = Arc::new(infra::RealAppService::new(storage.clone(), clipboard.clone()));

    // App State
    let mut app = App::new(storage.clone(), clipboard, git.clone(), service);
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

    tokio::spawn(async move {
        while let Some((path, query)) = file_search_rx.recv().await {
            let path_buf = std::path::PathBuf::from(path);
            let mut results = Vec::new();
            promptquiver::app::walk_files(&path_buf, &path_buf, &query, &mut results);
            let _ = file_result_tx.send((query, results)).await;
        }
    });

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
            let mode_str = match app.mode {
                promptquiver::app::Mode::List => "List",
                promptquiver::app::Mode::Editor => "Editor",
                promptquiver::app::Mode::Move => "Move",
                promptquiver::app::Mode::Search => "Search",
                promptquiver::app::Mode::GlobalSearch => "Global Search",
                promptquiver::app::Mode::ConfirmDiscard => "Confirm Discard",
                promptquiver::app::Mode::ThemePicker => "Theme Picker",
            };
            ui::render(
                f,
                ui::RenderState {
                    active_tab: app.nav.active_tab,
                    prompts: &app.nav.prompts,
                    selected_index: app.nav.selected_index,
                    list_state: &mut app.nav.list_state,
                    settings_slash_list_state: &mut app.nav.settings_slash_list_state,
                    theme_list_state: &mut app.nav.theme_list_state,
                    mode: mode_str,
                    textarea: &mut app.editor.textarea,
                    title_textarea: &mut app.editor.title_textarea,
                    title_focused: app.editor.title_focused,
                    current_branch: app.current_branch.as_deref(),
                    current_path: &app.nav.current_path,
                    suggestions: &app.editor.autocomplete.suggestions,
                    suggestion_index: app.editor.autocomplete.index,
                    autocomplete_open: app.editor.autocomplete.open,
                    autocomplete_list_state: &mut app.editor.autocomplete.list_state,
                    search_query: &app.nav.search_query,
                    global_search_query: &app.nav.global_search_query,
                    settings: &app.settings,
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
