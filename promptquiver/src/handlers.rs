use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, Event};
use crate::app::{App, Mode, AppMessage};
use contracts::Tab;
use ratatui_toaster::ToastType;
use ui::shortcuts::{get_action, ShortcutAction};

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
                    if let Some(action) = get_action(*key, app.mode, app.nav.active_tab, app.editor.autocomplete.open) {
                        map_action_to_messages(app, action)
                    } else {
                        // Fallback for keys not handled by ShortcutAction (e.g. typing in editor)
                        match app.mode {
                            Mode::Editor => vec![AppMessage::EditorInput(*key)],
                            Mode::Search => vec![AppMessage::SearchInput(*key)],
                            Mode::ThemePicker => vec![AppMessage::ThemePickerInput(*key)],
                            Mode::ProjectPicker => vec![AppMessage::ProjectPickerInput(*key)],
                            Mode::AddProject => handle_add_project_events(app, *key),
                            _ => Vec::new(),
                        }
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

fn map_action_to_messages(app: &App<'_>, action: ShortcutAction) -> Vec<AppMessage> {
    let mut messages = Vec::new();
    match action {
        ShortcutAction::Quit => messages.push(AppMessage::Quit),
        ShortcutAction::NextTab => {
            if app.mode == Mode::List && app.nav.active_tab == Tab::Settings {
                let tabs_len = Tab::settings_display_len();
                let slash_len = app.settings.slash_commands.len();
                let advanced_idx = tabs_len + slash_len + 1;

                if app.nav.selected_index < tabs_len {
                    messages.push(AppMessage::MoveDown);
                } else if app.nav.selected_index < advanced_idx {
                    messages.push(AppMessage::MoveDown);
                } else {
                    messages.push(AppMessage::MoveToTop);
                }
            } else {
                messages.push(AppMessage::NextTab);
            }
        }
        ShortcutAction::PrevTab => messages.push(AppMessage::PrevTab),
        ShortcutAction::SetTab(tab) => messages.push(AppMessage::SetTab(tab)),
        ShortcutAction::Undo => messages.push(AppMessage::Undo),
        ShortcutAction::Redo => messages.push(AppMessage::Redo),
        ShortcutAction::CyclePreviewMode => messages.push(AppMessage::CyclePreviewMode),
        ShortcutAction::SelectProject => messages.push(AppMessage::SelectProject),
        ShortcutAction::MoveDown => messages.push(AppMessage::MoveDown),
        ShortcutAction::MoveUp => messages.push(AppMessage::MoveUp),
        ShortcutAction::MoveToTop => messages.push(AppMessage::MoveToTop),
        ShortcutAction::MoveToBottom => messages.push(AppMessage::MoveToBottom),
        ShortcutAction::StageSelected => messages.push(AppMessage::StageSelected),
        ShortcutAction::ArchiveSelected => messages.push(AppMessage::ArchiveSelected),
        ShortcutAction::DuplicateSelected => messages.push(AppMessage::DuplicateSelected),
        ShortcutAction::RestoreSelected => messages.push(AppMessage::RestoreSelected),
        ShortcutAction::Add => {
            if app.nav.active_tab == Tab::Settings {
                messages.push(AppMessage::EditSetting);
            } else {
                messages.push(AppMessage::EnterEditorBefore(String::new(), app.nav.selected_index + 1));
            }
        }
        ShortcutAction::AddBefore => {
            if app.nav.active_tab != Tab::Settings {
                messages.push(AppMessage::EnterEditorBefore(String::new(), app.nav.selected_index));
            }
        }
        ShortcutAction::AddAtTop => {
            if app.nav.active_tab != Tab::Settings {
                messages.push(AppMessage::EnterEditorBefore(String::new(), 0));
            }
        }
        ShortcutAction::AddAtBottom => {
            if app.nav.active_tab != Tab::Settings {
                messages.push(AppMessage::EnterEditorBefore(String::new(), app.nav.prompts.len()));
            }
        }
        ShortcutAction::EditSelected => {
            if app.nav.active_tab == Tab::Settings {
                let tabs_len = Tab::settings_display_len();
                let slash_len = app.settings.slash_commands.len();
                let advanced_idx = tabs_len + slash_len + 1;
                if app.nav.selected_index == advanced_idx + 2 {
                    messages.push(AppMessage::SelectTheme);
                } else if app.nav.selected_index == advanced_idx + 4 {
                    messages.push(AppMessage::SelectStartupProject);
                } else if app.nav.selected_index < tabs_len 
                    || app.nav.selected_index == advanced_idx 
                    || app.nav.selected_index == advanced_idx + 1 
                    || app.nav.selected_index == advanced_idx + 3 
                {
                    messages.push(AppMessage::ToggleSetting);
                } else {
                    messages.push(AppMessage::EditSetting);
                }
            } else if !app.nav.prompts.is_empty() {
                let p = &app.nav.prompts[app.nav.selected_index];
                messages.push(AppMessage::EnterEditor(p.text.clone(), Some(p.id)));
            }
        }
        ShortcutAction::ToggleBranchFilter => messages.push(AppMessage::ToggleBranchFilter),
        ShortcutAction::ToggleFolderFilter => messages.push(AppMessage::ToggleFolderFilter),
        ShortcutAction::ToggleProjectFilter => messages.push(AppMessage::ToggleProjectFilter),
        ShortcutAction::Search => messages.push(AppMessage::Search(String::new())),
        ShortcutAction::ToggleMoveMode => messages.push(AppMessage::ToggleMoveMode),
        ShortcutAction::ToggleSetting => messages.push(AppMessage::ToggleSetting),
        ShortcutAction::CopySelected => messages.push(AppMessage::CopySelected),
        ShortcutAction::Save => messages.push(AppMessage::SaveEditor),
        ShortcutAction::SaveAndStage => messages.push(AppMessage::SaveAndStageEditor),
        ShortcutAction::CloseAutocomplete => messages.push(AppMessage::CloseAutocomplete),
        ShortcutAction::ConfirmDiscard => messages.push(AppMessage::ExitEditor),
        ShortcutAction::ExitEditor => {
            if app.editor.is_dirty() {
                messages.push(AppMessage::ConfirmDiscard);
            } else {
                messages.push(AppMessage::ExitEditor);
            }
        }
        ShortcutAction::CancelDiscard => messages.push(AppMessage::CancelDiscard),
        ShortcutAction::MoveSuggestionUp => messages.push(AppMessage::MoveSuggestionUp),
        ShortcutAction::MoveSuggestionDown => messages.push(AppMessage::MoveSuggestionDown),
        ShortcutAction::SelectSuggestion => messages.push(AppMessage::SelectSuggestion),
        ShortcutAction::MoveItemDown => messages.push(AppMessage::MoveItemDown),
        ShortcutAction::MoveItemUp => messages.push(AppMessage::MoveItemUp),
        _ => {}
    }
    messages
}

fn handle_add_project_events(app: &mut App<'_>, key: KeyEvent) -> Vec<AppMessage> {
    let mut messages = Vec::new();
    match key.code {
        KeyCode::Esc => { messages.push(AppMessage::SelectProject); }
        KeyCode::Enter => {
            let name = app.nav.projects_manager.new_project_name.clone();
            messages.push(AppMessage::AddProject(name));
        }
        KeyCode::Backspace => { app.nav.projects_manager.new_project_name.pop(); }
        KeyCode::Char(c) => { app.nav.projects_manager.new_project_name.push(c); }
        _ => {}
    }
    messages
}
