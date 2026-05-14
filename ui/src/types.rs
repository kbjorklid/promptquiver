use contracts::Tab;
use contracts::{AppService, Clipboard, Git, Settings, Storage};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    List,
    Editor,
    Move,
    Search,
    ConfirmDiscard,
    ThemePicker,
    ProjectPicker,
    AddProject,
    RenameProject,
    ExportDialog,
    ImportDialog,
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
    SelectSuggestion(bool),
    ToggleSetting,
    ToggleBranchFilter,
    ToggleFolderFilter,
    ToggleProjectFilter,
    SelectProject,
    SetProject(Option<Uuid>),
    AddProject(String),
    RenameProject(Uuid, String),
    DeleteProject(Uuid),
    EnterAddProject,
    EnterRenameProject(Uuid),
    ProjectPickerInput(crossterm::event::KeyEvent),
    RenameProjectInput(crossterm::event::KeyEvent),
    Search(String),
    Notify(String, ratatui_toaster::ToastType),
    EditSetting,
    ConfirmDiscard,
    CancelDiscard,
    EditorInput(crossterm::event::KeyEvent),
    Paste(String),
    SearchInput(crossterm::event::KeyEvent),
    ToggleMoveMode,
    ThemePickerInput(crossterm::event::KeyEvent),
    SetTheme(Option<String>),
    SelectTheme,
    CyclePreviewMode,
    ReloadPrompts,
    ToggleSettingValue,
    ToggleStartupBehavior,
    SelectStartupProject,
    ToggleHelp,
    ScrollHelpUp,
    ScrollHelpDown,
    EnterExport,
    EnterImport,
    ExportData(String, bool),
    ImportData(String),
    ExportDialogInput(crossterm::event::KeyEvent),
    ImportDialogInput(crossterm::event::KeyEvent),
    TitleGenerated(uuid::Uuid, String),
    RequestModelDownload,
    AiDownloadProgress(f32),
}

#[derive(Debug)]
pub struct RenderState<'a, 'b> {
    pub nav: &'a mut crate::list_module::ListModule,
    pub editor: &'a mut crate::editor_module::EditorModule<'b>,
    pub mode: Mode,
    pub settings: &'a Settings,
    pub current_branch: Option<&'a str>,
    pub show_help: bool,
    pub help_scroll: u16,
    pub ai_pending_titles: Option<&'a std::collections::HashSet<uuid::Uuid>>,
    pub ai_download_progress: Option<f32>,
}

pub struct UpdateContext<'a> {
    pub storage: &'a Arc<dyn Storage>,
    pub clipboard: &'a Arc<dyn Clipboard>,
    pub git: &'a Arc<dyn Git>,
    pub service: &'a Arc<dyn AppService>,
    pub settings: &'a mut Settings,
    pub active_tab: Tab,
    pub selected_index: usize,
    pub claude_commands: &'a [contracts::Prompt],
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
