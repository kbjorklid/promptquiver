use anyhow::Result;
use clap::Parser;
use contracts::{Clipboard, Git, PromptFilter, Storage};
use infra::{RealClipboard, RealGit, SqliteStorage};
use promptquiver::app::{App, AppMessage};
use promptquiver::tui::{self, Tui};
use ratatui::Terminal;
use ratatui_toaster::{ToastEngineBuilder, ToastType};
use std::{io, path::PathBuf, sync::Arc, time::Duration};
use uuid::Uuid;

macro_rules! handle_error {
    ($app:expr, $res:expr) => {
        if let Err(e) = $res {
            $app.notify(format!("Error: {e}"), ToastType::Error);
        }
    };
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Output the staged prompt to STDOUT and exit
    #[arg(long)]
    output_staged: bool,

    /// Move the staged prompt to archive and exit
    #[arg(long)]
    process_staged: bool,
}

type AppInfra =
    (Arc<dyn Storage>, Arc<dyn Clipboard>, Arc<dyn Git>, Arc<dyn contracts::AppService>);

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let (storage, clipboard, git, service) = setup_infra();

    if args.output_staged || args.process_staged {
        if args.output_staged {
            output_staged(storage.clone()).await?;
        }
        if args.process_staged {
            archive_staged(storage.clone()).await?;
        }
        return Ok(());
    }

    let mut app = App::new(storage.clone(), clipboard, git.clone(), service.clone());
    handle_error!(app, app.init_project().await);

    if let Ok(commands) = service.get_claude_commands(&app.nav.current_project_path()).await {
        app.claude_commands = commands;
    }

    let mut branch_rx = setup_git_poller(git.clone());
    let mut file_result_rx = setup_file_searcher(&mut app, service.clone());
    let mut db_sync_rx = setup_db_poller(storage.clone());

    let data_dir = directories::ProjectDirs::from("", "", "promptquiver")
        .map_or_else(|| PathBuf::from("."), |d| d.data_dir().to_path_buf());
    let (ai_title_tx, mut ai_title_result_rx, ai_progress_tx, mut ai_progress_rx) =
        setup_ai_engine(&app.settings, &data_dir);
    app.ai_title_tx = ai_title_tx;
    app.ai_progress_tx = Some(ai_progress_tx);

    let mut tui = Tui::new(Terminal::new(ratatui::backend::CrosstermBackend::new(io::stdout()))?);
    tui.enter()?;

    app.toaster = Some(
        ToastEngineBuilder::new(tui.terminal.size()?.into())
            .default_duration(Duration::from_secs(3))
            .build(),
    );

    run_app_loop(
        &mut app,
        &mut tui,
        &mut branch_rx,
        &mut file_result_rx,
        &mut db_sync_rx,
        &mut ai_title_result_rx,
        &mut ai_progress_rx,
    )
    .await?;
    tui.exit()?;
    Ok(())
}

async fn output_staged(storage: Arc<dyn Storage>) -> Result<()> {
    let filter = PromptFilter { staged: Some(true), ..Default::default() };
    let prompts = storage.get_prompts(filter).await?;
    if let Some(prompt) = prompts.first() {
        let snippets = storage
            .get_prompts(PromptFilter { tab: Some(contracts::Tab::Snippets), ..Default::default() })
            .await?;
        let processed = contracts::Processor::process(&prompt.text, &snippets);
        print!("{}", processed.trim());
    }
    Ok(())
}

async fn archive_staged(storage: Arc<dyn Storage>) -> Result<()> {
    let filter = PromptFilter { staged: Some(true), ..Default::default() };
    let prompts = storage.get_prompts(filter).await?;
    if let Some(mut prompt) = prompts.into_iter().next() {
        prompt.staged = false;
        prompt.is_archived = true;
        storage.save_prompt(prompt).await?;
    }
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
    let service: Arc<dyn contracts::AppService> =
        Arc::new(infra::RealAppService::new(storage.clone(), clipboard.clone()));
    (storage, clipboard, git, service)
}

fn setup_git_poller(git: Arc<dyn Git>) -> tokio::sync::mpsc::Receiver<Option<String>> {
    let (branch_tx, branch_rx) = tokio::sync::mpsc::channel(1);
    tokio::spawn(async move {
        let path = std::env::current_dir().unwrap_or_default().to_string_lossy().into_owned();
        loop {
            if let Ok(branch) = git.get_current_branch(&path).await {
                let _ = branch_tx.send(branch).await;
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
    branch_rx
}

fn setup_file_searcher(
    app: &mut App<'_>,
    service: Arc<dyn contracts::AppService>,
) -> tokio::sync::mpsc::Receiver<(String, Vec<contracts::Prompt>)> {
    let (file_search_tx, mut file_search_rx) = tokio::sync::mpsc::channel::<(String, String)>(10);
    let (file_result_tx, file_result_rx) =
        tokio::sync::mpsc::channel::<(String, Vec<contracts::Prompt>)>(10);
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
    ai_title_result_rx: &mut Option<tokio::sync::mpsc::Receiver<(Uuid, String)>>,
    ai_progress_rx: &mut tokio::sync::mpsc::Receiver<AppMessage>,
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

        if let Some(rx) = ai_title_result_rx {
            while let Ok((id, title)) = rx.try_recv() {
                handle_error!(
                    app,
                    app.handle_message(ui::AppMessage::TitleGenerated(id, title)).await
                );
            }
        }

        while let Ok(msg) = ai_progress_rx.try_recv() {
            handle_error!(app, app.handle_message(msg).await);
        }

        tui.terminal.draw(|f| {
            let state = ui::RenderState {
                nav: &mut app.nav,
                editor: &mut app.editor,
                mode: app.mode,
                settings: &app.settings,
                current_branch: app.current_branch.as_deref(),
                show_help: app.show_help,
                help_scroll: app.help_scroll,
                ai_pending_titles: Some(&app.ai_pending_titles),
                ai_download_progress: app.ai_download_progress,
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

type AiChannels = (
    Option<tokio::sync::mpsc::Sender<(Uuid, String)>>,
    Option<tokio::sync::mpsc::Receiver<(Uuid, String)>>,
    tokio::sync::mpsc::Sender<AppMessage>,
    tokio::sync::mpsc::Receiver<AppMessage>,
);

fn setup_ai_engine(settings: &contracts::Settings, data_dir: &PathBuf) -> AiChannels {
    let (progress_tx, progress_rx) = tokio::sync::mpsc::channel::<AppMessage>(20);

    #[cfg(feature = "ai")]
    {
        let model_id = infra::ai::model_id(settings.ai_model_tier);
        let downloader = infra::ModelDownloader::new(data_dir.clone());
        if settings.ai_enabled && downloader.is_downloaded(model_id) {
            let (title_req_tx, mut title_req_rx) = tokio::sync::mpsc::channel::<(Uuid, String)>(10);
            let (title_res_tx, title_res_rx) = tokio::sync::mpsc::channel::<(Uuid, String)>(10);
            let data_dir = data_dir.clone();
            let hf_token = settings.hf_token.clone();
            let model_id = model_id.to_string();
            tokio::spawn(async move {
                let engine = tokio::task::block_in_place(|| {
                    infra::CandleEngine::load(&data_dir, &model_id, hf_token.as_deref())
                });
                let engine = match engine {
                    Ok(e) => e,
                    Err(err) => {
                        eprintln!("AI engine load failed: {err}");
                        return;
                    }
                };
                while let Some((id, text)) = title_req_rx.recv().await {
                    if let Some(title) = infra::ai::titler::generate_title(&text, &engine).await {
                        let _ = title_res_tx.send((id, title)).await;
                    }
                }
            });
            return (Some(title_req_tx), Some(title_res_rx), progress_tx, progress_rx);
        }
    }
    let _ = settings; // suppress unused warning in non-ai build
    let _ = data_dir;
    (None, None, progress_tx, progress_rx)
}
