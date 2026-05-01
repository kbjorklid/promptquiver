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
    CopySelected,
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
    CloseAutocomplete,
    MoveSuggestionDown,
    MoveSuggestionUp,
    SelectSuggestion,
    ToggleSetting,
    ToggleBranchFilter,
    ToggleFolderFilter,
    Search(String),
    Notify(String, ratatui_toaster::ToastType),
    EditSetting,
    ConfirmDiscard,
    CancelDiscard,
    EditorInput(crossterm::event::KeyEvent),
    SearchInput(crossterm::event::KeyEvent),
    ToggleMoveMode,
    ThemePickerInput(crossterm::event::KeyEvent),
    SetTheme(Option<String>),
    SelectTheme,
    CyclePreviewMode,
    ReloadPrompts,
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

impl std::fmt::Debug for UpdateContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpdateContext")
            .field("active_tab", &self.active_tab)
            .field("selected_index", &self.selected_index)
            .field("settings", &self.settings)
            .finish_non_exhaustive()
    }
}
