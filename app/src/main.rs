use anyhow::Result;
use app::app::App;
use app::tui::{self, Tui};
use contracts::{Clipboard, Git, Storage};
use infra::{FileSystemStorage, RealClipboard, RealGit};
use ratatui::Terminal;
use ratatui_toaster::{ToastEngineBuilder, ToastType};
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
                app::app::Mode::Move => "Move",
                app::app::Mode::Search => "Search",
                app::app::Mode::GlobalSearch => "Global Search",
                app::app::Mode::ConfirmDiscard => "Confirm Discard",
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
                &app.search_query,
                &app.global_search_query,
                &app.settings,
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
                                KeyCode::Char('u') => {
                                    app.undo().await?;
                                }
                                KeyCode::Char('y') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                                    app.redo().await?;
                                }
                                KeyCode::Char('j') | KeyCode::Down => app.move_down(),
                                KeyCode::Char('k') | KeyCode::Up => app.move_up(),
                                KeyCode::Char('s') => {
                                    app.stage_selected().await?;
                                }
                                KeyCode::Char('d') => {
                                    app.archive_selected().await?;
                                }
                                KeyCode::Char('r') => {
                                    app.restore_selected().await?;
                                }
                                KeyCode::Char('a') => {
                                    app.enter_editor(String::new(), None);
                                }
                                KeyCode::Char('i') => {
                                    app.enter_editor_before(String::new(), app.selected_index);
                                }
                                KeyCode::Char('b') => {
                                    app.branch_filter = !app.branch_filter;
                                    app.load_prompts().await?;
                                    app.notify(format!("Branch filter: {}", if app.branch_filter { "ON" } else { "OFF" }), ToastType::Info);
                                }
                                KeyCode::Char('/') => {
                                    app.mode = app::app::Mode::Search;
                                    app.search_query.clear();
                                }
                                KeyCode::Char('G') | KeyCode::Char('g') if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) => {
                                    app.mode = app::app::Mode::GlobalSearch;
                                    app.global_search_query.clear();
                                }
                                KeyCode::Char('e') | KeyCode::Enter => {
                                    if !app.prompts.is_empty() {
                                        let p = &app.prompts[app.selected_index];
                                        app.enter_editor(p.text.clone(), Some(p.id));
                                    }
                                }
                                KeyCode::Char('m') => {
                                    app.mode = if app.mode == app::app::Mode::Move {
                                        app::app::Mode::List
                                    } else {
                                        app::app::Mode::Move
                                    };
                                }
                                KeyCode::Char(' ') => {
                                    if app.active_tab == contracts::Tab::Settings {
                                        app.toggle_setting().await?;
                                    }
                                }
                                KeyCode::Char('y') | KeyCode::Char('c') => {
                                    app.copy_selected().await?;
                                }
                                _ => {}
                            }
                        }
                        app::app::Mode::Move => {
                            match key.code {
                                KeyCode::Esc | KeyCode::Char('m') | KeyCode::Enter => {
                                    app.mode = app::app::Mode::List;
                                }
                                KeyCode::Char('j') | KeyCode::Down => {
                                    app.move_item_down().await?;
                                }
                                KeyCode::Char('k') | KeyCode::Up => {
                                    app.move_item_up().await?;
                                }
                                _ => {}
                            }
                        }
                        app::app::Mode::Search => {
                            match key.code {
                                KeyCode::Esc => {
                                    app.mode = app::app::Mode::List;
                                    app.search_query.clear();
                                    app.load_prompts().await?;
                                }
                                KeyCode::Enter => {
                                    app.mode = app::app::Mode::List;
                                }
                                KeyCode::Char(c) => {
                                    app.search_query.push(c);
                                    app.load_prompts().await?;
                                }
                                KeyCode::Backspace => {
                                    app.search_query.pop();
                                    app.load_prompts().await?;
                                }
                                _ => {}
                            }
                        }
                        app::app::Mode::GlobalSearch => {
                            match key.code {
                                KeyCode::Esc => {
                                    app.mode = app::app::Mode::List;
                                    app.global_search_query.clear();
                                    app.load_prompts().await?;
                                }
                                KeyCode::Enter => {
                                    app.mode = app::app::Mode::List;
                                    app.search_all(app.global_search_query.clone()).await?;
                                }
                                KeyCode::Char(c) => {
                                    app.global_search_query.push(c);
                                }
                                KeyCode::Backspace => {
                                    app.global_search_query.pop();
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
                                        let current_text = app.textarea.lines().join("\n");
                                        if current_text != app.original_text {
                                            app.mode = app::app::Mode::ConfirmDiscard;
                                        } else {
                                            app.exit_editor();
                                        }
                                    }
                                }
                                KeyCode::Char('s') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                                    app.save_editor().await?;
                                }
                                KeyCode::Char('g') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                                    app.save_and_stage_editor().await?;
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
                        app::app::Mode::ConfirmDiscard => {
                            match key.code {
                                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                                    app.exit_editor();
                                }
                                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                                    app.mode = app::app::Mode::Editor;
                                }
                                _ => {}
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
