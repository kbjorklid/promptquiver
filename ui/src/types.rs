use contracts::{Tab};
use uuid::Uuid;
use std::sync::Arc;
use contracts::{Storage, Clipboard, Git, AppService, Settings};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    List,
    Editor,
    Move,
    Search,
    GlobalSearch,
    ConfirmDiscard,
    ThemePicker,
}

#[derive(Debug, Clone)]
pub enum AppMessage {
    Quit,
    NextTab,
    PrevTab,
    SetTab(Tab),
    Undo,
    Redo,
    MoveDown,
    MoveUp,
    MoveToTop,
    MoveToBottom,
    MoveItemUp,
    MoveItemDown,
    StageSelected,
    ArchiveSelected,
    DuplicateSelected,
    RestoreSelected,
    EnterEditor(String, Option<Uuid>),
    EnterEditorBefore(String, usize),
    ExitEditor,
    SaveEditor,
    SaveAndStageEditor,
    UpdateAutocomplete,
    MoveSuggestionDown,
    MoveSuggestionUp,
    SelectSuggestion,
    ToggleSetting,
    ToggleBranchFilter,
    Search(String),
    GlobalSearch(String),
    Notify(String, ratatui_toaster::ToastType),
    EditSetting,
    ConfirmDiscard,
    CancelDiscard,
    EditorInput(crossterm::event::KeyEvent),
    SearchInput(crossterm::event::KeyEvent),
    GlobalSearchInput(crossterm::event::KeyEvent),
    ToggleMoveMode,
    ThemePickerInput(crossterm::event::KeyEvent),
    SetTheme(Option<String>),
    SelectTheme,
}

pub struct UpdateContext<'a> {
    pub storage: &'a Arc<dyn Storage>,
    pub clipboard: &'a Arc<dyn Clipboard>,
    pub git: &'a Arc<dyn Git>,
    pub service: &'a Arc<dyn AppService>,
    pub settings: &'a mut Settings,
    pub active_tab: Tab,
    pub selected_index: usize,
}
