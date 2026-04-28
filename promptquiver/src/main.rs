use anyhow::Result;
use promptquiver::app::App;
use promptquiver::tui::{self, Tui};
use contracts::{Clipboard, Git, Storage};
use infra::{FileSystemStorage, RealClipboard, RealGit};
use ratatui::Terminal;
use ratatui_toaster::{ToastEngineBuilder, ToastType};
use std::{io, sync::Arc, time::Duration};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyEvent};

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
            if let Some((trigger, current_query)) = app.get_current_autocomplete_query() {
                if trigger == "@" && current_query == query {
                    if !results.is_empty() {
                        app.suggestions = results;
                        app.autocomplete_open = true;
                        app.suggestion_index = 0;
                    } else {
                        app.autocomplete_open = false;
                        app.suggestions.clear();
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
                    active_tab: app.active_tab,
                    prompts: &app.prompts,
                    selected_index: app.selected_index,
                    list_state: &mut app.list_state,
                    settings_slash_list_state: &mut app.settings_slash_list_state,
                    theme_list_state: &mut app.theme_list_state,
                    mode: mode_str,
                    textarea: &app.textarea,
                    title_textarea: &app.title_textarea,
                    title_focused: app.title_focused,
                    current_branch: app.current_branch.as_deref(),
                    current_path: &app.current_path,
                    suggestions: &app.suggestions,
                    suggestion_index: app.suggestion_index,
                    search_query: &app.search_query,
                    global_search_query: &app.global_search_query,
                    settings: &app.settings,
                    throbber_state: &mut app.throbber_state,
                },
                &mut app.toaster,
            );

        })?;

        if let Some(event) = tui::next_event(Duration::from_millis(16))? {
            if let Event::Key(key) = event {
                if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat {
                    match app.mode {
                        promptquiver::app::Mode::List => {
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
                                        let slash_len = app.settings.slash_commands.len();
                                        let advanced_idx = tabs_len + slash_len + 1;

                                        if app.selected_index < tabs_len {
                                            app.selected_index = tabs_len; // Jump to Slash Commands
                                        } else if app.selected_index < advanced_idx {
                                            app.selected_index = advanced_idx; // Jump to Advanced
                                        } else {
                                            app.selected_index = 0; // Jump back to Tab Visibility
                                        }
                                    }
                                KeyCode::BackTab
                                    if app.active_tab == contracts::Tab::Settings => {
                                        let tabs_len = contracts::Tab::all().len();
                                        let slash_len = app.settings.slash_commands.len();
                                        let advanced_idx = tabs_len + slash_len + 1;

                                        if app.selected_index == 0 {
                                            app.selected_index = advanced_idx;
                                        } else if app.selected_index < advanced_idx && app.selected_index >= tabs_len {
                                            app.selected_index = 0;
                                        } else if app.selected_index >= advanced_idx {
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
                                KeyCode::Char('D') => {
                                    handle_error!(app, app.duplicate_selected().await);
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
                                    app.mode = promptquiver::app::Mode::Search;
                                    app.search_query.clear();
                                }
                                KeyCode::Char('f') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                                    app.mode = promptquiver::app::Mode::GlobalSearch;
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
                                        let tabs_len = contracts::Tab::all().len();
                                        let slash_len = app.settings.slash_commands.len();
                                        let advanced_idx = tabs_len + slash_len + 1;
                                        if app.selected_index == advanced_idx + 2 {
                                            app.original_theme = app.settings.theme_name.clone();
                                            app.mode = promptquiver::app::Mode::ThemePicker;
                                        } else {
                                            app.edit_setting();
                                        }
                                    } else if !app.prompts.is_empty() {
                                        let p = &app.prompts[app.selected_index];
                                        app.enter_editor(p.text.clone(), Some(p.id));
                                    }
                                }
                                KeyCode::Char('m') => {
                                    app.mode = if app.mode == promptquiver::app::Mode::Move {
                                        promptquiver::app::Mode::List
                                    } else {
                                        promptquiver::app::Mode::Move
                                    };
                                }
                                KeyCode::Char(' ')
                                    if app.active_tab == contracts::Tab::Settings => {
                                        let tabs_len = contracts::Tab::all().len();
                                        let slash_len = app.settings.slash_commands.len();
                                        let advanced_idx = tabs_len + slash_len + 1;
                                        if app.selected_index == advanced_idx + 2 {
                                            app.original_theme = app.settings.theme_name.clone();
                                            app.mode = promptquiver::app::Mode::ThemePicker;
                                        } else {
                                            handle_error!(app, app.toggle_setting().await);
                                        }
                                    }
                                KeyCode::Char('y' | 'c') => {
                                    handle_error!(app, app.copy_selected().await);
                                }
                                _ => {}
                            }
                        }
                        promptquiver::app::Mode::ThemePicker => {
                            match key.code {
                                KeyCode::Esc => {
                                    app.settings.theme_name = app.original_theme.take();
                                    app.mode = promptquiver::app::Mode::List;
                                }
                                KeyCode::Char('j') | KeyCode::Down => {
                                    let themes = ratatui_themes::ThemeName::all();
                                    let current = app.theme_list_state.selected().unwrap_or(0);
                                    if current < themes.len() - 1 {
                                        let new_idx = current + 1;
                                        app.theme_list_state.select(Some(new_idx));
                                        let theme_name = format!("{:?}", themes[new_idx]);
                                        app.settings.theme_name = Some(theme_name);
                                    }
                                }
                                KeyCode::Char('k') | KeyCode::Up => {
                                    let themes = ratatui_themes::ThemeName::all();
                                    let current = app.theme_list_state.selected().unwrap_or(0);
                                    if current > 0 {
                                        let new_idx = current - 1;
                                        app.theme_list_state.select(Some(new_idx));
                                        let theme_name = format!("{:?}", themes[new_idx]);
                                        app.settings.theme_name = Some(theme_name);
                                    }
                                }
                                KeyCode::Enter | KeyCode::Char(' ') => {
                                    let themes = ratatui_themes::ThemeName::all();
                                    let selected = app.theme_list_state.selected().unwrap_or(0);
                                    let theme_name = format!("{:?}", themes[selected]);
                                    app.settings.theme_name = Some(theme_name);
                                    app.original_theme = None;
                                    handle_error!(app, app.storage.save_settings(app.settings.clone()).await);
                                    app.mode = promptquiver::app::Mode::List;
                                    app.notify("Theme updated!", ToastType::Success);
                                }
                                _ => {}
                            }
                        }
                        promptquiver::app::Mode::Move => {
                            match key.code {
                                KeyCode::Esc | KeyCode::Char('m') | KeyCode::Enter => {
                                    app.mode = promptquiver::app::Mode::List;
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
                        promptquiver::app::Mode::Search => {
                            match key.code {
                                KeyCode::Esc => {
                                    app.mode = promptquiver::app::Mode::List;
                                    app.search_query.clear();
                                    handle_error!(app, app.load_prompts().await);
                                }
                                KeyCode::Enter => {
                                    app.mode = promptquiver::app::Mode::List;
                                }
                                KeyCode::Char('\u{7f}') => {
                                    if let Some(pos) = app.search_query.trim_end().rfind(' ') {
                                        app.search_query.truncate(pos + 1);
                                    } else {
                                        app.search_query.clear();
                                    }
                                    handle_error!(app, app.load_prompts().await);
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
                        promptquiver::app::Mode::GlobalSearch => {
                            match key.code {
                                KeyCode::Esc => {
                                    app.mode = promptquiver::app::Mode::List;
                                    app.global_search_query.clear();
                                    handle_error!(app, app.load_prompts().await);
                                }
                                KeyCode::Enter => {
                                    app.mode = promptquiver::app::Mode::List;
                                    handle_error!(app, app.search_all(app.global_search_query.clone()).await);
                                }
                                KeyCode::Backspace if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                                    if let Some(pos) = app.global_search_query.trim_end().rfind(' ') {
                                        app.global_search_query.truncate(pos + 1);
                                    } else {
                                        app.global_search_query.clear();
                                    }
                                    handle_error!(app, app.search_all(app.global_search_query.clone()).await);
                                }
                                KeyCode::Char('\u{7f}') => {
                                    if let Some(pos) = app.global_search_query.trim_end().rfind(' ') {
                                        app.global_search_query.truncate(pos + 1);
                                    } else {
                                        app.global_search_query.clear();
                                    }
                                    handle_error!(app, app.search_all(app.global_search_query.clone()).await);
                                }
                                KeyCode::Char(c) => {
                                    app.global_search_query.push(c);
                                    handle_error!(app, app.search_all(app.global_search_query.clone()).await);
                                }
                                KeyCode::Backspace => {
                                    app.global_search_query.pop();
                                    handle_error!(app, app.search_all(app.global_search_query.clone()).await);
                                }
                                _ => {}
                            }
                        }
                        promptquiver::app::Mode::Editor => {
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
                                        
                                        let title_changed = if app.active_tab == contracts::Tab::Snippets {
                                            if let Some(id) = app.editing_id {
                                                let original_title = app.prompts.iter().find(|p| p.id == id).and_then(|p| p.name.clone()).unwrap_or_default();
                                                current_title != original_title
                                            } else {
                                                !current_title.is_empty()
                                            }
                                        } else {
                                            false
                                        };

                                        if current_text == app.original_text && !title_changed {
                                            app.exit_editor();
                                        } else {
                                            app.mode = promptquiver::app::Mode::ConfirmDiscard;
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
                                KeyCode::Enter if app.title_focused && app.active_tab == contracts::Tab::Snippets => {
                                    app.title_focused = false;
                                }
                                KeyCode::Backspace if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                                    if app.title_focused && app.active_tab == contracts::Tab::Snippets {
                                        app.title_textarea.delete_word();
                                    } else {
                                        app.textarea.delete_word();
                                        handle_error!(app, app.update_autocomplete().await);
                                    }
                                }
                                KeyCode::Char('\u{7f}') => {
                                    if app.title_focused && app.active_tab == contracts::Tab::Snippets {
                                        app.title_textarea.delete_word();
                                    } else {
                                        app.textarea.delete_word();
                                        handle_error!(app, app.update_autocomplete().await);
                                    }
                                }
                                _ => {
                                    if app.title_focused && app.active_tab == contracts::Tab::Snippets {
                                        if !app.title_textarea.input(event) {
                                            if let KeyCode::Char(c) = key.code {
                                                app.title_textarea.input(KeyEvent::new(KeyCode::Char(c), crossterm::event::KeyModifiers::empty()));
                                            }
                                        }
                                        // Ensure it stays single line (e.g. after paste)
                                        if app.title_textarea.lines().len() > 1 {
                                            let joined = app.title_textarea.lines().join("");
                                            app.title_textarea = ratatui_textarea::TextArea::new(vec![joined]);
                                            app.title_textarea.move_cursor(ratatui_textarea::CursorMove::End);
                                        }
                                    } else {
                                        if app.active_tab == contracts::Tab::Settings {
                                            // Only allow one line for slash commands
                                            if key.code != KeyCode::Enter {
                                                if !app.textarea.input(event) {
                                                    if let KeyCode::Char(c) = key.code {
                                                        app.textarea.input(KeyEvent::new(KeyCode::Char(c), crossterm::event::KeyModifiers::empty()));
                                                    }
                                                }
                                                handle_error!(app, app.update_autocomplete().await);
                                            }
                                        } else {
                                            if !app.textarea.input(event) {
                                                if let KeyCode::Char(c) = key.code {
                                                    app.textarea.input(KeyEvent::new(KeyCode::Char(c), crossterm::event::KeyModifiers::empty()));
                                                }
                                            }
                                            handle_error!(app, app.update_autocomplete().await);
                                        }
                                    }
                                }
                            }
                        }
                        promptquiver::app::Mode::ConfirmDiscard => {
                            match key.code {
                                KeyCode::Char('y' | 'Y') | KeyCode::Enter => {
                                    app.exit_editor();
                                }
                                KeyCode::Char('n' | 'N') | KeyCode::Esc => {
                                    app.mode = promptquiver::app::Mode::Editor;
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
