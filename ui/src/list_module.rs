use contracts::{Prompt, Tab, Storage, Result, PreviewMode, PromptFilter};
use std::sync::Arc;
use uuid::Uuid;
use chrono;

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub tab: Tab,
    pub prompts: Vec<Prompt>,
}

#[derive(Debug)]
pub struct ListModule {
    pub active_tab: Tab,
    pub prompts: Vec<Prompt>,
    pub projects: Vec<contracts::Project>,
    pub selected_index: usize,
    pub list_state: ratatui::widgets::ListState,
    pub settings_slash_list_state: ratatui::widgets::ListState,
    pub theme_list_state: ratatui::widgets::ListState,
    pub project_list_state: ratatui::widgets::ListState,
    pub undo_stack: Vec<HistoryEntry>,
    pub redo_stack: Vec<HistoryEntry>,
    pub branch_filter: bool,
    pub folder_filter: bool,
    pub project_filter: bool,
    pub active_project_id: Option<Uuid>,
    pub search_query: String,
    pub current_path: String,
    pub original_theme: Option<String>,
    pub current_branch: Option<String>,
    pub settings_scroll_offset: u16,
    pub new_project_name: String,
}

impl Default for ListModule {
    fn default() -> Self {
        Self {
            active_tab: Tab::Prompts,
            prompts: Vec::new(),
            projects: Vec::new(),
            selected_index: 0,
            list_state: ratatui::widgets::ListState::default().with_selected(Some(0)),
            settings_slash_list_state: ratatui::widgets::ListState::default().with_selected(Some(0)),
            theme_list_state: ratatui::widgets::ListState::default().with_selected(Some(0)),
            project_list_state: ratatui::widgets::ListState::default().with_selected(Some(0)),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            branch_filter: false,
            folder_filter: false,
            project_filter: false,
            active_project_id: None,
            search_query: String::new(),
            current_path: std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .to_string_lossy()
                .into_owned(),
            original_theme: None,
            current_branch: None,
            settings_scroll_offset: 0,
            new_project_name: String::new(),
        }
    }
}

impl ListModule {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn load_prompts(&mut self, storage: &Arc<dyn Storage>) -> Result<()> {
        let path = self.current_project_path();
        
        // Ensure project info is saved
        let _ = storage.save_project_info(&path, contracts::ProjectInfo { path: path.clone() }).await;

        let filter = PromptFilter {
            folder: if self.folder_filter || self.active_tab == Tab::Canned || self.active_tab == Tab::Snippets { None } else { Some(path) },
            branch: if self.branch_filter { self.current_branch.clone() } else { None },
            project_id: if self.project_filter { self.active_project_id } else { None },
            project_filter: self.project_filter,
            tab: Some(self.active_tab),
        };

        let mut prompts = storage.get_prompts(filter).await?;

        if !self.search_query.is_empty() {
            let query = self.search_query.to_lowercase();
            prompts.retain(|p| {
                p.text.to_lowercase().contains(&query) || 
                p.name.as_deref().unwrap_or("").to_lowercase().contains(&query)
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

    pub fn set_tab(&mut self, tab: Tab) {
        self.active_tab = tab;
        self.selected_index = 0;
        self.list_state.select(Some(0));
    }

    pub fn move_down(&mut self, settings: &contracts::Settings) {
        if self.active_tab == Tab::Settings {
            let tabs_len = Tab::settings_display_len();
            let slash_len = settings.slash_commands.len();
            let total_settings = tabs_len + slash_len + 4; // tabs + slash commands + Add New + 3 advanced
            if self.selected_index < total_settings - 1 {
                self.selected_index += 1;
                self.list_state.select(Some(self.selected_index));
                
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

    pub fn move_to_top(&mut self) {
        self.selected_index = 0;
        self.list_state.select(Some(0));
    }

    pub fn move_to_bottom(&mut self, settings: &contracts::Settings) {
        if self.active_tab == Tab::Settings {
            let tabs_len = Tab::settings_display_len();
            let slash_len = settings.slash_commands.len();
            let total_settings = tabs_len + slash_len + 4;
            self.selected_index = total_settings - 1;
            self.list_state.select(Some(self.selected_index));
        } else if !self.prompts.is_empty() {
            self.selected_index = self.prompts.len() - 1;
            self.list_state.select(Some(self.selected_index));
        }
    }

    pub fn push_history(&mut self) {
        let entry = HistoryEntry {
            tab: self.active_tab,
            prompts: self.prompts.clone(),
        };
        self.undo_stack.push(entry);
        self.redo_stack.clear();
        
        // Limit stack size
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
    }

    pub async fn undo(&mut self, storage: &Arc<dyn Storage>) -> Result<bool> {
        if let Some(entry) = self.undo_stack.pop() {
            let current = HistoryEntry {
                tab: self.active_tab,
                prompts: self.prompts.clone(),
            };
            self.redo_stack.push(current);

            self.active_tab = entry.tab;
            self.prompts = entry.prompts;
            
            self.save_current_list(storage).await?;
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn redo(&mut self, storage: &Arc<dyn Storage>) -> Result<bool> {
        if let Some(entry) = self.redo_stack.pop() {
            let current = HistoryEntry {
                tab: self.active_tab,
                prompts: self.prompts.clone(),
            };
            self.undo_stack.push(current);

            self.active_tab = entry.tab;
            self.prompts = entry.prompts;

            self.save_current_list(storage).await?;
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn save_current_list(&self, storage: &Arc<dyn Storage>) -> Result<()> {
        let mut prompts = self.prompts.clone();
        for (i, p) in prompts.iter_mut().enumerate() {
            p.order_index = i as i32;
        }
        storage.save_prompts(prompts).await?;
        Ok(())
    }

    pub async fn update(&mut self, msg: crate::types::AppMessage, ctx: &mut crate::types::UpdateContext<'_>) -> Result<Option<crate::types::AppMessage>> {
        use crate::types::AppMessage;
        match msg {
            AppMessage::NextTab => { self.next_tab(ctx.settings); self.load_prompts(ctx.storage).await?; }
            AppMessage::PrevTab => { self.prev_tab(ctx.settings); self.load_prompts(ctx.storage).await?; }
            AppMessage::SetTab(tab) => { self.set_tab(tab); self.load_prompts(ctx.storage).await?; }
            AppMessage::Undo => {
                if self.undo(ctx.storage).await? {
                    return Ok(Some(AppMessage::Notify("Undo".into(), ratatui_toaster::ToastType::Info)));
                }
            }
            AppMessage::Redo => {
                if self.redo(ctx.storage).await? {
                    return Ok(Some(AppMessage::Notify("Redo".into(), ratatui_toaster::ToastType::Info)));
                }
            }
            AppMessage::MoveDown => self.move_down(ctx.settings),
            AppMessage::MoveUp => self.move_up(ctx.settings),
            AppMessage::MoveToTop => self.move_to_top(),
            AppMessage::MoveToBottom => self.move_to_bottom(ctx.settings),
            AppMessage::MoveItemUp => {
                if self.selected_index > 0 && !self.prompts.is_empty() {
                    self.push_history();
                    self.prompts.swap(self.selected_index, self.selected_index - 1);
                    self.selected_index -= 1;
                    self.save_current_list(ctx.storage).await?;
                }
            }
            AppMessage::MoveItemDown => {
                if !self.prompts.is_empty() && self.selected_index < self.prompts.len() - 1 {
                    self.push_history();
                    self.prompts.swap(self.selected_index, self.selected_index + 1);
                    self.selected_index += 1;
                    self.save_current_list(ctx.storage).await?;
                }
            }
            AppMessage::StageSelected => {
                if self.active_tab == Tab::Settings || self.prompts.is_empty() {
                    return Ok(None);
                }

                let item = self.prompts[self.selected_index].clone();
                let is_staged = item.staged;
                let is_alias = self.active_tab == Tab::Notes || self.active_tab == Tab::Snippets;

                self.push_history();
                ctx.service.stage_item(&self.current_project_path(), self.active_tab, item).await?;

                let notify_msg = if is_alias {
                    "Copied to clipboard!"
                } else if is_staged {
                    "Prompt un-staged"
                } else {
                    "Prompt staged and copied to clipboard!"
                };
                
                let notify_type = if is_staged && !is_alias { ratatui_toaster::ToastType::Info } else { ratatui_toaster::ToastType::Success };
                
                self.load_prompts(ctx.storage).await?;
                return Ok(Some(AppMessage::Notify(notify_msg.into(), notify_type)));
            }
            AppMessage::ArchiveSelected => {
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
                        return Ok(Some(AppMessage::Notify("Slash command deleted".into(), ratatui_toaster::ToastType::Warning)));
                    }
                    return Ok(None);
                }

                if self.prompts.is_empty() {
                    return Ok(None);
                }

                self.push_history();
                let target = self.prompts[self.selected_index].clone();

                ctx.service.archive_item(&self.current_project_path(), self.active_tab, target).await?;

                let notify_msg = if self.active_tab == Tab::Archive {
                    "Prompt deleted permanently"
                } else {
                    "Prompt moved to archive"
                };
                let notify_type = if self.active_tab == Tab::Archive { ratatui_toaster::ToastType::Warning } else { ratatui_toaster::ToastType::Info };

                self.load_prompts(ctx.storage).await?;
                return Ok(Some(AppMessage::Notify(notify_msg.into(), notify_type)));
            }
            AppMessage::DuplicateSelected => {
                if self.active_tab == Tab::Settings || self.prompts.is_empty() {
                    return Ok(None);
                }

                self.push_history();
                let target = self.prompts[self.selected_index].clone();

                if let Some(new_prompt) = ctx.service.duplicate_item(&self.current_project_path(), self.active_tab, target).await? {
                    self.prompts.insert(self.selected_index + 1, new_prompt);
                    self.selected_index += 1;
                    self.list_state.select(Some(self.selected_index));
                    return Ok(Some(AppMessage::Notify("Prompt duplicated".into(), ratatui_toaster::ToastType::Success)));
                }
            }
            AppMessage::RestoreSelected => {
                if self.active_tab != Tab::Archive || self.prompts.is_empty() {
                    return Ok(None);
                }

                self.push_history();
                let target = self.prompts[self.selected_index].clone();

                ctx.service.restore_item(&self.current_project_path(), target).await?;

                self.load_prompts(ctx.storage).await?;
                return Ok(Some(AppMessage::Notify("Prompt restored".into(), ratatui_toaster::ToastType::Success)));
            }
            AppMessage::ToggleSetting => {
                if self.active_tab != Tab::Settings {
                    return Ok(None);
                }

                let tabs: Vec<Tab> = Tab::all().into_iter().filter(|&t| t != Tab::Settings).collect();
                if self.selected_index < tabs.len() {
                    let tab = tabs[self.selected_index];
                    let current = ctx.settings.tab_visibility.get(&tab).copied().unwrap_or(true);
                    ctx.settings.tab_visibility.insert(tab, !current);
                    ctx.storage.save_settings(ctx.settings.clone()).await?;
                    return Ok(Some(AppMessage::Notify(format!("Toggled visibility for {tab:?}"), ratatui_toaster::ToastType::Info)));
                } else if self.selected_index == tabs.len() + ctx.settings.slash_commands.len() + 1 {
                    ctx.settings.enable_claude_commands = !ctx.settings.enable_claude_commands;
                    ctx.storage.save_settings(ctx.settings.clone()).await?;
                    return Ok(Some(AppMessage::Notify(format!("Claude commands: {}", if ctx.settings.enable_claude_commands { "ON" } else { "OFF" }), ratatui_toaster::ToastType::Info)));
                } else if self.selected_index == tabs.len() + ctx.settings.slash_commands.len() + 2 {
                    ctx.settings.use_nerd_font = !ctx.settings.use_nerd_font;
                    ctx.storage.save_settings(ctx.settings.clone()).await?;
                    return Ok(Some(AppMessage::Notify(format!("Use Nerd Font Icons: {}", if ctx.settings.use_nerd_font { "ON" } else { "OFF" }), ratatui_toaster::ToastType::Info)));
                }
            }
            AppMessage::ToggleBranchFilter => {
                self.branch_filter = !self.branch_filter;
                self.load_prompts(ctx.storage).await?;
                let status = if self.branch_filter { "ON" } else { "OFF" };
                return Ok(Some(AppMessage::Notify(format!("Branch filter: {}", status), ratatui_toaster::ToastType::Info)));
            }
            AppMessage::ToggleFolderFilter => {
                self.folder_filter = !self.folder_filter;
                self.load_prompts(ctx.storage).await?;
                let status = if self.folder_filter { "Global (ON)" } else { "Local (OFF)" };
                return Ok(Some(AppMessage::Notify(format!("Folder filter: {}", status), ratatui_toaster::ToastType::Info)));
            }
            AppMessage::ToggleProjectFilter => {
                self.project_filter = !self.project_filter;
                ctx.settings.project_filter = self.project_filter;
                ctx.storage.save_settings(ctx.settings.clone()).await?;
                self.load_prompts(ctx.storage).await?;
                let status = if self.project_filter { "ON" } else { "OFF" };
                return Ok(Some(AppMessage::Notify(format!("Project filter: {}", status), ratatui_toaster::ToastType::Info)));
            }
            AppMessage::SelectProject => {
                self.projects = ctx.storage.get_projects().await?;
                // Select active project in list
                let pos = if let Some(id) = self.active_project_id {
                    self.projects.iter().position(|p| p.id == id).map(|p| p + 1).unwrap_or(0)
                } else {
                    0
                };
                self.project_list_state.select(Some(pos));
            }
            AppMessage::SetProject(id) => {
                self.active_project_id = id;
                ctx.settings.active_project_id = id;
                ctx.storage.save_settings(ctx.settings.clone()).await?;
                self.load_prompts(ctx.storage).await?;
            }
            AppMessage::AddProject(name) => {
                let name = name.trim();
                if name.is_empty() {
                    return Ok(Some(AppMessage::Notify("Project title cannot be empty".into(), ratatui_toaster::ToastType::Error)));
                }
                let project = contracts::Project {
                    id: Uuid::new_v4(),
                    title: name.to_string(),
                    created_at: chrono::Utc::now(),
                };
                ctx.storage.save_project(project.clone()).await?;
                self.active_project_id = Some(project.id);
                ctx.settings.active_project_id = Some(project.id);
                ctx.storage.save_settings(ctx.settings.clone()).await?;
                self.projects = ctx.storage.get_projects().await?;
                self.load_prompts(ctx.storage).await?;
            }
            AppMessage::DeleteProject(id) => {
                ctx.storage.delete_project(id).await?;
                if self.active_project_id == Some(id) {
                    self.active_project_id = None;
                    ctx.settings.active_project_id = None;
                    ctx.storage.save_settings(ctx.settings.clone()).await?;
                }
                self.projects = ctx.storage.get_projects().await?;
                self.load_prompts(ctx.storage).await?;
                return Ok(Some(AppMessage::Notify("Project deleted".into(), ratatui_toaster::ToastType::Warning)));
            }
            AppMessage::ProjectPickerInput(key) => {
                match key.code {
                    crossterm::event::KeyCode::Char('j') | crossterm::event::KeyCode::Down => {
                        let total = self.projects.len() + 2; // Default + Projects + Add New
                        let current = self.project_list_state.selected().unwrap_or(0);
                        self.project_list_state.select(Some((current + 1) % total));
                    }
                    crossterm::event::KeyCode::Char('k') | crossterm::event::KeyCode::Up => {
                        let total = self.projects.len() + 2;
                        let current = self.project_list_state.selected().unwrap_or(0);
                        self.project_list_state.select(Some((current + total - 1) % total));
                    }
                    crossterm::event::KeyCode::Enter => {
                        let selected = self.project_list_state.selected().unwrap_or(0);
                        if selected == 0 {
                            return Ok(Some(AppMessage::SetProject(None)));
                        } else if selected <= self.projects.len() {
                            let id = self.projects[selected - 1].id;
                            return Ok(Some(AppMessage::SetProject(Some(id))));
                        } else {
                            return Ok(Some(AppMessage::EnterAddProject));
                        }
                    }
                    crossterm::event::KeyCode::Char('x') => {
                        let selected = self.project_list_state.selected().unwrap_or(0);
                        if selected > 0 && selected <= self.projects.len() {
                            let id = self.projects[selected - 1].id;
                            return Ok(Some(AppMessage::DeleteProject(id)));
                        }
                    }
                    _ => {}
                }
            }
            AppMessage::Search(query) => {
                self.search_query = query;
                self.load_prompts(ctx.storage).await?;
            }
            AppMessage::Paste(content) => {
                self.search_query.push_str(&content.replace(['\n', '\r'], ""));
                self.load_prompts(ctx.storage).await?;
            }
            AppMessage::SearchInput(key) => {
                match key.code {
                    crossterm::event::KeyCode::Esc => {
                        self.search_query.clear();
                        self.load_prompts(ctx.storage).await?;
                    }
                    crossterm::event::KeyCode::Enter => { /* Mode change to List by App */ }
                    crossterm::event::KeyCode::Char('\u{7f}') => {
                        if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                            if let Some(pos) = self.search_query.trim_end().rfind(' ') {
                                self.search_query.truncate(pos + 1);
                            } else {
                                self.search_query.clear();
                            }
                        } else {
                            self.search_query.pop();
                        }
                        self.load_prompts(ctx.storage).await?;
                    }
                    crossterm::event::KeyCode::Char(c) => {
                        self.search_query.push(c);
                        self.load_prompts(ctx.storage).await?;
                    }
                    crossterm::event::KeyCode::Backspace => {
                        self.search_query.pop();
                        self.load_prompts(ctx.storage).await?;
                    }
                    _ => {}
                }
            }
            AppMessage::SelectTheme => {
                self.original_theme = ctx.settings.theme_name.clone();
                let themes = ratatui_themes::ThemeName::all();
                if let Some(ref current_name) = ctx.settings.theme_name {
                    if let Some(pos) = themes.iter().position(|t| format!("{:?}", t) == *current_name) {
                        self.theme_list_state.select(Some(pos));
                    } else {
                        self.theme_list_state.select(Some(0));
                    }
                } else {
                    self.theme_list_state.select(Some(0));
                }
            }
            AppMessage::ThemePickerInput(key) => {
                match key.code {
                    crossterm::event::KeyCode::Esc => {
                        ctx.settings.theme_name = self.original_theme.take();
                    }
                    crossterm::event::KeyCode::Char('j') | crossterm::event::KeyCode::Down => {
                        let themes = ratatui_themes::ThemeName::all();
                        let current = self.theme_list_state.selected().unwrap_or(0);
                        if current < themes.len() - 1 {
                            let new_idx = current + 1;
                            self.theme_list_state.select(Some(new_idx));
                            ctx.settings.theme_name = Some(format!("{:?}", themes[new_idx]));
                        }
                    }
                    crossterm::event::KeyCode::Char('k') | crossterm::event::KeyCode::Up => {
                        let themes = ratatui_themes::ThemeName::all();
                        let current = self.theme_list_state.selected().unwrap_or(0);
                        if current > 0 {
                            let new_idx = current - 1;
                            self.theme_list_state.select(Some(new_idx));
                            ctx.settings.theme_name = Some(format!("{:?}", themes[new_idx]));
                        }
                    }
                    crossterm::event::KeyCode::Enter | crossterm::event::KeyCode::Char(' ') => {
                        let themes = ratatui_themes::ThemeName::all();
                        let selected = self.theme_list_state.selected().unwrap_or(0);
                        ctx.settings.theme_name = Some(format!("{:?}", themes[selected]));
                        self.original_theme = None;
                        ctx.storage.save_settings(ctx.settings.clone()).await?;
                        return Ok(Some(AppMessage::Notify("Theme updated!".into(), ratatui_toaster::ToastType::Success)));
                    }
                    _ => {}
                }
            }
            AppMessage::CyclePreviewMode => {
                ctx.settings.preview_mode = match ctx.settings.preview_mode {
                    PreviewMode::Bottom => PreviewMode::Side,
                    PreviewMode::Side => PreviewMode::Hidden,
                    PreviewMode::Hidden => PreviewMode::Bottom,
                };
                let mode_str = match ctx.settings.preview_mode {
                    PreviewMode::Bottom => "Bottom",
                    PreviewMode::Side => "Side",
                    PreviewMode::Hidden => "Hidden",
                };
                ctx.storage.save_settings(ctx.settings.clone()).await?;
                return Ok(Some(AppMessage::Notify(format!("Preview mode: {}", mode_str), ratatui_toaster::ToastType::Info)));
            }
            _ => {}
        }
        Ok(None)
    }

    pub fn current_project_path(&self) -> String {
        self.current_path.clone()
    }
}
