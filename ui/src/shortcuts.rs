use contracts::Tab;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::types::Mode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutAction {
    Quit,
    NextTab,
    PrevTab,
    SetTab(Tab),
    Undo,
    Redo,
    CyclePreviewMode,
    SelectProject,
    MoveDown,
    MoveUp,
    MoveToTop,
    MoveToBottom,
    StageSelected,
    ArchiveSelected,
    DuplicateSelected,
    RestoreSelected,
    Add,
    AddBefore,
    AddAtTop,
    AddAtBottom,
    EditSelected,
    ToggleBranchFilter,
    ToggleFolderFilter,
    ToggleProjectFilter,
    Search,
    ToggleMoveMode,
    ToggleSetting,
    CopySelected,
    SelectTheme,
    SelectStartupProject,

    // Editor actions
    Save,
    SaveAndStage,
    CloseAutocomplete,
    ConfirmDiscard,
    ExitEditor,
    CancelDiscard,
    MoveSuggestionUp,
    MoveSuggestionDown,
    SelectSuggestion,

    // Move actions
    MoveItemDown,
    MoveItemUp,
}

#[derive(Debug)]
pub struct Shortcut {
    pub key: &'static str,
    pub desc: &'static str,
}

impl Shortcut {
    pub const fn new(key: &'static str, desc: &'static str) -> Self {
        Self { key, desc }
    }
}

pub fn get_shortcuts(mode: &str, tab_name: &str, has_suggestions: bool) -> Vec<Shortcut> {
    match mode {
        "List" => {
            let mut shortcuts = vec![
                Shortcut::new("q", "Quit"),
                Shortcut::new("1-6/Tab/hl", "Tabs"),
                Shortcut::new("j/k/g/G", "Nav"),
            ];
            
            if tab_name == "Archive" {
                shortcuts.push(Shortcut::new("r", "Restore"));
            }

            if tab_name != "Notes" && tab_name != "Snippets" {
                shortcuts.push(Shortcut::new("s", "Stage"));
            }

            shortcuts.extend(vec![
                Shortcut::new("a/i", "Add"),
                Shortcut::new("e/Ent", "Edit"),
                Shortcut::new("d/D", "Del/Dupe"),
                Shortcut::new("m", "Move"),
                Shortcut::new("u", "Undo"),
                Shortcut::new("Ctrl+y", "Redo"),
                Shortcut::new("/", "Search"),
                Shortcut::new("Ctrl+e", "Preview"),
                Shortcut::new("y/c", "Copy"),
                Shortcut::new("b", "Branch"),
                Shortcut::new("f", "Folder"),
                Shortcut::new("p", "Project"),
                Shortcut::new("Ctrl+p", "Pick Prj"),
            ]);
            if tab_name == "Settings" {
                shortcuts.push(Shortcut::new("Space", "Toggle"));
            }
            shortcuts
        }
        "Move" => vec![
            Shortcut::new("j/k", "Move"),
            Shortcut::new("Esc/m/Ent", "Back"),
        ],
        "Editor" => {
            if has_suggestions {
                vec![
                    Shortcut::new("Up/Down", "Select"),
                    Shortcut::new("Enter", "Complete"),
                    Shortcut::new("Esc", "Close"),
                ]
            } else {
                let mut shortcuts = vec![
                    Shortcut::new("Ctrl+s", "Save"),
                ];
                if tab_name != "Notes" && tab_name != "Snippets" {
                    shortcuts.push(Shortcut::new("Ctrl+g", "Save & Stage"));
                }
                shortcuts.push(Shortcut::new("Esc", "Cancel"));
                shortcuts
            }
        }
        "Search" => vec![
            Shortcut::new("Enter", "Confirm"),
            Shortcut::new("Esc", "Cancel"),
        ],
        "Confirm Discard" => vec![
            Shortcut::new("y", "Discard"),
            Shortcut::new("n", "Cancel"),
        ],
        _ => vec![],
    }
}

pub fn get_action(key: KeyEvent, mode: Mode, active_tab: Tab, autocomplete_open: bool) -> Option<ShortcutAction> {
    match mode {
        Mode::List => get_list_action(key, active_tab),
        Mode::Editor => get_editor_action(key, autocomplete_open),
        Mode::Move => get_move_action(key),
        Mode::ConfirmDiscard => get_confirm_discard_action(key),
        _ => None,
    }
}

fn get_list_action(key: KeyEvent, active_tab: Tab) -> Option<ShortcutAction> {
    match key.code {
        KeyCode::Char('q') => Some(ShortcutAction::Quit),
        KeyCode::Right | KeyCode::Char('l') => Some(ShortcutAction::NextTab),
        KeyCode::Left | KeyCode::Char('h') => Some(ShortcutAction::PrevTab),
        KeyCode::Tab => Some(ShortcutAction::NextTab), // For simplicity, though Settings has special logic
        KeyCode::Char('1') => Some(ShortcutAction::SetTab(Tab::Prompts)),
        KeyCode::Char('2') => Some(ShortcutAction::SetTab(Tab::Canned)),
        KeyCode::Char('3') => Some(ShortcutAction::SetTab(Tab::Notes)),
        KeyCode::Char('4') => Some(ShortcutAction::SetTab(Tab::Snippets)),
        KeyCode::Char('5') => Some(ShortcutAction::SetTab(Tab::Archive)),
        KeyCode::Char('6') => Some(ShortcutAction::SetTab(Tab::Settings)),
        KeyCode::Char('u') => Some(ShortcutAction::Undo),
        KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(ShortcutAction::Redo),
        KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(ShortcutAction::CyclePreviewMode),
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(ShortcutAction::SelectProject),
        KeyCode::Char('j') | KeyCode::Down => Some(ShortcutAction::MoveDown),
        KeyCode::Char('k') | KeyCode::Up => Some(ShortcutAction::MoveUp),
        KeyCode::Char('s') => Some(ShortcutAction::StageSelected),
        KeyCode::Char('d') => Some(ShortcutAction::ArchiveSelected),
        KeyCode::Char('D') => Some(ShortcutAction::DuplicateSelected),
        KeyCode::Char('r') => Some(ShortcutAction::RestoreSelected),
        KeyCode::Char('a') => Some(ShortcutAction::Add),
        KeyCode::Char('i') => Some(ShortcutAction::AddBefore),
        KeyCode::Char('I') => Some(ShortcutAction::AddAtTop),
        KeyCode::Char('A') => Some(ShortcutAction::AddAtBottom),
        KeyCode::Char('b') => Some(ShortcutAction::ToggleBranchFilter),
        KeyCode::Char('f') => Some(ShortcutAction::ToggleFolderFilter),
        KeyCode::Char('p') => Some(ShortcutAction::ToggleProjectFilter),
        KeyCode::Char('/') => Some(ShortcutAction::Search),
        KeyCode::Char('G') => Some(ShortcutAction::MoveToBottom),
        KeyCode::Char('g') => Some(ShortcutAction::MoveToTop),
        KeyCode::Char('e') | KeyCode::Enter => Some(ShortcutAction::EditSelected),
        KeyCode::Char('m') => Some(ShortcutAction::ToggleMoveMode),
        KeyCode::Char(' ') if active_tab == Tab::Settings => Some(ShortcutAction::ToggleSetting),
        KeyCode::Char('y' | 'c') => Some(ShortcutAction::CopySelected),
        _ => None,
    }
}

const fn get_editor_action(key: KeyEvent, autocomplete_open: bool) -> Option<ShortcutAction> {
    match key.code {
        KeyCode::Esc => {
            if autocomplete_open {
                Some(ShortcutAction::CloseAutocomplete)
            } else {
                Some(ShortcutAction::ExitEditor) // Will be transformed to ConfirmDiscard if dirty in handlers
            }
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(ShortcutAction::Save),
        KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(ShortcutAction::SaveAndStage),
        KeyCode::Up if autocomplete_open => Some(ShortcutAction::MoveSuggestionUp),
        KeyCode::Down if autocomplete_open => Some(ShortcutAction::MoveSuggestionDown),
        KeyCode::Enter if autocomplete_open => Some(ShortcutAction::SelectSuggestion),
        _ => None,
    }
}

const fn get_move_action(key: KeyEvent) -> Option<ShortcutAction> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('m') | KeyCode::Enter => Some(ShortcutAction::ToggleMoveMode),
        KeyCode::Char('j') | KeyCode::Down => Some(ShortcutAction::MoveItemDown),
        KeyCode::Char('k') | KeyCode::Up => Some(ShortcutAction::MoveItemUp),
        _ => None,
    }
}

const fn get_confirm_discard_action(key: KeyEvent) -> Option<ShortcutAction> {
    match key.code {
        KeyCode::Char('y' | 'Y') | KeyCode::Enter => Some(ShortcutAction::ConfirmDiscard),
        KeyCode::Char('n' | 'N') | KeyCode::Esc => Some(ShortcutAction::CancelDiscard),
        _ => None,
    }
}
