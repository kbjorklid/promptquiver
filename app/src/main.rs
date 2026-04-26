use anyhow::Result;
use app::app::App;
use app::tui::{self, Tui};
use contracts::{Clipboard, Git, Storage};
use infra::{FileSystemStorage, RealClipboard, RealGit};
use ratatui::Terminal;
use ratatui_toaster::{ToastEngineBuilder, ToastType};
use std::{io, sync::Arc, time::Duration};
use crossterm::event::{Event, KeyCode, KeyEventKind};

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

    // App State
    let mut app = App::new(storage.clone(), clipboard, git.clone());
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
                ui::RenderState {
                    active_tab: app.active_tab,
                    prompts: &app.prompts,
                    selected_index: app.selected_index,
                    mode: mode_str,
                    textarea: &app.textarea,
                    title_textarea: &app.title_textarea,
                    title_focused: app.title_focused,
                    current_branch: app.current_branch.as_deref(),
                    suggestions: &app.suggestions,
                    suggestion_index: app.suggestion_index,
                    search_query: &app.search_query,
                    global_search_query: &app.global_search_query,
                    settings: &app.settings,
                },
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
                                KeyCode::Right | KeyCode::Char('l') => {
                                    app.next_tab();
                                    handle_error!(app, app.load_prompts().await);
                                }
                                KeyCode::Left | KeyCode::Char('h') => {
                                    app.prev_tab();
                                    handle_error!(app, app.load_prompts().await);
                                }
                                KeyCode::Tab
                                    if app.active_tab == contracts::Tab::Settings => {
                                        let tabs_len = contracts::Tab::all().len();
                                        match app.selected_index.cmp(&tabs_len) {
                                            std::cmp::Ordering::Less => app.selected_index = tabs_len, // Jump to Slash Commands
                                            std::cmp::Ordering::Equal => app.selected_index = tabs_len + 1, // Jump to Advanced
                                            std::cmp::Ordering::Greater => app.selected_index = 0, // Jump back to Tab Visibility
                                        }
                                    }
                                KeyCode::BackTab
                                    if app.active_tab == contracts::Tab::Settings => {
                                        let tabs_len = contracts::Tab::all().len();
                                        if app.selected_index == 0 {
                                            app.selected_index = tabs_len + 1;
                                        } else if app.selected_index <= tabs_len {
                                            app.selected_index = 0;
                                        } else {
                                            app.selected_index = tabs_len;
                                        }
                                    }
                                KeyCode::Char('1') => { app.set_tab(contracts::Tab::Prompts); handle_error!(app, app.load_prompts().await); }
                                KeyCode::Char('2') => { app.set_tab(contracts::Tab::Canned); handle_error!(app, app.load_prompts().await); }
                                KeyCode::Char('3') => { app.set_tab(contracts::Tab::Notes); handle_error!(app, app.load_prompts().await); }
                                KeyCode::Char('4') => { app.set_tab(contracts::Tab::Snippets); handle_error!(app, app.load_prompts().await); }
                                KeyCode::Char('5') => { app.set_tab(contracts::Tab::Archive); handle_error!(app, app.load_prompts().await); }
                                KeyCode::Char('6') => { app.set_tab(contracts::Tab::Settings); handle_error!(app, app.load_prompts().await); }
                                KeyCode::Char('u') => {
                                    handle_error!(app, app.undo().await);
                                }
                                KeyCode::Char('y') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                                    handle_error!(app, app.redo().await);
                                }
                                KeyCode::Char('j') | KeyCode::Down => app.move_down(),
                                KeyCode::Char('k') | KeyCode::Up => app.move_up(),
                                KeyCode::Char('s') => {
                                    handle_error!(app, app.stage_selected().await);
                                }
                                KeyCode::Char('d') => {
                                    handle_error!(app, app.archive_selected().await);
                                }
                                KeyCode::Char('r') => {
                                    handle_error!(app, app.restore_selected().await);
                                }
                                KeyCode::Char('a') => {
                                    if app.active_tab == contracts::Tab::Settings {
                                        app.edit_setting();
                                    } else {
                                        app.enter_editor(String::new(), None);
                                    }
                                }
                                KeyCode::Char('i') => {
                                    app.enter_editor_before(String::new(), app.selected_index);
                                }
                                KeyCode::Char('b') => {
                                    app.branch_filter = !app.branch_filter;
                                    handle_error!(app, app.load_prompts().await);
                                    app.notify(format!("Branch filter: {}", if app.branch_filter { "ON" } else { "OFF" }), ToastType::Info);
                                }
                                KeyCode::Char('/') => {
                                    app.mode = app::app::Mode::Search;
                                    app.search_query.clear();
                                }
                                KeyCode::Char('f') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                                    app.mode = app::app::Mode::GlobalSearch;
                                    app.global_search_query.clear();
                                }
                                KeyCode::Char('G') => {
                                    app.move_to_bottom();
                                }
                                KeyCode::Char('g') => {
                                    // Basic gg implementation (needs state to track first 'g')
                                    // For now, let's just make 'g' move to top to simplify, 
                                    // or actually implement a small state.
                                    // To keep main.rs simple, I'll just use 'g' for top for now.
                                    app.move_to_top();
                                }
                                KeyCode::Char('e') | KeyCode::Enter => {
                                    if app.active_tab == contracts::Tab::Settings {
                                        app.edit_setting();
                                    } else if !app.prompts.is_empty() {
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
                                KeyCode::Char(' ')
                                    if app.active_tab == contracts::Tab::Settings => {
                                        handle_error!(app, app.toggle_setting().await);
                                    }
                                KeyCode::Char('y' | 'c') => {
                                    handle_error!(app, app.copy_selected().await);
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
                                    handle_error!(app, app.move_item_down().await);
                                }
                                KeyCode::Char('k') | KeyCode::Up => {
                                    handle_error!(app, app.move_item_up().await);
                                }
                                _ => {}
                            }
                        }
                        app::app::Mode::Search => {
                            match key.code {
                                KeyCode::Esc => {
                                    app.mode = app::app::Mode::List;
                                    app.search_query.clear();
                                    handle_error!(app, app.load_prompts().await);
                                }
                                KeyCode::Enter => {
                                    app.mode = app::app::Mode::List;
                                }
                                KeyCode::Char(c) => {
                                    app.search_query.push(c);
                                    handle_error!(app, app.load_prompts().await);
                                }
                                KeyCode::Backspace => {
                                    app.search_query.pop();
                                    handle_error!(app, app.load_prompts().await);
                                }
                                _ => {}
                            }
                        }
                        app::app::Mode::GlobalSearch => {
                            match key.code {
                                KeyCode::Esc => {
                                    app.mode = app::app::Mode::List;
                                    app.global_search_query.clear();
                                    handle_error!(app, app.load_prompts().await);
                                }
                                KeyCode::Enter => {
                                    app.mode = app::app::Mode::List;
                                    handle_error!(app, app.search_all(app.global_search_query.clone()).await);
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
                                KeyCode::Tab if app.active_tab == contracts::Tab::Snippets => {
                                    app.title_focused = !app.title_focused;
                                }
                                KeyCode::Esc => {
                                    if app.autocomplete_open {
                                        app.autocomplete_open = false;
                                    } else if app.active_tab == contracts::Tab::Settings {
                                        app.exit_editor();
                                    } else {
                                        let current_text = app.textarea.lines().join("\n");
                                        let current_title = app.title_textarea.lines().join("");
                                        // Simple original check for now (mostly for text)
                                        if current_text == app.original_text && (app.active_tab != contracts::Tab::Snippets || current_title.is_empty() || app.editing_id.is_some()) {
                                            app.exit_editor();
                                        } else {
                                            app.mode = app::app::Mode::ConfirmDiscard;
                                        }
                                    }
                                }
                                KeyCode::Char('s') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                                    handle_error!(app, app.save_editor().await);
                                }
                                KeyCode::Char('g') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                                    handle_error!(app, app.save_and_stage_editor().await);
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
                                KeyCode::Enter if app.active_tab == contracts::Tab::Settings => {
                                    handle_error!(app, app.save_editor().await);
                                }
                                _ => {
                                    if app.title_focused && app.active_tab == contracts::Tab::Snippets {
                                        app.title_textarea.input(event);
                                    } else {
                                        if app.active_tab == contracts::Tab::Settings {
                                            // Only allow one line for slash commands
                                            if key.code != KeyCode::Enter {
                                                app.textarea.input(event);
                                                handle_error!(app, app.update_autocomplete().await);
                                            }
                                        } else {
                                            app.textarea.input(event);
                                            handle_error!(app, app.update_autocomplete().await);
                                        }
                                    }
                                }
                            }
                        }
                        app::app::Mode::ConfirmDiscard => {
                            match key.code {
                                KeyCode::Char('y' | 'Y') | KeyCode::Enter => {
                                    app.exit_editor();
                                }
                                KeyCode::Char('n' | 'N') | KeyCode::Esc => {
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
