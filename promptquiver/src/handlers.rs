use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, Event};
use crate::app::{App, Mode, AppMessage};
use contracts::Tab;
use ratatui_toaster::ToastType;

pub async fn handle_events(app: &mut App<'_>, events: Vec<Event>) {
    let mut i = 0;
    while i < events.len() {
        let event = &events[i];
        
        // Performance optimization: Batch sequential character inputs (simulated paste)
        if let Event::Key(KeyEvent { code: KeyCode::Char(c), modifiers, kind, .. }) = event {
            let is_press = *kind == crossterm::event::KeyEventKind::Press || *kind == crossterm::event::KeyEventKind::Repeat;
            let is_simple_char = !modifiers.contains(KeyModifiers::CONTROL) && !modifiers.contains(KeyModifiers::ALT);
            
            if is_press && is_simple_char && app.mode == Mode::Editor && !app.editor.title_focused {
                let mut content = String::from(*c);
                let mut j = i + 1;
                while j < events.len() {
                    if let Event::Key(KeyEvent { code: KeyCode::Char(nc), modifiers: nm, kind: nk, .. }) = &events[j] {
                        let is_next_press = *nk == crossterm::event::KeyEventKind::Press || *nk == crossterm::event::KeyEventKind::Repeat;
                        let is_next_simple = !nm.contains(KeyModifiers::CONTROL) && !nm.contains(KeyModifiers::ALT);
                        
                        if is_next_press && is_next_simple {
                            content.push(*nc);
                            j += 1;
                            continue;
                        } else if !is_next_press {
                            // Skip release events for the characters we've already handled or are handling
                            j += 1;
                            continue;
                        }
                    }
                    break;
                }
                
                if content.len() > 1 {
                    // We found a burst of characters, process as a single Paste
                    if let Err(e) = app.handle_message(AppMessage::Paste(content)).await {
                        app.notify(format!("Error: {}", e), ToastType::Error);
                    }
                    i = j; // Skip all batched characters (including their release events)
                    continue;
                }
            }
        }

        // Normal event processing
        let messages = match event {
            Event::Key(key) => {
                if key.kind == crossterm::event::KeyEventKind::Press || key.kind == crossterm::event::KeyEventKind::Repeat {
                    match app.mode {
                        Mode::List => handle_list_events(app, *key),
                        Mode::Editor => handle_editor_events(app, *key),
                        Mode::Move => handle_move_events(app, *key),
                        Mode::Search => handle_search_events(app, *key),
                        Mode::ConfirmDiscard => handle_confirm_discard_events(app, *key),
                        Mode::ThemePicker => handle_theme_picker_events(app, *key),
                    }
                } else {
                    Vec::new()
                }
            }
            Event::Paste(content) => vec![AppMessage::Paste(content.clone())],
            _ => Vec::new(),
        };

        for msg in messages {
            if let Err(e) = app.handle_message(msg).await {
                app.notify(format!("Error: {}", e), ToastType::Error);
            }
        }
        i += 1;
    }
}

pub async fn handle_event(app: &mut App<'_>, event: Event) {
    handle_events(app, vec![event]).await;
}

pub async fn handle_key_event(app: &mut App<'_>, key: KeyEvent) {
    handle_event(app, Event::Key(key)).await;
}

fn handle_list_events(app: &App<'_>, key: KeyEvent) -> Vec<AppMessage> {
    let mut messages = Vec::new();
    match key.code {
        KeyCode::Char('q') => messages.push(AppMessage::Quit),
        KeyCode::Right | KeyCode::Char('l') => messages.push(AppMessage::NextTab),
        KeyCode::Left | KeyCode::Char('h') => messages.push(AppMessage::PrevTab),
        KeyCode::Tab if app.nav.active_tab == Tab::Settings => {
            let tabs_len = Tab::settings_display_len();
            let slash_len = app.settings.slash_commands.len();
            let advanced_idx = tabs_len + slash_len + 1;

            if app.nav.selected_index < tabs_len {
                messages.push(AppMessage::MoveDown); // Simplification: we'd need a specific jump message for exact match
            } else if app.nav.selected_index < advanced_idx {
                messages.push(AppMessage::MoveDown);
            } else {
                messages.push(AppMessage::MoveToTop);
            }
        }
        KeyCode::Char('1') => messages.push(AppMessage::SetTab(Tab::Prompts)),
        KeyCode::Char('2') => messages.push(AppMessage::SetTab(Tab::Canned)),
        KeyCode::Char('3') => messages.push(AppMessage::SetTab(Tab::Notes)),
        KeyCode::Char('4') => messages.push(AppMessage::SetTab(Tab::Snippets)),
        KeyCode::Char('5') => messages.push(AppMessage::SetTab(Tab::Archive)),
        KeyCode::Char('6') => messages.push(AppMessage::SetTab(Tab::Settings)),
        KeyCode::Char('u') => messages.push(AppMessage::Undo),
        KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => messages.push(AppMessage::Redo),
        KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            messages.push(AppMessage::CyclePreviewMode);
        }
        KeyCode::Char('j') | KeyCode::Down => messages.push(AppMessage::MoveDown),
        KeyCode::Char('k') | KeyCode::Up => messages.push(AppMessage::MoveUp),
        KeyCode::Char('s') => messages.push(AppMessage::StageSelected),
        KeyCode::Char('d') => messages.push(AppMessage::ArchiveSelected),
        KeyCode::Char('D') => messages.push(AppMessage::DuplicateSelected),
        KeyCode::Char('r') => messages.push(AppMessage::RestoreSelected),
        KeyCode::Char('a') => {
            if app.nav.active_tab == Tab::Settings {
                messages.push(AppMessage::EditSetting);
            } else {
                messages.push(AppMessage::EnterEditor(String::new(), None));
            }
        }
        KeyCode::Char('i') => messages.push(AppMessage::EnterEditorBefore(String::new(), app.nav.selected_index)),
        KeyCode::Char('b') => messages.push(AppMessage::ToggleBranchFilter),
        KeyCode::Char('f') => messages.push(AppMessage::ToggleFolderFilter),
        KeyCode::Char('/') => {
            messages.push(AppMessage::Search(String::new()));
        }
        KeyCode::Char('G') => messages.push(AppMessage::MoveToBottom),
        KeyCode::Char('g') => messages.push(AppMessage::MoveToTop),
        KeyCode::Char('e') | KeyCode::Enter => {
            if app.nav.active_tab == Tab::Settings {
                let tabs_len = Tab::settings_display_len();
                let slash_len = app.settings.slash_commands.len();
                let advanced_idx = tabs_len + slash_len + 1;
                if app.nav.selected_index == advanced_idx + 2 {
                    messages.push(AppMessage::SelectTheme);
                } else {
                    messages.push(AppMessage::EditSetting);
                }
            } else if !app.nav.prompts.is_empty() {
                let p = &app.nav.prompts[app.nav.selected_index];
                messages.push(AppMessage::EnterEditor(p.text.clone(), Some(p.id)));
            }
        }
        KeyCode::Char('m') => messages.push(AppMessage::ToggleMoveMode),
        KeyCode::Char(' ') if app.nav.active_tab == Tab::Settings => {
             messages.push(AppMessage::ToggleSetting);
        }
        KeyCode::Char('y' | 'c') => {
             messages.push(AppMessage::CopySelected);
        }
        _ => {}
    }
    messages
}

fn handle_editor_events(app: &App<'_>, key: KeyEvent) -> Vec<AppMessage> {
    let mut messages = Vec::new();
    match key.code {
        KeyCode::Esc => {
            if app.editor.autocomplete.open {
                messages.push(AppMessage::CloseAutocomplete);
            } else {
                // Exit or confirm discard
                messages.push(AppMessage::ExitEditor); // Or ConfirmDiscard
            }
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            messages.push(AppMessage::SaveEditor);
        }
        KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            messages.push(AppMessage::SaveAndStageEditor);
        }
        KeyCode::Up if app.editor.autocomplete.open => { messages.push(AppMessage::MoveSuggestionUp); }
        KeyCode::Down if app.editor.autocomplete.open => { messages.push(AppMessage::MoveSuggestionDown); }
        KeyCode::Enter if app.editor.autocomplete.open => { messages.push(AppMessage::SelectSuggestion); }
        _ => {
            messages.push(AppMessage::EditorInput(key));
        }
    }
    messages
}

fn handle_move_events(_app: &App<'_>, key: KeyEvent) -> Vec<AppMessage> {
    let mut messages = Vec::new();
    match key.code {
        KeyCode::Esc | KeyCode::Char('m') | KeyCode::Enter => { messages.push(AppMessage::ToggleMoveMode); }
        KeyCode::Char('j') | KeyCode::Down => { messages.push(AppMessage::MoveItemDown); }
        KeyCode::Char('k') | KeyCode::Up => { messages.push(AppMessage::MoveItemUp); }
        _ => {}
    }
    messages
}

fn handle_search_events(_app: &App<'_>, key: KeyEvent) -> Vec<AppMessage> {
    let mut messages = Vec::new();
    messages.push(AppMessage::SearchInput(key));
    messages
}

fn handle_confirm_discard_events(_app: &App<'_>, key: KeyEvent) -> Vec<AppMessage> {
    let mut messages = Vec::new();
    match key.code {
        KeyCode::Char('y' | 'Y') | KeyCode::Enter => { messages.push(AppMessage::ExitEditor); }
        KeyCode::Char('n' | 'N') | KeyCode::Esc => { messages.push(AppMessage::CancelDiscard); }
        _ => {}
    }
    messages
}

fn handle_theme_picker_events(_app: &App<'_>, key: KeyEvent) -> Vec<AppMessage> {
    let mut messages = Vec::new();
    messages.push(AppMessage::ThemePickerInput(key));
    messages
}
