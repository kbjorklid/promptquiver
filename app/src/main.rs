use anyhow::Result;
use app::app::App;
use app::tui::{self, Tui};
use contracts::{Clipboard, Git, Storage};
use infra::{FileSystemStorage, RealClipboard, RealGit};
use ratatui::Terminal;
use ratatui_toaster::ToastEngineBuilder;
use std::{io, sync::Arc, time::Duration};
use crossterm::event::{Event, KeyCode, KeyEventKind};

#[tokio::main]
async fn main() -> Result<()> {
    // Infrastructure
    let storage: Arc<dyn Storage> = Arc::new(FileSystemStorage::new(None));
    let clipboard: Arc<dyn Clipboard> = Arc::new(RealClipboard::new());
    let git: Arc<dyn Git> = Arc::new(RealGit::new());

    // App State
    let mut app = App::new(storage.clone(), clipboard, git.clone());
    app.load_prompts().await?;

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

        tui.terminal.draw(|f| {
            let mode_str = match app.mode {
                app::app::Mode::List => "List",
                app::app::Mode::Editor => "Editor",
            };
            ui::render(
                f,
                app.active_tab,
                &app.prompts,
                app.selected_index,
                mode_str,
                &app.textarea,
                app.current_branch.as_deref(),
                &app.suggestions,
                app.suggestion_index,
                &mut app.toaster,
            );
        })?;

        if let Some(event) = tui::next_event(Duration::from_millis(16))? {
            if let Event::Key(key) = event {
                if key.kind == KeyEventKind::Press {
                    match app.mode {
                        app::app::Mode::List => {
                            match key.code {
                                KeyCode::Char('q') => app.quit(),
                                KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                                    app.next_tab();
                                    app.load_prompts().await?;
                                }
                                KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                                    app.prev_tab();
                                    app.load_prompts().await?;
                                }
                                KeyCode::Char('1') => { app.set_tab(contracts::Tab::Prompts); app.load_prompts().await?; }
                                KeyCode::Char('2') => { app.set_tab(contracts::Tab::Canned); app.load_prompts().await?; }
                                KeyCode::Char('3') => { app.set_tab(contracts::Tab::Notes); app.load_prompts().await?; }
                                KeyCode::Char('4') => { app.set_tab(contracts::Tab::Snippets); app.load_prompts().await?; }
                                KeyCode::Char('5') => { app.set_tab(contracts::Tab::Archive); app.load_prompts().await?; }
                                KeyCode::Char('6') => { app.set_tab(contracts::Tab::Settings); app.load_prompts().await?; }
                                KeyCode::Char('j') | KeyCode::Down => app.move_down(),
                                KeyCode::Char('k') | KeyCode::Up => app.move_up(),
                                KeyCode::Char('s') => {
                                    app.stage_selected().await?;
                                }
                                KeyCode::Char('d') => {
                                    app.archive_selected().await?;
                                }
                                KeyCode::Char('a') => {
                                    app.enter_editor(String::new(), None);
                                }
                                KeyCode::Char('e') | KeyCode::Enter => {
                                    if !app.prompts.is_empty() {
                                        let p = &app.prompts[app.selected_index];
                                        app.enter_editor(p.text.clone(), Some(p.id));
                                    }
                                }
                                _ => {}
                            }
                        }
                        app::app::Mode::Editor => {
                            match key.code {
                                KeyCode::Esc => {
                                    if app.autocomplete_open {
                                        app.autocomplete_open = false;
                                    } else {
                                        app.exit_editor();
                                    }
                                }
                                KeyCode::Char('s') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                                    app.save_editor().await?;
                                }
                                KeyCode::Up if app.autocomplete_open => {
                                    app.move_suggestion_up();
                                }
                                KeyCode::Down if app.autocomplete_open => {
                                    app.move_suggestion_down();
                                }
                                KeyCode::Enter if app.autocomplete_open => {
                                    app.select_suggestion();
                                }
                                _ => {
                                    app.textarea.input(event);
                                    app.update_autocomplete().await?;
                                }
                            }
                        }
                    }
                }
            }
        }

        app.tick();
    }

    tui.exit()?;

    Ok(())
}
