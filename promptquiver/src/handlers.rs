use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::app::{App, Mode};
use contracts::Tab;
use ratatui_toaster::ToastType;

macro_rules! handle_error {
    ($app:expr, $res:expr) => {
        if let Err(e) = $res {
            $app.notify(format!("Error: {}", e), ToastType::Error);
        }
    };
}

pub async fn handle_key_event(app: &mut App<'_>, key: KeyEvent) {
    match app.mode {
        Mode::List => handle_list_events(app, key).await,
        Mode::Editor => handle_editor_events(app, key).await,
        Mode::Move => handle_move_events(app, key).await,
        Mode::Search => handle_search_events(app, key).await,
        Mode::GlobalSearch => handle_global_search_events(app, key).await,
        Mode::ConfirmDiscard => handle_confirm_discard_events(app, key).await,
        Mode::ThemePicker => handle_theme_picker_events(app, key).await,
    }
}

async fn handle_list_events(app: &mut App<'_>, key: KeyEvent) {
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
        KeyCode::Tab if app.active_tab == Tab::Settings => {
            let tabs_len = Tab::all().len();
            let slash_len = app.settings.slash_commands.len();
            let advanced_idx = tabs_len + slash_len + 1;

            if app.selected_index < tabs_len {
                app.selected_index = tabs_len;
            } else if app.selected_index < advanced_idx {
                app.selected_index = advanced_idx;
            } else {
                app.selected_index = 0;
            }
        }
        KeyCode::BackTab if app.active_tab == Tab::Settings => {
            let tabs_len = Tab::all().len();
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
        KeyCode::Char('1') => { app.set_tab(Tab::Prompts); handle_error!(app, app.load_prompts().await); }
        KeyCode::Char('2') => { app.set_tab(Tab::Canned); handle_error!(app, app.load_prompts().await); }
        KeyCode::Char('3') => { app.set_tab(Tab::Notes); handle_error!(app, app.load_prompts().await); }
        KeyCode::Char('4') => { app.set_tab(Tab::Snippets); handle_error!(app, app.load_prompts().await); }
        KeyCode::Char('5') => { app.set_tab(Tab::Archive); handle_error!(app, app.load_prompts().await); }
        KeyCode::Char('6') => { app.set_tab(Tab::Settings); handle_error!(app, app.load_prompts().await); }
        KeyCode::Char('u') => { handle_error!(app, app.undo().await); }
        KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => { handle_error!(app, app.redo().await); }
        KeyCode::Char('j') | KeyCode::Down => app.move_down(),
        KeyCode::Char('k') | KeyCode::Up => app.move_up(),
        KeyCode::Char('s') => { handle_error!(app, app.stage_selected().await); }
        KeyCode::Char('d') => { handle_error!(app, app.archive_selected().await); }
        KeyCode::Char('D') => { handle_error!(app, app.duplicate_selected().await); }
        KeyCode::Char('r') => { handle_error!(app, app.restore_selected().await); }
        KeyCode::Char('a') => {
            if app.active_tab == Tab::Settings {
                app.edit_setting();
            } else {
                app.enter_editor(String::new(), None);
            }
        }
        KeyCode::Char('i') => { app.enter_editor_before(String::new(), app.selected_index); }
        KeyCode::Char('b') => {
            app.branch_filter = !app.branch_filter;
            handle_error!(app, app.load_prompts().await);
            app.notify(format!("Branch filter: {}", if app.branch_filter { "ON" } else { "OFF" }), ToastType::Info);
        }
        KeyCode::Char('/') => {
            app.mode = Mode::Search;
            app.search_query.clear();
        }
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.mode = Mode::GlobalSearch;
            app.global_search_query.clear();
        }
        KeyCode::Char('G') => { app.move_to_bottom(); }
        KeyCode::Char('g') => { app.move_to_top(); }
        KeyCode::Char('e') | KeyCode::Enter => {
            if app.active_tab == Tab::Settings {
                let tabs_len = Tab::all().len();
                let slash_len = app.settings.slash_commands.len();
                let advanced_idx = tabs_len + slash_len + 1;
                if app.selected_index == advanced_idx + 2 {
                    app.original_theme = app.settings.theme_name.clone();
                    app.mode = Mode::ThemePicker;
                } else {
                    app.edit_setting();
                }
            } else if !app.prompts.is_empty() {
                let p = &app.prompts[app.selected_index];
                app.enter_editor(p.text.clone(), Some(p.id));
            }
        }
        KeyCode::Char('m') => {
            app.mode = if app.mode == Mode::Move { Mode::List } else { Mode::Move };
        }
        KeyCode::Char(' ') if app.active_tab == Tab::Settings => {
            let tabs_len = Tab::all().len();
            let slash_len = app.settings.slash_commands.len();
            let advanced_idx = tabs_len + slash_len + 1;
            if app.selected_index == advanced_idx + 2 {
                app.original_theme = app.settings.theme_name.clone();
                app.mode = Mode::ThemePicker;
            } else {
                handle_error!(app, app.toggle_setting().await);
            }
        }
        KeyCode::Char('y' | 'c') => { handle_error!(app, app.copy_selected().await); }
        _ => {}
    }
}

async fn handle_editor_events(app: &mut App<'_>, key: KeyEvent) {
    match key.code {
        KeyCode::Tab if app.active_tab == Tab::Snippets => {
            app.title_focused = !app.title_focused;
        }
        KeyCode::Esc => {
            if app.autocomplete_open {
                app.autocomplete_open = false;
                app.suggestions.clear();
            } else if app.active_tab == Tab::Settings {
                app.exit_editor();
            } else {
                let current_text = app.textarea.lines().join("\n");
                let current_title = app.title_textarea.lines().join("");
                
                let title_changed = if app.active_tab == Tab::Snippets {
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
                    app.mode = Mode::ConfirmDiscard;
                }
            }
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            handle_error!(app, app.save_editor().await);
        }
        KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            handle_error!(app, app.save_and_stage_editor().await);
        }
        KeyCode::Up if app.autocomplete_open => { app.move_suggestion_up(); }
        KeyCode::Down if app.autocomplete_open => { app.move_suggestion_down(); }
        KeyCode::Enter if app.autocomplete_open => { app.select_suggestion(); }
        KeyCode::Enter if app.active_tab == Tab::Settings => { handle_error!(app, app.save_editor().await); }
        KeyCode::Enter if app.title_focused && app.active_tab == Tab::Snippets => { app.title_focused = false; }
        KeyCode::Backspace if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.title_focused && app.active_tab == Tab::Snippets {
                app.title_textarea.delete_word();
            } else {
                app.textarea.delete_word();
                handle_error!(app, app.update_autocomplete().await);
            }
        }
        KeyCode::Char('\u{7f}') => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                if app.title_focused && app.active_tab == Tab::Snippets {
                    app.title_textarea.delete_word();
                } else {
                    app.textarea.delete_word();
                    handle_error!(app, app.update_autocomplete().await);
                }
            } else {
                let mut bs_key = key;
                bs_key.code = KeyCode::Backspace;
                if app.title_focused && app.active_tab == Tab::Snippets {
                    app.title_textarea.input(bs_key);
                } else {
                    app.textarea.input(bs_key);
                    handle_error!(app, app.update_autocomplete().await);
                }
            }
        }
        _ => {
            if app.title_focused && app.active_tab == Tab::Snippets {
                if !app.title_textarea.input(key) {
                    if let KeyCode::Char(c) = key.code {
                        app.title_textarea.input(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()));
                    }
                }
                if app.title_textarea.lines().len() > 1 {
                    let joined = app.title_textarea.lines().join("");
                    app.title_textarea = ratatui_textarea::TextArea::new(vec![joined]);
                    app.title_textarea.move_cursor(ratatui_textarea::CursorMove::End);
                }
            } else {
                if app.active_tab == Tab::Settings {
                    if key.code != KeyCode::Enter {
                        if !app.textarea.input(key) {
                            if let KeyCode::Char(c) = key.code {
                                app.textarea.input(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()));
                            }
                        }
                        handle_error!(app, app.update_autocomplete().await);
                    }
                } else {
                    if !app.textarea.input(key) {
                        if let KeyCode::Char(c) = key.code {
                            app.textarea.input(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()));
                        }
                    }
                    handle_error!(app, app.update_autocomplete().await);
                }
            }
        }
    }
}

async fn handle_move_events(app: &mut App<'_>, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('m') | KeyCode::Enter => { app.mode = Mode::List; }
        KeyCode::Char('j') | KeyCode::Down => { handle_error!(app, app.move_item_down().await); }
        KeyCode::Char('k') | KeyCode::Up => { handle_error!(app, app.move_item_up().await); }
        _ => {}
    }
}

async fn handle_search_events(app: &mut App<'_>, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::List;
            app.search_query.clear();
            handle_error!(app, app.load_prompts().await);
        }
        KeyCode::Enter => { app.mode = Mode::List; }
        KeyCode::Char('\u{7f}') => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                if let Some(pos) = app.search_query.trim_end().rfind(' ') {
                    app.search_query.truncate(pos + 1);
                } else {
                    app.search_query.clear();
                }
            } else {
                app.search_query.pop();
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

async fn handle_global_search_events(app: &mut App<'_>, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::List;
            app.global_search_query.clear();
            handle_error!(app, app.load_prompts().await);
        }
        KeyCode::Enter => {
            app.mode = Mode::List;
            handle_error!(app, app.search_all(app.global_search_query.clone()).await);
        }
        KeyCode::Char('\u{7f}') => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                if let Some(pos) = app.global_search_query.trim_end().rfind(' ') {
                    app.global_search_query.truncate(pos + 1);
                } else {
                    app.global_search_query.clear();
                }
            } else {
                app.global_search_query.pop();
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

async fn handle_confirm_discard_events(app: &mut App<'_>, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y' | 'Y') | KeyCode::Enter => { app.exit_editor(); }
        KeyCode::Char('n' | 'N') | KeyCode::Esc => { app.mode = Mode::Editor; }
        _ => {}
    }
}

async fn handle_theme_picker_events(app: &mut App<'_>, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.settings.theme_name = app.original_theme.take();
            app.mode = Mode::List;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let themes = ratatui_themes::ThemeName::all();
            let current = app.theme_list_state.selected().unwrap_or(0);
            if current < themes.len() - 1 {
                let new_idx = current + 1;
                app.theme_list_state.select(Some(new_idx));
                app.settings.theme_name = Some(format!("{:?}", themes[new_idx]));
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let themes = ratatui_themes::ThemeName::all();
            let current = app.theme_list_state.selected().unwrap_or(0);
            if current > 0 {
                let new_idx = current - 1;
                app.theme_list_state.select(Some(new_idx));
                app.settings.theme_name = Some(format!("{:?}", themes[new_idx]));
            }
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            let themes = ratatui_themes::ThemeName::all();
            let selected = app.theme_list_state.selected().unwrap_or(0);
            app.settings.theme_name = Some(format!("{:?}", themes[selected]));
            app.original_theme = None;
            handle_error!(app, app.storage.save_settings(app.settings.clone()).await);
            app.mode = Mode::List;
            app.notify("Theme updated!", ToastType::Success);
        }
        _ => {}
    }
}
