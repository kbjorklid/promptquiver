use contracts::{Clipboard, Git, Storage, Tab};
use ratatui_toaster::{ToastBuilder, ToastEngine, ToastMessage, ToastPosition, ToastType};
use std::sync::Arc;
pub use ui::{AppMessage, EditorModule, ListModule, Mode, UpdateContext};

#[cfg(feature = "ai")]
fn data_dir() -> std::path::PathBuf {
    directories::ProjectDirs::from("", "", "promptquiver")
        .map_or_else(|| std::path::PathBuf::from("."), |d| d.data_dir().to_path_buf())
}

use std::fmt;

pub struct App<'a> {
    pub storage: Arc<dyn Storage>,
    pub clipboard: Arc<dyn Clipboard>,
    pub git: Arc<dyn Git>,
    pub service: Arc<dyn contracts::AppService>,
    pub should_quit: bool,
    pub mode: Mode,
    pub nav: ListModule,
    pub editor: EditorModule<'a>,
    pub current_branch: Option<String>,
    pub toaster: Option<ToastEngine<ToastMessage>>,
    pub settings: contracts::Settings,
    pub last_notification_time: Option<std::time::Instant>,
    pub file_search_tx: Option<tokio::sync::mpsc::Sender<(String, String)>>,
    pub show_help: bool,
    pub help_scroll: u16,
    pub claude_commands: Vec<contracts::Prompt>,
    pub ai_title_tx: Option<tokio::sync::mpsc::Sender<(uuid::Uuid, String)>>,
    pub ai_pending_titles: std::collections::HashSet<uuid::Uuid>,
    pub ai_download_progress: Option<f32>,
    pub ai_progress_tx: Option<tokio::sync::mpsc::Sender<AppMessage>>,
}

impl fmt::Debug for App<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App")
            .field("should_quit", &self.should_quit)
            .field("active_tab", &self.nav.active_tab)
            .field("prompts_count", &self.nav.prompts.len())
            .field("selected_index", &self.nav.selected_index)
            .field("mode", &self.mode)
            .field("current_branch", &self.current_branch)
            .field("autocomplete_open", &self.editor.autocomplete.open)
            .finish_non_exhaustive()
    }
}

impl App<'_> {
    /// Handles application messages and orchestrates state transitions.
    ///
    /// # Errors
    /// Returns an error if any message handler fails.
    pub async fn handle_message(&mut self, initial_msg: AppMessage) -> contracts::Result<()> {
        let mut msg = initial_msg;
        loop {
            self.apply_pre_transitions(&msg);

            let mut ctx = UpdateContext {
                storage: &self.storage,
                clipboard: &self.clipboard,
                git: &self.git,
                service: &self.service,
                settings: &mut self.settings,
                active_tab: self.nav.active_tab,
                selected_index: self.nav.selected_index,
                claude_commands: &self.claude_commands,
            };

            let next_msg = match self.mode {
                Mode::Editor | Mode::ConfirmDiscard => {
                    self.editor
                        .update(
                            msg.clone(),
                            &mut ctx,
                            self.nav.current_path.clone(),
                            &self.file_search_tx,
                        )
                        .await?
                }
                Mode::ExportDialog
                | Mode::ImportDialog
                | Mode::List
                | Mode::Move
                | Mode::Search
                | Mode::ThemePicker
                | Mode::ProjectPicker
                | Mode::AddProject
                | Mode::RenameProject => self.nav.update(msg.clone(), &mut ctx).await?,
            };

            if let Some(m) = next_msg {
                msg = m;
                continue;
            }

            if let Some(final_msg) = self.apply_global_action(msg).await? {
                msg = final_msg;
                continue;
            }
            break;
        }

        Ok(())
    }

    fn apply_pre_transitions(&mut self, msg: &AppMessage) {
        match msg {
            AppMessage::ThemePickerInput(ref key)
                if key.code == crossterm::event::KeyCode::Esc
                    || key.code == crossterm::event::KeyCode::Enter
                    || key.code == crossterm::event::KeyCode::Char(' ') =>
            {
                self.mode = Mode::List;
            }
            AppMessage::ProjectPickerInput(ref key)
                if key.code == crossterm::event::KeyCode::Esc =>
            {
                self.mode = Mode::List;
            }
            AppMessage::SetProject(_)
            | AppMessage::AddProject(_)
            | AppMessage::RenameProject(_, _)
            | AppMessage::DeleteProject(_)
            | AppMessage::ExportData(_, _)
            | AppMessage::ImportData(_) => {
                self.mode = Mode::List;
            }
            AppMessage::ExportDialogInput(ref key)
                if key.code == crossterm::event::KeyCode::Esc =>
            {
                self.mode = Mode::List;
            }
            AppMessage::ImportDialogInput(ref key)
                if key.code == crossterm::event::KeyCode::Esc =>
            {
                self.mode = Mode::List;
            }
            _ => {}
        }
    }

    fn start_model_download(&mut self) {
        self.ai_download_progress = Some(0.0);
        #[cfg(feature = "ai")]
        if let Some(tx) = self.ai_progress_tx.clone() {
            let settings = self.settings.clone();
            let dir = data_dir();
            tokio::spawn(async move {
                let downloader = infra::ModelDownloader::new(dir);
                let mid = infra::ai::model_id(settings.ai_model_tier);
                let token = settings.hf_token.as_deref().map(str::to_string);
                let (prog_tx, mut prog_rx) =
                    tokio::sync::mpsc::channel::<infra::ai::download::DownloadProgress>(20);
                let dl_fut = downloader.download(mid, token.as_deref(), prog_tx);
                tokio::pin!(dl_fut);
                loop {
                    tokio::select! {
                        result = &mut dl_fut => {
                            match result {
                                Ok(()) => { let _ = tx.send(AppMessage::AiDownloadProgress(1.0)).await; }
                                Err(e) => { let _ = tx.send(AppMessage::Notify(format!("Download failed: {e}"), ratatui_toaster::ToastType::Error)).await; }
                            }
                            break;
                        }
                        Some(p) = prog_rx.recv() => {
                            let _ = tx.send(AppMessage::AiDownloadProgress(p.fraction())).await;
                        }
                    }
                }
            });
        }
        #[cfg(not(feature = "ai"))]
        self.notify("Rebuild with --features ai to enable model download", ToastType::Error);
    }

    async fn apply_global_action(
        &mut self,
        msg: AppMessage,
    ) -> contracts::Result<Option<AppMessage>> {
        match msg {
            AppMessage::Quit => self.quit(),
            AppMessage::Notify(m, kind) => self.notify(m, kind),
            AppMessage::EnterEditor(text, id) => self.enter_editor(text, id),
            AppMessage::EnterEditorBefore(text, index) => self.enter_editor_before(text, index),
            AppMessage::ExitEditor => self.exit_editor(),
            AppMessage::CopySelected => self.copy_selected().await?,
            AppMessage::SaveEditor => self.save_editor().await?,
            AppMessage::SaveAndStageEditor => {
                self.save_editor().await?;
                return Ok(Some(AppMessage::StageSelected));
            }
            AppMessage::EditSetting => self.edit_setting(),
            AppMessage::ConfirmDiscard => self.mode = Mode::ConfirmDiscard,
            AppMessage::CancelDiscard => self.mode = Mode::Editor,
            AppMessage::ToggleMoveMode => {
                self.mode = if self.mode == Mode::Move { Mode::List } else { Mode::Move };
            }
            AppMessage::Search(_) => self.mode = Mode::Search,
            AppMessage::SearchInput(key)
                if key.code == crossterm::event::KeyCode::Esc
                    || key.code == crossterm::event::KeyCode::Enter =>
            {
                self.mode = Mode::List;
            }
            AppMessage::SelectTheme => self.mode = Mode::ThemePicker,
            AppMessage::ReloadPrompts => {
                self.load_prompts().await?;
            }
            AppMessage::SelectProject => {
                self.mode = Mode::ProjectPicker;
            }
            AppMessage::EnterAddProject => {
                self.mode = Mode::AddProject;
                self.nav.projects_manager.new_project_name.clear();
            }
            AppMessage::EnterRenameProject(_) => {
                self.mode = Mode::RenameProject;
            }
            AppMessage::EnterExport => {
                self.mode = Mode::ExportDialog;
            }
            AppMessage::EnterImport => {
                self.mode = Mode::ImportDialog;
            }
            AppMessage::ToggleHelp => {
                self.show_help = !self.show_help;
                if !self.show_help {
                    self.help_scroll = 0;
                }
            }
            AppMessage::ScrollHelpUp => {
                self.help_scroll = self.help_scroll.saturating_sub(1);
            }
            AppMessage::ScrollHelpDown => {
                self.help_scroll = self.help_scroll.saturating_add(1);
            }
            AppMessage::TitleGenerated(id, title) => {
                self.ai_pending_titles.remove(&id);
                let all = self.storage.get_prompts(contracts::PromptFilter::default()).await?;
                if let Some(mut prompt) = all.into_iter().find(|p| p.id == id) {
                    prompt.name = Some(title);
                    self.storage.save_prompt(prompt).await?;
                    self.load_prompts().await?;
                    self.notify("Title generated", ToastType::Info);
                }
            }
            AppMessage::AiDownloadProgress(pct) => {
                if pct >= 1.0 {
                    self.ai_download_progress = None;
                    self.notify("Model ready — AI features enabled", ToastType::Success);
                } else {
                    self.ai_download_progress = Some(pct);
                }
            }
            AppMessage::RequestModelDownload => self.start_model_download(),
            _ => {}
        }
        Ok(None)
    }

    #[must_use]
    pub fn new(
        storage: Arc<dyn Storage>,
        clipboard: Arc<dyn Clipboard>,
        git: Arc<dyn Git>,
        service: Arc<dyn contracts::AppService>,
    ) -> Self {
        Self {
            storage,
            clipboard,
            git,
            service,
            should_quit: false,
            mode: Mode::List,
            nav: ListModule::default(),
            editor: EditorModule::default(),
            current_branch: None,
            toaster: None,
            settings: contracts::Settings::default(),
            last_notification_time: None,
            file_search_tx: None,
            show_help: false,
            help_scroll: 0,
            claude_commands: Vec::new(),
            ai_title_tx: None,
            ai_pending_titles: std::collections::HashSet::new(),
            ai_download_progress: None,
            ai_progress_tx: None,
        }
    }

    pub fn tick(&mut self) {
        if let Some(last_time) = self.last_notification_time {
            if last_time.elapsed() >= std::time::Duration::from_secs(3) {
                if let Some(ref mut toaster) = self.toaster {
                    toaster.hide_toast();
                }
                self.last_notification_time = None;
            }
        }
    }

    pub fn notify(&mut self, message: impl Into<String>, kind: ToastType) {
        if let Some(ref mut toaster) = self.toaster {
            let toast = ToastBuilder::new(message.into().into())
                .toast_type(kind)
                .position(ToastPosition::BottomRight);
            toaster.show_toast(toast);
            self.last_notification_time = Some(std::time::Instant::now());
        }
    }

    pub const fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Loads prompts from storage and updates settings.
    ///
    /// # Errors
    /// Returns an error if storage cannot be accessed.
    pub async fn load_prompts(&mut self) -> contracts::Result<()> {
        let old_theme = self.settings.theme_name.clone();
        self.settings = self.storage.get_settings().await.unwrap_or_default();
        if self.mode == Mode::ThemePicker {
            self.settings.theme_name = old_theme;
        }
        self.nav.current_branch.clone_from(&self.current_branch);
        self.nav.load_prompts(&self.storage).await
    }

    /// Initializes the project state and loads settings.
    ///
    /// # Errors
    /// Returns an error if storage cannot be accessed.
    pub async fn init_project(&mut self) -> contracts::Result<()> {
        self.settings = self.storage.get_settings().await.unwrap_or_default();
        let projects = self.storage.get_projects().await?;
        self.nav.projects_manager.projects.clone_from(&projects);

        match self.settings.startup_behavior {
            contracts::StartupBehavior::Ask => {
                if projects.is_empty() {
                    self.nav.projects_manager.active_project_id = None;
                    self.nav.project_filter = true;
                } else {
                    self.mode = Mode::ProjectPicker;
                    // Pre-select last active project
                    let pos = self.settings.last_active_project_id.map_or(0, |id| {
                        projects.iter().position(|p| p.id == id).map_or(0, |p| p + 1)
                    });
                    self.nav.projects_manager.project_list_state.select(Some(pos));
                }
            }
            contracts::StartupBehavior::LastActivated => {
                self.nav.projects_manager.active_project_id = self.settings.last_active_project_id;
                self.nav.project_filter = true;
            }
            contracts::StartupBehavior::Specific => {
                if let Some(id) = self.settings.specific_project_id {
                    if projects.iter().any(|p| p.id == id) {
                        self.nav.projects_manager.active_project_id = Some(id);
                    } else {
                        self.nav.projects_manager.active_project_id = None;
                    }
                } else {
                    self.nav.projects_manager.active_project_id = None;
                }
                self.nav.project_filter = true;
            }
        }
        self.nav.load_prompts(&self.storage).await
    }

    // Wrappers for tests
    pub fn next_tab(&mut self) {
        self.nav.next_tab(&self.settings);
    }
    pub fn prev_tab(&mut self) {
        self.nav.prev_tab(&self.settings);
    }
    pub const fn set_tab(&mut self, tab: Tab) {
        self.nav.set_tab(tab);
    }
    pub fn move_down(&mut self) {
        self.nav.move_down(&self.settings);
    }
    pub fn move_up(&mut self) {
        self.nav.move_up(&self.settings);
    }
    pub const fn move_to_top(&mut self) {
        self.nav.move_to_top();
    }
    pub fn move_to_bottom(&mut self) {
        self.nav.move_to_bottom(&self.settings);
    }

    /// Stages the currently selected item.
    ///
    /// # Errors
    /// Returns an error if the item cannot be staged.
    pub async fn stage_selected(&mut self) -> contracts::Result<()> {
        self.handle_message(AppMessage::StageSelected).await
    }

    /// Archives the currently selected item.
    ///
    /// # Errors
    /// Returns an error if the item cannot be archived.
    pub async fn archive_selected(&mut self) -> contracts::Result<()> {
        self.handle_message(AppMessage::ArchiveSelected).await
    }

    /// Duplicates the currently selected item.
    ///
    /// # Errors
    /// Returns an error if the item cannot be duplicated.
    pub async fn duplicate_selected(&mut self) -> contracts::Result<()> {
        self.handle_message(AppMessage::DuplicateSelected).await
    }

    /// Copies the currently selected item's text to the clipboard.
    ///
    /// # Errors
    /// Returns an error if the item cannot be copied.
    pub async fn copy_selected(&mut self) -> contracts::Result<()> {
        if self.nav.prompts.is_empty() {
            return Ok(());
        }
        let item = self.nav.prompts[self.nav.selected_index].clone();
        self.service.copy_item(&self.nav.current_project_path(), self.nav.active_tab, item).await?;
        self.load_prompts().await?;
        self.notify("Copied to clipboard!", ToastType::Success);
        Ok(())
    }

    /// Restores the currently selected item from the archive.
    ///
    /// # Errors
    /// Returns an error if the item cannot be restored.
    pub async fn restore_selected(&mut self) -> contracts::Result<()> {
        self.handle_message(AppMessage::RestoreSelected).await
    }

    /// Updates autocomplete suggestions in the editor.
    ///
    /// # Errors
    /// Returns an error if the autocomplete update fails.
    pub async fn update_autocomplete(&mut self) -> contracts::Result<()> {
        self.handle_message(AppMessage::UpdateAutocomplete).await
    }

    /// Selects the current autocomplete suggestion.
    ///
    /// # Errors
    /// Returns an error if the suggestion cannot be selected.
    pub async fn select_suggestion(&mut self, add_space: bool) -> contracts::Result<()> {
        self.handle_message(AppMessage::SelectSuggestion(add_space)).await
    }

    pub fn enter_editor(&mut self, text: String, id: Option<uuid::Uuid>) {
        self.nav.search_query.clear();
        self.mode = Mode::Editor;

        let title = if self.nav.active_tab == Tab::Snippets {
            if let Some(id) = id {
                self.nav
                    .prompts
                    .iter()
                    .find(|p| p.id == id)
                    .and_then(|p| p.name.clone())
                    .unwrap_or_default()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        self.editor.enter(text, id, Some(title), self.nav.active_tab, None);
    }

    pub fn enter_editor_before(&mut self, text: String, index: usize) {
        self.nav.search_query.clear();
        self.mode = Mode::Editor;
        self.editor.enter(text, None, Some(String::new()), self.nav.active_tab, Some(index));
    }

    pub fn exit_editor(&mut self) {
        self.mode = Mode::List;
        self.editor.exit();
    }

    pub fn edit_setting(&mut self) {
        if self.nav.active_tab != Tab::Settings {
            return;
        }
        let tabs_len = Tab::settings_display_len();
        let slash_len = self.settings.slash_commands.len();

        if self.nav.selected_index >= tabs_len && self.nav.selected_index < tabs_len + slash_len {
            // Edit existing Slash Command
            let idx = self.nav.selected_index - tabs_len;
            let text = self.settings.slash_commands[idx].clone();
            self.mode = Mode::Editor;
            self.editor.enter(text, None, None, self.nav.active_tab, None);
        } else if self.nav.selected_index == tabs_len + slash_len {
            // Add New Slash Command
            self.mode = Mode::Editor;
            self.editor.enter(String::new(), None, None, self.nav.active_tab, None);
        }
    }

    /// Saves the current editor content.
    ///
    /// # Errors
    /// Returns an error if the item cannot be saved.
    ///
    /// # Panics
    /// Panics if the title validation regex fails to compile.
    pub async fn save_editor(&mut self) -> contracts::Result<()> {
        let text = self.editor.textarea.lines().join("\n");
        let path = self.nav.current_project_path();

        if self.nav.active_tab == Tab::Settings {
            let tabs_len = Tab::settings_display_len();
            let slash_len = self.settings.slash_commands.len();

            let re = regex::Regex::new("^[a-zA-Z0-9_:-]+$").unwrap();
            let mut trimmed = text.trim();
            if trimmed.starts_with('/') {
                trimmed = &trimmed[1..];
            }
            if !trimmed.is_empty() && !re.is_match(trimmed) {
                self.notify("Slash command must match [a-zA-Z0-9_:-]+", ToastType::Error);
                return Ok(());
            }

            if self.nav.selected_index >= tabs_len && self.nav.selected_index < tabs_len + slash_len
            {
                // Update existing
                let idx = self.nav.selected_index - tabs_len;
                self.settings.slash_commands[idx] = trimmed.to_string();
                self.storage.save_settings(self.settings.clone()).await?;
            } else if self.nav.selected_index == tabs_len + slash_len {
                // Add new
                let new_cmd = trimmed.to_string();
                if !new_cmd.is_empty() {
                    self.settings.slash_commands.push(new_cmd);
                    self.storage.save_settings(self.settings.clone()).await?;
                }
            }
            self.exit_editor();
            self.notify("Settings saved!", ToastType::Success);
            return Ok(());
        }

        let title = if self.nav.active_tab == Tab::Snippets {
            let t = self.editor.title_textarea.lines().join("");
            let re = regex::Regex::new("^[a-zA-Z0-9_-]+$").unwrap();
            if !re.is_match(&t) {
                self.notify("Snippet name must match [a-zA-Z0-9_-]+", ToastType::Error);
                return Ok(());
            }
            Some(t)
        } else {
            contracts::Processor::extract_title(&text).0
        };

        self.nav.push_history();

        let branch = self.git.get_current_branch(&path).await.unwrap_or_default();
        let project_id = self.nav.projects_manager.active_project_id;
        let text_for_ai = text.clone();
        let title_is_empty = title.is_none();

        let res = self
            .service
            .save_item(contracts::SaveItemArgs {
                project_path: path,
                tab: self.nav.active_tab,
                text,
                title,
                id: self.editor.editing_id,
                insert_index: self.editor.insert_index,
                branch,
                project_id,
            })
            .await;

        match res {
            Ok(saved_id) => {
                self.exit_editor();
                self.load_prompts().await?;

                // Select the saved item
                if let Some(index) = self.nav.prompts.iter().position(|p| p.id == saved_id) {
                    self.nav.selected_index = index;
                }

                // Queue AI title generation for untitled prompts
                if self.settings.ai_enabled
                    && self.settings.ai_auto_title
                    && title_is_empty
                    && self.nav.active_tab != Tab::Snippets
                {
                    if let Some(tx) = &self.ai_title_tx {
                        if tx.try_send((saved_id, text_for_ai)).is_ok() {
                            self.ai_pending_titles.insert(saved_id);
                        }
                    }
                }

                self.notify("Prompt saved!", ToastType::Success);
            }
            Err(contracts::Error::Conflict(m)) => {
                self.notify(format!("Conflict: {m}. Changes NOT saved."), ToastType::Error);
                // We stay in editor so user can copy their work or try again after reload
            }
            Err(e) => {
                self.notify(format!("Error: {e}"), ToastType::Error);
            }
        }
        Ok(())
    }

    /// Saves the current editor content and then stages it.
    ///
    /// # Errors
    /// Returns an error if the item cannot be saved or staged.
    pub async fn save_and_stage_editor(&mut self) -> contracts::Result<()> {
        self.save_editor().await?;
        self.handle_message(AppMessage::StageSelected).await
    }
}
