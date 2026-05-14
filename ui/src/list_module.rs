use crate::history_manager::HistoryManager;
use crate::project_manager::ProjectManager;
use contracts::{Prompt, PromptFilter, Result, Storage, Tab};
use std::sync::Arc;

#[derive(Debug)]
pub struct ListModule {
    pub active_tab: Tab,
    pub prompts: Vec<Prompt>,
    pub selected_index: usize,
    pub list_state: ratatui::widgets::ListState,
    pub settings_slash_list_state: ratatui::widgets::ListState,
    pub theme_list_state: ratatui::widgets::ListState,
    pub history: HistoryManager,
    pub projects_manager: ProjectManager,
    pub branch_filter: bool,
    pub folder_filter: bool,
    pub project_filter: bool,
    pub search_query: String,
    pub current_path: String,
    pub original_theme: Option<String>,
    pub current_branch: Option<String>,
    pub settings_scroll_offset: u16,
    pub data_manager: DataManagerState,
}

#[derive(Debug, Default)]
pub struct DataManagerState {
    pub path: String,
    pub include_archived: bool,
    pub focus_checkbox: bool,
}

impl Default for ListModule {
    fn default() -> Self {
        Self {
            active_tab: Tab::Prompts,
            prompts: Vec::new(),
            selected_index: 0,
            list_state: ratatui::widgets::ListState::default().with_selected(Some(0)),
            settings_slash_list_state: ratatui::widgets::ListState::default()
                .with_selected(Some(0)),
            theme_list_state: ratatui::widgets::ListState::default().with_selected(Some(0)),
            history: HistoryManager::default(),
            projects_manager: ProjectManager::default(),
            branch_filter: false,
            folder_filter: false,
            project_filter: false,
            search_query: String::new(),
            current_path: std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .to_string_lossy()
                .into_owned(),
            original_theme: None,
            current_branch: None,
            settings_scroll_offset: 0,
            data_manager: DataManagerState::default(),
        }
    }
}

impl ListModule {
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads prompts from storage based on current filters.
    ///
    /// # Errors
    /// Returns an error if the storage cannot be accessed.
    pub async fn load_prompts(&mut self, storage: &Arc<dyn Storage>) -> Result<()> {
        let path = self.current_project_path();

        // Ensure project info is saved
        let _ =
            storage.save_project_info(&path, contracts::ProjectInfo { path: path.clone() }).await;

        let is_global_tab = self.active_tab == Tab::Canned || self.active_tab == Tab::Snippets;
        let filter = PromptFilter {
            folder: if !self.folder_filter || is_global_tab { None } else { Some(path) },
            branch: if self.branch_filter && !is_global_tab {
                self.current_branch.clone()
            } else {
                None
            },
            project_id: if self.project_filter && !is_global_tab {
                self.projects_manager.active_project_id
            } else {
                None
            },
            project_filter: self.project_filter && !is_global_tab,
            tab: Some(self.active_tab),
            ..Default::default()
        };

        let mut prompts = storage.get_prompts(filter).await?;

        if !self.search_query.is_empty() {
            let query = self.search_query.to_lowercase();
            prompts.retain(|p| {
                p.text.to_lowercase().contains(&query)
                    || p.name.as_deref().unwrap_or("").to_lowercase().contains(&query)
            });
        }

        self.prompts = prompts;

        if self.selected_index >= self.prompts.len() && !self.prompts.is_empty() {
            self.selected_index = self.prompts.len() - 1;
        }
        self.list_state.select(Some(self.selected_index));

        Ok(())
    }

    pub fn next_tab(&mut self, settings: &contracts::Settings) {
        let visible = settings.visible_tabs();
        let pos = visible.iter().position(|&t| t == self.active_tab).unwrap_or(0);
        self.active_tab = visible[(pos + 1) % visible.len()];
        self.selected_index = 0;
        self.list_state.select(Some(0));
    }

    pub fn prev_tab(&mut self, settings: &contracts::Settings) {
        let visible = settings.visible_tabs();
        let pos = visible.iter().position(|&t| t == self.active_tab).unwrap_or(0);
        self.active_tab = visible[(pos + visible.len() - 1) % visible.len()];
        self.selected_index = 0;
        self.list_state.select(Some(0));
    }

    pub const fn set_tab(&mut self, tab: Tab) {
        self.active_tab = tab;
        self.selected_index = 0;
        self.list_state.select(Some(0));
    }

    pub fn move_down(&mut self, settings: &contracts::Settings) {
        if self.active_tab == Tab::Settings {
            let total_settings = self.total_settings_count(settings);

            if self.selected_index < total_settings - 1 {
                self.selected_index += 1;
                self.list_state.select(Some(self.selected_index));

                let tabs_len = Tab::settings_display_len();
                let slash_len = settings.slash_commands.len();
                // Update slash list state
                if self.selected_index >= tabs_len && self.selected_index <= tabs_len + slash_len {
                    self.settings_slash_list_state.select(Some(self.selected_index - tabs_len));
                } else {
                    self.settings_slash_list_state.select(None);
                }
            }
        } else if !self.prompts.is_empty() && self.selected_index < self.prompts.len() - 1 {
            self.selected_index += 1;
            self.list_state.select(Some(self.selected_index));
        }
    }

    pub fn move_up(&mut self, settings: &contracts::Settings) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.list_state.select(Some(self.selected_index));

            if self.active_tab == Tab::Settings {
                let tabs_len = Tab::settings_display_len();
                let slash_len = settings.slash_commands.len();
                if self.selected_index >= tabs_len && self.selected_index <= tabs_len + slash_len {
                    self.settings_slash_list_state.select(Some(self.selected_index - tabs_len));
                } else {
                    self.settings_slash_list_state.select(None);
                }
            }
        }
    }

    pub const fn move_to_top(&mut self) {
        self.selected_index = 0;
        self.list_state.select(Some(0));
    }

    pub fn move_to_bottom(&mut self, settings: &contracts::Settings) {
        if self.active_tab == Tab::Settings {
            let total_settings = self.total_settings_count(settings);
            self.selected_index = total_settings - 1;
            self.list_state.select(Some(self.selected_index));
        } else if !self.prompts.is_empty() {
            self.selected_index = self.prompts.len() - 1;
            self.list_state.select(Some(self.selected_index));
        }
    }

    pub fn total_settings_count(&self, settings: &contracts::Settings) -> usize {
        let tabs_len = Tab::settings_display_len();
        let slash_len = settings.slash_commands.len();
        let mut count = tabs_len + slash_len + 1; // tabs + slash commands + "Add New"
        count += 2; // Maintenance: Export, Import
        count += 5; // Advanced: Claude Discovery, Claude Builtin, Nerd Font, Theme, Startup Behavior
        if settings.startup_behavior == contracts::StartupBehavior::Specific {
            count += 1; // Startup Project
        }
        count += 6; // AI: Enable, Model Tier, Auto-title, Download, HF Token, Model Path
        count
    }

    /// Saves the current list order to storage.
    ///
    /// # Errors
    /// Returns an error if the storage cannot be accessed.
    pub async fn save_current_list(&self, storage: &Arc<dyn Storage>) -> Result<()> {
        let mut prompts = self.prompts.clone();
        for (i, p) in prompts.iter_mut().enumerate() {
            p.order_index = i32::try_from(i).unwrap_or(i32::MAX);
        }
        storage.save_prompts(prompts).await?;
        Ok(())
    }

    pub fn push_history(&mut self) {
        self.history.push(self.active_tab, self.prompts.clone());
    }

    /// Handles application messages and updates state.
    ///
    /// # Errors
    /// Returns an error if storage or service operations fail.
    pub async fn update(
        &mut self,
        msg: crate::types::AppMessage,
        ctx: &mut crate::types::UpdateContext<'_>,
    ) -> Result<Option<crate::types::AppMessage>> {
        use crate::types::AppMessage;
        match msg {
            AppMessage::NextTab | AppMessage::PrevTab | AppMessage::SetTab(_) => {
                self.handle_navigation(msg, ctx).await
            }
            AppMessage::Undo | AppMessage::Redo => self.handle_history(msg, ctx).await,
            AppMessage::MoveDown
            | AppMessage::MoveUp
            | AppMessage::MoveToTop
            | AppMessage::MoveToBottom => {
                self.handle_movement(&msg, ctx);
                Ok(None)
            }
            AppMessage::MoveItemUp
            | AppMessage::MoveItemDown
            | AppMessage::StageSelected
            | AppMessage::ArchiveSelected
            | AppMessage::DuplicateSelected
            | AppMessage::RestoreSelected => self.handle_item_ops(msg, ctx).await,
            AppMessage::ToggleSetting
            | AppMessage::ToggleBranchFilter
            | AppMessage::ToggleFolderFilter
            | AppMessage::ToggleProjectFilter
            | AppMessage::CyclePreviewMode
            | AppMessage::EnterExport
            | AppMessage::EnterImport
            | AppMessage::ExportData(_, _)
            | AppMessage::ImportData(_)
            | AppMessage::ExportDialogInput(_)
            | AppMessage::ImportDialogInput(_) => self.handle_settings_ops(msg, ctx).await,
            AppMessage::SelectProject
            | AppMessage::SetProject(_)
            | AppMessage::ToggleStartupBehavior
            | AppMessage::SelectStartupProject
            | AppMessage::AddProject(_)
            | AppMessage::RenameProject(_, _)
            | AppMessage::DeleteProject(_)
            | AppMessage::ProjectPickerInput(_)
            | AppMessage::RenameProjectInput(_)
            | AppMessage::EnterRenameProject(_) => self.handle_project_ops(msg, ctx).await,
            AppMessage::Search(_) | AppMessage::Paste(_) | AppMessage::SearchInput(_) => {
                self.handle_search_ops(msg, ctx).await
            }
            AppMessage::SelectTheme | AppMessage::ThemePickerInput(_) => {
                self.handle_theme_ops(&msg, ctx).await
            }

            _ => Ok(None),
        }
    }

    async fn handle_navigation(
        &mut self,
        msg: crate::types::AppMessage,
        ctx: &crate::types::UpdateContext<'_>,
    ) -> Result<Option<crate::types::AppMessage>> {
        use crate::types::AppMessage;
        match msg {
            AppMessage::NextTab => {
                self.next_tab(ctx.settings);
                self.load_prompts(ctx.storage).await?;
            }
            AppMessage::PrevTab => {
                self.prev_tab(ctx.settings);
                self.load_prompts(ctx.storage).await?;
            }
            AppMessage::SetTab(tab) => {
                self.set_tab(tab);
                self.load_prompts(ctx.storage).await?;
            }
            _ => {}
        }
        Ok(None)
    }

    async fn handle_history(
        &mut self,
        msg: crate::types::AppMessage,
        ctx: &crate::types::UpdateContext<'_>,
    ) -> Result<Option<crate::types::AppMessage>> {
        use crate::types::AppMessage;
        match msg {
            AppMessage::Undo => {
                if let Some(entry) =
                    self.history.undo(self.active_tab, self.prompts.clone(), ctx.storage).await?
                {
                    self.active_tab = entry.tab;
                    self.prompts = entry.prompts;
                    return Ok(Some(AppMessage::Notify(
                        "Undo".into(),
                        ratatui_toaster::ToastType::Info,
                    )));
                }
            }
            AppMessage::Redo => {
                if let Some(entry) =
                    self.history.redo(self.active_tab, self.prompts.clone(), ctx.storage).await?
                {
                    self.active_tab = entry.tab;
                    self.prompts = entry.prompts;
                    return Ok(Some(AppMessage::Notify(
                        "Redo".into(),
                        ratatui_toaster::ToastType::Info,
                    )));
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn handle_movement(
        &mut self,
        msg: &crate::types::AppMessage,
        ctx: &crate::types::UpdateContext<'_>,
    ) {
        use crate::types::AppMessage;
        match msg {
            AppMessage::MoveDown => self.move_down(ctx.settings),
            AppMessage::MoveUp => self.move_up(ctx.settings),
            AppMessage::MoveToTop => self.move_to_top(),
            AppMessage::MoveToBottom => self.move_to_bottom(ctx.settings),
            _ => {}
        }
    }

    async fn handle_item_ops(
        &mut self,
        msg: crate::types::AppMessage,
        ctx: &mut crate::types::UpdateContext<'_>,
    ) -> Result<Option<crate::types::AppMessage>> {
        use crate::types::AppMessage;
        match msg {
            AppMessage::MoveItemUp if self.selected_index > 0 && !self.prompts.is_empty() => {
                self.history.push(self.active_tab, self.prompts.clone());
                self.prompts.swap(self.selected_index, self.selected_index - 1);
                self.selected_index -= 1;
                self.save_current_list(ctx.storage).await?;
            }
            AppMessage::MoveItemDown
                if !self.prompts.is_empty() && self.selected_index < self.prompts.len() - 1 =>
            {
                self.history.push(self.active_tab, self.prompts.clone());
                self.prompts.swap(self.selected_index, self.selected_index + 1);
                self.selected_index += 1;
                self.save_current_list(ctx.storage).await?;
            }
            AppMessage::StageSelected => return self.stage_selected(ctx).await,
            AppMessage::ArchiveSelected => return self.archive_selected(ctx).await,
            AppMessage::DuplicateSelected => return self.duplicate_selected(ctx).await,
            AppMessage::RestoreSelected => return self.restore_selected(ctx).await,
            _ => {}
        }
        Ok(None)
    }

    async fn stage_selected(
        &mut self,
        ctx: &crate::types::UpdateContext<'_>,
    ) -> Result<Option<crate::types::AppMessage>> {
        use crate::types::AppMessage;
        if self.active_tab == Tab::Settings || self.prompts.is_empty() {
            return Ok(None);
        }

        let item = self.prompts[self.selected_index].clone();
        let is_staged = item.staged;
        let is_alias = self.active_tab == Tab::Notes || self.active_tab == Tab::Snippets;

        self.history.push(self.active_tab, self.prompts.clone());
        ctx.service.stage_item(&self.current_project_path(), self.active_tab, item).await?;

        let notify_msg = if is_alias {
            "Copied to clipboard!"
        } else if is_staged {
            "Prompt un-staged"
        } else {
            "Prompt staged and copied to clipboard!"
        };

        let notify_type = if is_staged && !is_alias {
            ratatui_toaster::ToastType::Info
        } else {
            ratatui_toaster::ToastType::Success
        };

        self.load_prompts(ctx.storage).await?;
        Ok(Some(AppMessage::Notify(notify_msg.into(), notify_type)))
    }

    async fn archive_selected(
        &mut self,
        ctx: &mut crate::types::UpdateContext<'_>,
    ) -> Result<Option<crate::types::AppMessage>> {
        use crate::types::AppMessage;
        if self.active_tab == Tab::Settings {
            let tabs_len = Tab::settings_display_len();
            let slash_len = ctx.settings.slash_commands.len();
            if self.selected_index >= tabs_len && self.selected_index < tabs_len + slash_len {
                let idx = self.selected_index - tabs_len;
                ctx.settings.slash_commands.remove(idx);
                ctx.storage.save_settings(ctx.settings.clone()).await?;
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
                return Ok(Some(AppMessage::Notify(
                    "Slash command deleted".into(),
                    ratatui_toaster::ToastType::Warning,
                )));
            }
            return Ok(None);
        }

        if self.prompts.is_empty() {
            return Ok(None);
        }

        self.history.push(self.active_tab, self.prompts.clone());
        let target = self.prompts[self.selected_index].clone();
        ctx.service.archive_item(&self.current_project_path(), self.active_tab, target).await?;

        let (msg, toast) = if self.active_tab == Tab::Archive {
            ("Prompt deleted permanently", ratatui_toaster::ToastType::Warning)
        } else {
            ("Prompt moved to archive", ratatui_toaster::ToastType::Info)
        };

        self.load_prompts(ctx.storage).await?;
        Ok(Some(AppMessage::Notify(msg.into(), toast)))
    }

    async fn duplicate_selected(
        &mut self,
        ctx: &crate::types::UpdateContext<'_>,
    ) -> Result<Option<crate::types::AppMessage>> {
        use crate::types::AppMessage;
        if self.active_tab == Tab::Settings || self.prompts.is_empty() {
            return Ok(None);
        }

        self.history.push(self.active_tab, self.prompts.clone());
        let target = self.prompts[self.selected_index].clone();

        if let Some(new_prompt) = ctx
            .service
            .duplicate_item(&self.current_project_path(), self.active_tab, target)
            .await?
        {
            self.prompts.insert(self.selected_index + 1, new_prompt);
            self.selected_index += 1;
            self.list_state.select(Some(self.selected_index));
            return Ok(Some(AppMessage::Notify(
                "Prompt duplicated".into(),
                ratatui_toaster::ToastType::Success,
            )));
        }
        Ok(None)
    }

    async fn restore_selected(
        &mut self,
        ctx: &crate::types::UpdateContext<'_>,
    ) -> Result<Option<crate::types::AppMessage>> {
        use crate::types::AppMessage;
        if self.active_tab != Tab::Archive || self.prompts.is_empty() {
            return Ok(None);
        }

        self.history.push(self.active_tab, self.prompts.clone());
        let target = self.prompts[self.selected_index].clone();
        ctx.service.restore_item(&self.current_project_path(), target).await?;
        self.load_prompts(ctx.storage).await?;
        Ok(Some(AppMessage::Notify("Prompt restored".into(), ratatui_toaster::ToastType::Success)))
    }

    async fn handle_settings_ops(
        &mut self,
        msg: crate::types::AppMessage,
        ctx: &mut crate::types::UpdateContext<'_>,
    ) -> Result<Option<crate::types::AppMessage>> {
        use crate::types::AppMessage;
        match msg {
            AppMessage::ToggleSetting => self.toggle_setting(ctx).await,
            AppMessage::ToggleBranchFilter => {
                self.branch_filter = !self.branch_filter;
                self.load_prompts(ctx.storage).await?;
                Ok(Some(AppMessage::Notify(
                    format!("Branch filter: {}", if self.branch_filter { "ON" } else { "OFF" }),
                    ratatui_toaster::ToastType::Info,
                )))
            }
            AppMessage::ToggleFolderFilter => {
                self.folder_filter = !self.folder_filter;
                self.load_prompts(ctx.storage).await?;
                Ok(Some(AppMessage::Notify(
                    format!("Folder filter: {}", if self.folder_filter { "ON" } else { "OFF" }),
                    ratatui_toaster::ToastType::Info,
                )))
            }
            AppMessage::ToggleProjectFilter => {
                self.project_filter = !self.project_filter;
                self.load_prompts(ctx.storage).await?;
                Ok(Some(AppMessage::Notify(
                    format!("Project filter: {}", if self.project_filter { "ON" } else { "OFF" }),
                    ratatui_toaster::ToastType::Info,
                )))
            }
            AppMessage::CyclePreviewMode => {
                ctx.settings.preview_mode = match ctx.settings.preview_mode {
                    contracts::PreviewMode::Bottom => contracts::PreviewMode::Side,
                    contracts::PreviewMode::Side => contracts::PreviewMode::Hidden,
                    contracts::PreviewMode::Hidden => contracts::PreviewMode::Bottom,
                };
                ctx.storage.save_settings(ctx.settings.clone()).await?;
                Ok(Some(AppMessage::Notify(
                    format!("Preview mode: {:?}", ctx.settings.preview_mode),
                    ratatui_toaster::ToastType::Info,
                )))
            }
            AppMessage::EnterExport
            | AppMessage::EnterImport
            | AppMessage::ExportData(_, _)
            | AppMessage::ImportData(_)
            | AppMessage::ExportDialogInput(_)
            | AppMessage::ImportDialogInput(_) => self.handle_data_ops(msg, ctx).await,
            _ => Ok(None),
        }
    }

    async fn handle_data_ops(
        &mut self,
        msg: crate::types::AppMessage,
        ctx: &crate::types::UpdateContext<'_>,
    ) -> Result<Option<crate::types::AppMessage>> {
        use crate::types::AppMessage;
        match msg {
            AppMessage::EnterExport => {
                self.data_manager.path = "export.toml".to_string();
                self.data_manager.include_archived = true;
                self.data_manager.focus_checkbox = false;
            }
            AppMessage::EnterImport => {
                self.data_manager.path = "export.toml".to_string();
            }
            AppMessage::ExportData(path, include_archived) => {
                let data = ctx.service.export_data(include_archived).await?;
                std::fs::write(&path, data)
                    .map_err(|e| contracts::Error::Storage(e.to_string()))?;
                return Ok(Some(AppMessage::Notify(
                    format!("Data exported to {path}"),
                    ratatui_toaster::ToastType::Success,
                )));
            }
            AppMessage::ImportData(path) => {
                let data = std::fs::read_to_string(&path)
                    .map_err(|e| contracts::Error::Storage(e.to_string()))?;
                ctx.service.import_data(&data).await?;
                self.load_prompts(ctx.storage).await?;
                return Ok(Some(AppMessage::Notify(
                    "Data imported successfully".into(),
                    ratatui_toaster::ToastType::Success,
                )));
            }
            AppMessage::ExportDialogInput(key) => return Ok(self.handle_export_dialog_input(key)),
            AppMessage::ImportDialogInput(key) => return Ok(self.handle_import_dialog_input(key)),
            _ => {}
        }
        Ok(None)
    }

    fn handle_export_dialog_input(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Option<crate::types::AppMessage> {
        use crate::types::AppMessage;
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Esc => Some(AppMessage::SetTab(Tab::Settings)), // Exit dialog
            KeyCode::Enter => {
                if self.data_manager.focus_checkbox {
                    self.data_manager.include_archived = !self.data_manager.include_archived;
                    None
                } else {
                    Some(AppMessage::ExportData(
                        self.data_manager.path.clone(),
                        self.data_manager.include_archived,
                    ))
                }
            }
            KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
                self.data_manager.focus_checkbox = !self.data_manager.focus_checkbox;
                None
            }
            KeyCode::Char(' ') if self.data_manager.focus_checkbox => {
                self.data_manager.include_archived = !self.data_manager.include_archived;
                None
            }
            KeyCode::Backspace if !self.data_manager.focus_checkbox => {
                self.data_manager.path.pop();
                None
            }
            KeyCode::Char(c) if !self.data_manager.focus_checkbox => {
                self.data_manager.path.push(c);
                None
            }
            _ => None,
        }
    }

    fn handle_import_dialog_input(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Option<crate::types::AppMessage> {
        use crate::types::AppMessage;
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Esc => Some(AppMessage::SetTab(Tab::Settings)), // Exit dialog
            KeyCode::Enter => Some(AppMessage::ImportData(self.data_manager.path.clone())),
            KeyCode::Backspace => {
                self.data_manager.path.pop();
                None
            }
            KeyCode::Char(c) => {
                self.data_manager.path.push(c);
                None
            }
            _ => None,
        }
    }

    async fn toggle_setting(
        &self,
        ctx: &mut crate::types::UpdateContext<'_>,
    ) -> Result<Option<crate::types::AppMessage>> {
        use crate::types::AppMessage;
        if self.active_tab != Tab::Settings {
            return Ok(None);
        }

        let tabs: Vec<Tab> = Tab::all().into_iter().filter(|&t| t != Tab::Settings).collect();
        let slash_len = ctx.settings.slash_commands.len();

        if self.selected_index < tabs.len() {
            let tab = tabs[self.selected_index];
            let current = ctx.settings.tab_visibility.get(&tab).copied().unwrap_or(true);
            ctx.settings.tab_visibility.insert(tab, !current);
            ctx.storage.save_settings(ctx.settings.clone()).await?;
            return Ok(Some(AppMessage::Notify(
                format!("Toggled visibility for {tab:?}"),
                ratatui_toaster::ToastType::Info,
            )));
        }

        if self.selected_index < tabs.len() + slash_len + 1 {
            return Ok(None);
        }

        let maintenance_start = tabs.len() + slash_len + 1;
        if self.selected_index == maintenance_start {
            return Ok(Some(AppMessage::EnterExport));
        }
        if self.selected_index == maintenance_start + 1 {
            return Ok(Some(AppMessage::EnterImport));
        }

        let advanced_start = maintenance_start + 2;

        match self.selected_index {
            idx if idx == advanced_start => {
                ctx.settings.enable_claude_commands = !ctx.settings.enable_claude_commands;
                ctx.storage.save_settings(ctx.settings.clone()).await?;
                Ok(Some(AppMessage::Notify(
                    format!(
                        "Claude Command and Skill Discovery: {}",
                        if ctx.settings.enable_claude_commands { "ON" } else { "OFF" }
                    ),
                    ratatui_toaster::ToastType::Info,
                )))
            }
            idx if idx == advanced_start + 1 => {
                ctx.settings.enable_claude_builtin_commands =
                    !ctx.settings.enable_claude_builtin_commands;
                ctx.storage.save_settings(ctx.settings.clone()).await?;
                Ok(Some(AppMessage::Notify(
                    format!(
                        "Claude Built-in Commands: {}",
                        if ctx.settings.enable_claude_builtin_commands { "ON" } else { "OFF" }
                    ),
                    ratatui_toaster::ToastType::Info,
                )))
            }
            idx if idx == advanced_start + 2 => {
                ctx.settings.use_nerd_font = !ctx.settings.use_nerd_font;
                ctx.storage.save_settings(ctx.settings.clone()).await?;
                Ok(Some(AppMessage::Notify(
                    format!(
                        "Use Nerd Font Icons: {}",
                        if ctx.settings.use_nerd_font { "ON" } else { "OFF" }
                    ),
                    ratatui_toaster::ToastType::Info,
                )))
            }
            idx if idx == advanced_start + 4 => Ok(Some(AppMessage::ToggleStartupBehavior)),
            idx if idx == advanced_start + 5
                && ctx.settings.startup_behavior == contracts::StartupBehavior::Specific =>
            {
                Ok(Some(AppMessage::SelectStartupProject))
            }
            _ => {
                let ai_idx = advanced_start
                    + 5
                    + usize::from(
                        ctx.settings.startup_behavior == contracts::StartupBehavior::Specific,
                    );
                match self.selected_index {
                    idx if idx == ai_idx => {
                        ctx.settings.ai_enabled = !ctx.settings.ai_enabled;
                        ctx.storage.save_settings(ctx.settings.clone()).await?;
                        Ok(Some(AppMessage::Notify(
                            format!(
                                "AI features: {}",
                                if ctx.settings.ai_enabled { "ON" } else { "OFF" }
                            ),
                            ratatui_toaster::ToastType::Info,
                        )))
                    }
                    idx if idx == ai_idx + 1 => {
                        ctx.settings.ai_model_tier = match ctx.settings.ai_model_tier {
                            contracts::ModelTier::Fast => contracts::ModelTier::Balanced,
                            contracts::ModelTier::Balanced => contracts::ModelTier::Quality,
                            contracts::ModelTier::Quality => contracts::ModelTier::Fast,
                        };
                        ctx.storage.save_settings(ctx.settings.clone()).await?;
                        Ok(None)
                    }
                    idx if idx == ai_idx + 2 => {
                        ctx.settings.ai_auto_title = !ctx.settings.ai_auto_title;
                        ctx.storage.save_settings(ctx.settings.clone()).await?;
                        Ok(None)
                    }
                    idx if idx == ai_idx + 3 => Ok(Some(AppMessage::RequestModelDownload)),
                    _ => Ok(None),
                }
            }
        }
    }

    async fn handle_project_ops(
        &mut self,
        msg: crate::types::AppMessage,
        ctx: &crate::types::UpdateContext<'_>,
    ) -> Result<Option<crate::types::AppMessage>> {
        use crate::types::AppMessage;
        match msg {
            AppMessage::SelectProject => {
                self.projects_manager.load_projects(ctx.storage).await?;
                let id_to_match = if self.projects_manager.selecting_startup_project {
                    ctx.settings.specific_project_id
                } else {
                    self.projects_manager.active_project_id
                };
                let pos = id_to_match
                    .and_then(|id| self.projects_manager.projects.iter().position(|p| p.id == id))
                    .map_or(0, |p| p + 1);
                self.projects_manager.project_list_state.select(Some(pos));
            }
            AppMessage::SetProject(id) => {
                let mut settings = ctx.settings.clone();
                let is_startup = self.projects_manager.select_project(id, &mut settings);
                ctx.storage.save_settings(settings).await?;
                if is_startup {
                    return Ok(Some(AppMessage::Notify(
                        "Startup project updated".into(),
                        ratatui_toaster::ToastType::Info,
                    )));
                }
                self.project_filter = true;
                self.load_prompts(ctx.storage).await?;
                let name = id
                    .and_then(|id| self.projects_manager.projects.iter().find(|p| p.id == id))
                    .map_or_else(|| "Default".into(), |p| p.title.clone());
                return Ok(Some(AppMessage::Notify(
                    format!("Active project: {name}"),
                    ratatui_toaster::ToastType::Info,
                )));
            }
            AppMessage::ToggleStartupBehavior => {
                let mut settings = ctx.settings.clone();
                settings.startup_behavior = match settings.startup_behavior {
                    contracts::StartupBehavior::Ask => contracts::StartupBehavior::LastActivated,
                    contracts::StartupBehavior::LastActivated => {
                        contracts::StartupBehavior::Specific
                    }
                    contracts::StartupBehavior::Specific => contracts::StartupBehavior::Ask,
                };
                ctx.storage.save_settings(settings).await?;
            }
            AppMessage::SelectStartupProject => {
                self.projects_manager.selecting_startup_project = true;
                return Ok(Some(AppMessage::SelectProject));
            }
            AppMessage::AddProject(name) => {
                let name = name.trim();
                if name.is_empty() {
                    return Ok(Some(AppMessage::Notify(
                        "Project title cannot be empty".into(),
                        ratatui_toaster::ToastType::Error,
                    )));
                }
                let mut settings = ctx.settings.clone();
                self.projects_manager.add_project(name, ctx.storage, &mut settings).await?;
                self.project_filter = true;
                self.load_prompts(ctx.storage).await?;
            }
            AppMessage::RenameProject(id, name) => {
                let name = name.trim();
                if name.is_empty() {
                    return Ok(Some(AppMessage::Notify(
                        "Project title cannot be empty".into(),
                        ratatui_toaster::ToastType::Error,
                    )));
                }
                self.projects_manager.rename_project(id, name, ctx.storage).await?;
                return Ok(Some(AppMessage::Notify(
                    "Project renamed".into(),
                    ratatui_toaster::ToastType::Success,
                )));
            }
            AppMessage::DeleteProject(id) => {
                let mut settings = ctx.settings.clone();
                self.projects_manager.delete_project(id, ctx.storage, &mut settings).await?;
                self.project_filter = true;
                self.load_prompts(ctx.storage).await?;
                return Ok(Some(AppMessage::Notify(
                    "Project deleted".into(),
                    ratatui_toaster::ToastType::Warning,
                )));
            }
            AppMessage::EnterRenameProject(id) => {
                self.projects_manager.renaming_project_id = Some(id);
                if let Some(p) = self.projects_manager.projects.iter().find(|p| p.id == id) {
                    self.projects_manager.new_project_name.clone_from(&p.title);
                }
            }
            AppMessage::ProjectPickerInput(key) => return Ok(self.handle_project_picker_input(key)),
            AppMessage::RenameProjectInput(key) => return Ok(self.handle_rename_project_input(key)),
            _ => {}
        }
        Ok(None)
    }

    fn handle_project_picker_input(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Option<crate::types::AppMessage> {
        use crate::types::AppMessage;
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                let total = self.projects_manager.projects.len() + 2;
                let current = self.projects_manager.project_list_state.selected().unwrap_or(0);
                self.projects_manager.project_list_state.select(Some((current + 1) % total));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let total = self.projects_manager.projects.len() + 2;
                let current = self.projects_manager.project_list_state.selected().unwrap_or(0);
                self.projects_manager
                    .project_list_state
                    .select(Some((current + total - 1) % total));
            }
            KeyCode::Enter => {
                let selected = self.projects_manager.project_list_state.selected().unwrap_or(0);
                if selected == 0 {
                    return Some(AppMessage::SetProject(None));
                } else if selected <= self.projects_manager.projects.len() {
                    let id = self.projects_manager.projects[selected - 1].id;
                    return Some(AppMessage::SetProject(Some(id)));
                }
                return Some(AppMessage::EnterAddProject);
            }
            KeyCode::Tab => self.project_filter = !self.project_filter,
            KeyCode::Char('x' | 'd') | KeyCode::Delete => {
                let selected = self.projects_manager.project_list_state.selected().unwrap_or(0);
                if selected > 0 && selected <= self.projects_manager.projects.len() {
                    let id = self.projects_manager.projects[selected - 1].id;
                    return Some(AppMessage::DeleteProject(id));
                }
            }
            KeyCode::Char('r') => {
                let selected = self.projects_manager.project_list_state.selected().unwrap_or(0);
                if selected > 0 && selected <= self.projects_manager.projects.len() {
                    let id = self.projects_manager.projects[selected - 1].id;
                    return Some(AppMessage::EnterRenameProject(id));
                }
            }
            _ => {}
        }
        None
    }

    fn handle_rename_project_input(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Option<crate::types::AppMessage> {
        use crate::types::AppMessage;
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Esc => {
                return Some(AppMessage::SelectProject);
            }
            KeyCode::Enter => {
                if let Some(id) = self.projects_manager.renaming_project_id {
                    let name = self.projects_manager.new_project_name.clone();
                    return Some(AppMessage::RenameProject(id, name));
                }
            }
            KeyCode::Backspace => {
                self.projects_manager.new_project_name.pop();
            }
            KeyCode::Char(c) => {
                self.projects_manager.new_project_name.push(c);
            }
            _ => {}
        }
        None
    }

    async fn handle_search_ops(
        &mut self,
        msg: crate::types::AppMessage,
        ctx: &crate::types::UpdateContext<'_>,
    ) -> Result<Option<crate::types::AppMessage>> {
        use crate::types::AppMessage;
        match msg {
            AppMessage::Search(query) => {
                self.search_query = query;
                self.load_prompts(ctx.storage).await?;
            }
            AppMessage::Paste(content) => {
                self.search_query.push_str(&content.replace(['\n', '\r'], ""));
                self.load_prompts(ctx.storage).await?;
            }
            AppMessage::SearchInput(key) => self.handle_search_input(key, ctx).await?,
            _ => {}
        }
        Ok(None)
    }

    async fn handle_search_input(
        &mut self,
        key: crossterm::event::KeyEvent,
        ctx: &crate::types::UpdateContext<'_>,
    ) -> Result<()> {
        use crossterm::event::{KeyCode, KeyModifiers};
        match key.code {
            KeyCode::Esc => self.search_query.clear(),
            KeyCode::Enter => {}
            KeyCode::Char('\u{7f}') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    if let Some(pos) = self.search_query.trim_end().rfind(' ') {
                        self.search_query.truncate(pos + 1);
                    } else {
                        self.search_query.clear();
                    }
                } else {
                    self.search_query.pop();
                }
            }
            KeyCode::Char(c) => self.search_query.push(c),
            KeyCode::Backspace => {
                self.search_query.pop();
            }
            _ => return Ok(()),
        }
        self.load_prompts(ctx.storage).await?;
        Ok(())
    }

    async fn handle_theme_ops(
        &mut self,
        msg: &crate::types::AppMessage,
        ctx: &mut crate::types::UpdateContext<'_>,
    ) -> Result<Option<crate::types::AppMessage>> {
        use crate::types::AppMessage;
        match msg {
            AppMessage::SelectTheme => {
                self.original_theme = ctx.settings.theme_name.clone();
                let themes = ratatui_themes::ThemeName::all();
                let pos = ctx
                    .settings
                    .theme_name
                    .as_ref()
                    .and_then(|name| themes.iter().position(|t| format!("{t:?}") == *name))
                    .unwrap_or(0);
                self.theme_list_state.select(Some(pos));
            }
            AppMessage::ThemePickerInput(key) => {
                return self.handle_theme_picker_input(key, ctx).await
            }
            _ => {}
        }
        Ok(None)
    }

    async fn handle_theme_picker_input(
        &mut self,
        key: &crossterm::event::KeyEvent,
        ctx: &mut crate::types::UpdateContext<'_>,
    ) -> Result<Option<crate::types::AppMessage>> {
        use crate::types::AppMessage;
        use crossterm::event::KeyCode;
        use ratatui_themes::ThemeName;
        let themes = ThemeName::all();
        match key.code {
            KeyCode::Esc => {
                ctx.settings.theme_name = self.original_theme.take();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let current = self.theme_list_state.selected().unwrap_or(0);
                if current < themes.len() - 1 {
                    let new_idx = current + 1;
                    self.theme_list_state.select(Some(new_idx));
                    ctx.settings.theme_name = Some(format!("{:?}", themes[new_idx]));
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let current = self.theme_list_state.selected().unwrap_or(0);
                if current > 0 {
                    let new_idx = current - 1;
                    self.theme_list_state.select(Some(new_idx));
                    ctx.settings.theme_name = Some(format!("{:?}", themes[new_idx]));
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let selected = self.theme_list_state.selected().unwrap_or(0);
                ctx.settings.theme_name = Some(format!("{:?}", themes[selected]));
                self.original_theme = None;
                ctx.storage.save_settings(ctx.settings.clone()).await?;
                return Ok(Some(AppMessage::Notify(
                    "Theme updated!".into(),
                    ratatui_toaster::ToastType::Success,
                )));
            }
            _ => {}
        }
        Ok(None)
    }

    pub fn current_project_path(&self) -> String {
        self.current_path.clone()
    }
}
