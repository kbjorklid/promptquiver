use contracts::{Clipboard, Git, Prompt, PromptType, Storage, Tab};
use ratatui_toaster::{ToastBuilder, ToastType, ToastEngine, ToastMessage, ToastPosition};
use std::sync::Arc;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use crate::editor::EditorModule;
use crate::list_module::ListModule;

use std::fmt;
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
}


impl fmt::Debug for App<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

use uuid::Uuid;

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

impl App<'_> {
    pub async fn handle_message(&mut self, msg: AppMessage) -> contracts::Result<()> {
        match msg {
            AppMessage::Quit => self.quit(),
            AppMessage::NextTab => { self.next_tab(); self.load_prompts().await?; }
            AppMessage::PrevTab => { self.prev_tab(); self.load_prompts().await?; }
            AppMessage::SetTab(tab) => { self.set_tab(tab); self.load_prompts().await?; }
            AppMessage::Undo => self.undo().await?,
            AppMessage::Redo => self.redo().await?,
            AppMessage::MoveDown => self.move_down(),
            AppMessage::MoveUp => self.move_up(),
            AppMessage::MoveToTop => self.move_to_top(),
            AppMessage::MoveToBottom => self.move_to_bottom(),
            AppMessage::MoveItemUp => self.move_item_up().await?,
            AppMessage::MoveItemDown => self.move_item_down().await?,
            AppMessage::StageSelected => self.stage_selected().await?,
            AppMessage::ArchiveSelected => self.archive_selected().await?,
            AppMessage::DuplicateSelected => self.duplicate_selected().await?,
            AppMessage::RestoreSelected => self.restore_selected().await?,
            AppMessage::EnterEditor(text, id) => self.enter_editor(text, id),
            AppMessage::EnterEditorBefore(text, index) => self.enter_editor_before(text, index),
            AppMessage::ExitEditor => self.exit_editor(),
            AppMessage::SaveEditor => self.save_editor().await?,
            AppMessage::SaveAndStageEditor => self.save_and_stage_editor().await?,
            AppMessage::UpdateAutocomplete => self.update_autocomplete().await?,
            AppMessage::MoveSuggestionDown => self.move_suggestion_down(),
            AppMessage::MoveSuggestionUp => self.move_suggestion_up(),
            AppMessage::SelectSuggestion => self.select_suggestion(),
            AppMessage::ToggleSetting => self.toggle_setting().await?,
            AppMessage::ToggleBranchFilter => {
                self.nav.branch_filter = !self.nav.branch_filter;
                self.load_prompts().await?;
                let status = if self.nav.branch_filter { "ON" } else { "OFF" };
                self.notify(format!("Branch filter: {}", status), ratatui_toaster::ToastType::Info);
            }
            AppMessage::Search(query) => {
                self.nav.search_query = query;
                self.load_prompts().await?;
            }
            AppMessage::GlobalSearch(query) => {
                self.search_all(query).await?;
            }
            AppMessage::Notify(msg, kind) => self.notify(msg, kind),
            AppMessage::EditSetting => self.edit_setting(),
            AppMessage::ConfirmDiscard => {
                self.mode = Mode::ConfirmDiscard;
            }
            AppMessage::CancelDiscard => {
                self.mode = Mode::Editor;
            }
            AppMessage::EditorInput(key) => {
                if self.editor.title_focused && self.nav.active_tab == Tab::Snippets {
                    if !self.editor.title_textarea.input(key) {
                        if let crossterm::event::KeyCode::Char(c) = key.code {
                            self.editor.title_textarea.input(crossterm::event::KeyEvent::new(crossterm::event::KeyCode::Char(c), crossterm::event::KeyModifiers::empty()));
                        }
                    }
                    if self.editor.title_textarea.lines().len() > 1 {
                        let joined = self.editor.title_textarea.lines().join("");
                        self.editor.title_textarea = ratatui_textarea::TextArea::new(vec![joined]);
                        self.editor.title_textarea.move_cursor(ratatui_textarea::CursorMove::End);
                    }
                } else {
                    if self.nav.active_tab == Tab::Settings {
                        if key.code != crossterm::event::KeyCode::Enter {
                            if !self.editor.textarea.input(key) {
                                if let crossterm::event::KeyCode::Char(c) = key.code {
                                    self.editor.textarea.input(crossterm::event::KeyEvent::new(crossterm::event::KeyCode::Char(c), crossterm::event::KeyModifiers::empty()));
                                }
                            }
                            self.update_autocomplete().await?;
                        }
                    } else {
                        if !self.editor.textarea.input(key) {
                            if let crossterm::event::KeyCode::Char(c) = key.code {
                                self.editor.textarea.input(crossterm::event::KeyEvent::new(crossterm::event::KeyCode::Char(c), crossterm::event::KeyModifiers::empty()));
                            }
                        }
                        self.update_autocomplete().await?;
                    }
                }
            }
            AppMessage::SearchInput(key) => {
                match key.code {
                    crossterm::event::KeyCode::Esc => {
                        self.mode = Mode::List;
                        self.nav.search_query.clear();
                        self.load_prompts().await?;
                    }
                    crossterm::event::KeyCode::Enter => { self.mode = Mode::List; }
                    crossterm::event::KeyCode::Char('\u{7f}') => {
                        if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                            if let Some(pos) = self.nav.search_query.trim_end().rfind(' ') {
                                self.nav.search_query.truncate(pos + 1);
                            } else {
                                self.nav.search_query.clear();
                            }
                        } else {
                            self.nav.search_query.pop();
                        }
                        self.load_prompts().await?;
                    }
                    crossterm::event::KeyCode::Char(c) => {
                        self.nav.search_query.push(c);
                        self.load_prompts().await?;
                    }
                    crossterm::event::KeyCode::Backspace => {
                        self.nav.search_query.pop();
                        self.load_prompts().await?;
                    }
                    _ => {}
                }
            }
            AppMessage::GlobalSearchInput(key) => {
                match key.code {
                    crossterm::event::KeyCode::Esc => {
                        self.mode = Mode::List;
                        self.nav.global_search_query.clear();
                        self.load_prompts().await?;
                    }
                    crossterm::event::KeyCode::Enter => {
                        self.mode = Mode::List;
                        self.search_all(self.nav.global_search_query.clone()).await?;
                    }
                    crossterm::event::KeyCode::Char('\u{7f}') => {
                        if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                            if let Some(pos) = self.nav.global_search_query.trim_end().rfind(' ') {
                                self.nav.global_search_query.truncate(pos + 1);
                            } else {
                                self.nav.global_search_query.clear();
                            }
                        } else {
                            self.nav.global_search_query.pop();
                        }
                        self.search_all(self.nav.global_search_query.clone()).await?;
                    }
                    crossterm::event::KeyCode::Char(c) => {
                        self.nav.global_search_query.push(c);
                    }
                    crossterm::event::KeyCode::Backspace => {
                        self.nav.global_search_query.pop();
                    }
                    _ => {}
                }
            }
            AppMessage::ToggleMoveMode => {
                self.mode = if self.mode == Mode::Move { Mode::List } else { Mode::Move };
            }
            AppMessage::ThemePickerInput(key) => {
                match key.code {
                    crossterm::event::KeyCode::Esc => {
                        self.settings.theme_name = self.nav.original_theme.take();
                        self.mode = Mode::List;
                    }
                    crossterm::event::KeyCode::Char('j') | crossterm::event::KeyCode::Down => {
                        let themes = ratatui_themes::ThemeName::all();
                        let current = self.nav.theme_list_state.selected().unwrap_or(0);
                        if current < themes.len() - 1 {
                            let new_idx = current + 1;
                            self.nav.theme_list_state.select(Some(new_idx));
                            self.settings.theme_name = Some(format!("{:?}", themes[new_idx]));
                        }
                    }
                    crossterm::event::KeyCode::Char('k') | crossterm::event::KeyCode::Up => {
                        let themes = ratatui_themes::ThemeName::all();
                        let current = self.nav.theme_list_state.selected().unwrap_or(0);
                        if current > 0 {
                            let new_idx = current - 1;
                            self.nav.theme_list_state.select(Some(new_idx));
                            self.settings.theme_name = Some(format!("{:?}", themes[new_idx]));
                        }
                    }
                    crossterm::event::KeyCode::Enter | crossterm::event::KeyCode::Char(' ') => {
                        let themes = ratatui_themes::ThemeName::all();
                        let selected = self.nav.theme_list_state.selected().unwrap_or(0);
                        self.settings.theme_name = Some(format!("{:?}", themes[selected]));
                        self.nav.original_theme = None;
                        self.storage.save_settings(self.settings.clone()).await?;
                        self.mode = Mode::List;
                        self.notify("Theme updated!", ratatui_toaster::ToastType::Success);
                    }
                    _ => {}
                }
            }
            AppMessage::SetTheme(theme) => {
                self.settings.theme_name = theme;
            }
            AppMessage::SelectTheme => {
                self.mode = Mode::ThemePicker;
            }
        }
        Ok(())
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

    pub fn next_tab(&mut self) {
        self.nav.next_tab();
    }

    pub fn prev_tab(&mut self) {
        self.nav.prev_tab();
    }

    pub fn set_tab(&mut self, tab: Tab) {
        self.nav.set_tab(tab);
    }

    pub fn move_down(&mut self) {
        self.nav.move_down(&self.settings);
    }

    pub fn move_up(&mut self) {
        self.nav.move_up(&self.settings);
    }

    pub fn move_to_top(&mut self) {
        self.nav.move_to_top();
    }

    pub fn move_to_bottom(&mut self) {
        self.nav.move_to_bottom(&self.settings);
    }

    pub async fn move_item_up(&mut self) -> contracts::Result<()> {
        if self.nav.selected_index > 0 && !self.nav.prompts.is_empty() {
            self.nav.push_history();
            self.nav.prompts.swap(self.nav.selected_index, self.nav.selected_index - 1);
            self.nav.selected_index -= 1;
            self.nav.save_current_list(&self.storage).await?;
        }
        Ok(())
    }

    pub async fn move_item_down(&mut self) -> contracts::Result<()> {
        if !self.nav.prompts.is_empty() && self.nav.selected_index < self.nav.prompts.len() - 1 {
            self.nav.push_history();
            self.nav.prompts.swap(self.nav.selected_index, self.nav.selected_index + 1);
            self.nav.selected_index += 1;
            self.nav.save_current_list(&self.storage).await?;
        }
        Ok(())
    }

    pub async fn load_prompts(&mut self) -> contracts::Result<()> {
        let path = self.nav.current_project_path();
        
        // Ensure project info is saved
        let _ = self.storage.save_project_info(&path, contracts::ProjectInfo { path: path.clone() }).await;

        self.settings = self.storage.get_settings().await.unwrap_or_default();

        let mut prompts = match self.nav.active_tab {
            Tab::Prompts => self.storage.get_project_prompts(&path).await?,
            Tab::Notes => self.storage.get_project_notes(&path).await?,
            Tab::Archive => self.storage.get_project_archive(&path).await?,
            Tab::Canned => self.storage.get_global_canned().await?,
            Tab::Snippets => self.storage.get_global_snippets().await?,
            Tab::Settings => Vec::new(),
        };

        if self.nav.branch_filter {
            if let Some(ref branch) = self.current_branch {
                prompts.retain(|p| p.branch.as_deref() == Some(branch));
            }
        }

        if !self.nav.search_query.is_empty() {
            let query = self.nav.search_query.to_lowercase();
            prompts.retain(|p| {
                p.text.to_lowercase().contains(&query) || 
                p.name.as_deref().unwrap_or("").to_lowercase().contains(&query)
            });
        }
        
        self.nav.prompts = prompts;
        
        if self.nav.selected_index >= self.nav.prompts.len() && !self.nav.prompts.is_empty() {
            self.nav.selected_index = self.nav.prompts.len() - 1;
        }
        self.nav.list_state.select(Some(self.nav.selected_index));
        
        Ok(())
    }

    pub async fn undo(&mut self) -> contracts::Result<()> {
        if self.nav.undo(&self.storage).await? {
            self.notify("Undo", ToastType::Info);
        }
        Ok(())
    }

    pub async fn redo(&mut self) -> contracts::Result<()> {
        if self.nav.redo(&self.storage).await? {
            self.notify("Redo", ToastType::Info);
        }
        Ok(())
    }

    pub async fn stage_selected(&mut self) -> contracts::Result<()> {
        if self.nav.active_tab == Tab::Settings || self.nav.prompts.is_empty() {
            return Ok(());
        }

        let item = self.nav.prompts[self.nav.selected_index].clone();
        let is_staged = item.staged;
        let is_alias = self.nav.active_tab == Tab::Notes || self.nav.active_tab == Tab::Snippets;

        self.nav.push_history();
        self.service.stage_item(&self.nav.current_project_path(), self.nav.active_tab, item).await?;

        if is_alias {
            self.notify("Copied to clipboard!", ToastType::Success);
        } else if is_staged {
            self.notify("Prompt un-staged", ToastType::Info);
        } else {
            self.notify("Prompt staged and copied to clipboard!", ToastType::Success);
        }

        self.load_prompts().await?;
        Ok(())
    }

    pub fn enter_editor(&mut self, text: String, id: Option<uuid::Uuid>) {
        self.nav.search_query.clear();
        self.nav.global_search_query.clear();
        self.mode = Mode::Editor;
        
        let title = if self.nav.active_tab == Tab::Snippets {
            if let Some(id) = id {
                self.nav.prompts.iter().find(|p| p.id == id).and_then(|p| p.name.clone()).unwrap_or_default()
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
        self.nav.global_search_query.clear();
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
        let tabs_len = Tab::all().len();
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

    pub async fn save_editor(&mut self) -> contracts::Result<()> {
        let text = self.editor.textarea.lines().join("\n");
        let path = self.nav.current_project_path();

        if self.nav.active_tab == Tab::Settings {
            let tabs_len = Tab::all().len();
            let slash_len = self.settings.slash_commands.len();

            let re = regex::Regex::new("^[a-zA-Z0-9_-]+$").unwrap();
            let trimmed = text.trim();
            if !trimmed.is_empty() && !re.is_match(trimmed) {
                self.notify("Slash command must match [a-zA-Z0-9_-]+", ToastType::Error);
                return Ok(());
            }

            if self.nav.selected_index >= tabs_len && self.nav.selected_index < tabs_len + slash_len {
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

        if let Some(id) = self.editor.editing_id {
            match self.nav.active_tab {
                Tab::Prompts => {
                    let mut list = self.storage.get_project_prompts(&path).await?;
                    if let Some(p) = list.iter_mut().find(|p| p.id == id) {
                        p.text = text;
                        p.name = title;
                        p.updated_at = chrono::Utc::now();
                    }
                    self.storage.save_project_prompts(&path, list).await?;
                }
                Tab::Notes => {
                    let mut list = self.storage.get_project_notes(&path).await?;
                    if let Some(p) = list.iter_mut().find(|p| p.id == id) {
                        p.text = text;
                        p.name = title;
                        p.updated_at = chrono::Utc::now();
                    }
                    self.storage.save_project_notes(&path, list).await?;
                }
                Tab::Canned => {
                    let mut list = self.storage.get_global_canned().await?;
                    if let Some(p) = list.iter_mut().find(|p| p.id == id) {
                        p.text = text;
                        p.name = title;
                        p.updated_at = chrono::Utc::now();
                    }
                    self.storage.save_global_canned(list).await?;
                }
                Tab::Snippets => {
                    let mut list = self.storage.get_global_snippets().await?;
                    if let Some(p) = list.iter_mut().find(|p| p.id == id) {
                        p.text = text;
                        p.name = title;
                        p.updated_at = chrono::Utc::now();
                    }
                    self.storage.save_global_snippets(list).await?;
                }
                _ => {}
            }
        } else {
            // Add new
            let r#type = match self.nav.active_tab {
                Tab::Notes => PromptType::Note,
                Tab::Snippets => PromptType::Snippet,
                _ => PromptType::Prompt,
            };
            
            let current_branch = self.git.get_current_branch(&path).await.unwrap_or_default();
            let prompt = Prompt::new(text, r#type, current_branch, title);
            
            match self.nav.active_tab {
                Tab::Prompts => {
                    let mut list = self.storage.get_project_prompts(&path).await?;
                    if let Some(idx) = self.editor.insert_index {
                        list.insert(idx, prompt);
                    } else {
                        list.push(prompt);
                    }
                    self.storage.save_project_prompts(&path, list).await?;
                }
                Tab::Notes => {
                    let mut list = self.storage.get_project_notes(&path).await?;
                    if let Some(idx) = self.editor.insert_index {
                        list.insert(idx, prompt);
                    } else {
                        list.push(prompt);
                    }
                    self.storage.save_project_notes(&path, list).await?;
                }
                Tab::Canned => {
                    let mut list = self.storage.get_global_canned().await?;
                    if let Some(idx) = self.editor.insert_index {
                        list.insert(idx, prompt);
                    } else {
                        list.push(prompt);
                    }
                    self.storage.save_global_canned(list).await?;
                }
                Tab::Snippets => {
                    let mut list = self.storage.get_global_snippets().await?;
                    if let Some(idx) = self.editor.insert_index {
                        list.insert(idx, prompt);
                    } else {
                        list.push(prompt);
                    }
                    self.storage.save_global_snippets(list).await?;
                }
                _ => {}
            }
        }

        self.exit_editor();
        self.load_prompts().await?;
        self.notify("Prompt saved!", ToastType::Success);
        Ok(())
    }

    pub async fn archive_selected(&mut self) -> contracts::Result<()> {
        if self.nav.active_tab == Tab::Settings {
            let tabs_len = Tab::all().len();
            let slash_len = self.settings.slash_commands.len();
            if self.nav.selected_index >= tabs_len && self.nav.selected_index < tabs_len + slash_len {
                let idx = self.nav.selected_index - tabs_len;
                self.settings.slash_commands.remove(idx);
                self.storage.save_settings(self.settings.clone()).await?;
                self.notify("Slash command deleted", ToastType::Warning);
                if self.nav.selected_index > 0 {
                    self.nav.selected_index -= 1;
                }
            }
            return Ok(());
        }

        if self.nav.prompts.is_empty() {
            return Ok(());
        }

        self.nav.push_history();
        let target = self.nav.prompts[self.nav.selected_index].clone();

        self.service.archive_item(&self.nav.current_project_path(), self.nav.active_tab, target).await?;

        if self.nav.active_tab == Tab::Archive {
            self.notify("Prompt deleted permanently", ToastType::Warning);
        } else {
            self.notify("Prompt moved to archive", ToastType::Info);
        }

        self.load_prompts().await?;
        Ok(())
    }

    pub async fn duplicate_selected(&mut self) -> contracts::Result<()> {
        if self.nav.active_tab == Tab::Settings || self.nav.prompts.is_empty() {
            return Ok(());
        }

        self.nav.push_history();
        let target = self.nav.prompts[self.nav.selected_index].clone();

        if let Some(new_prompt) = self.service.duplicate_item(&self.nav.current_project_path(), self.nav.active_tab, target).await? {
            // Update in-memory list and selection
            self.nav.prompts.insert(self.nav.selected_index + 1, new_prompt);
            self.nav.selected_index += 1;
            self.nav.list_state.select(Some(self.nav.selected_index));
            self.notify("Prompt duplicated", ToastType::Success);
        }
        
        Ok(())
    }

    pub async fn copy_selected(&mut self) -> contracts::Result<()> {
        if self.nav.prompts.is_empty() || self.nav.active_tab == Tab::Settings {
            return Ok(());
        }

        let target = self.nav.prompts[self.nav.selected_index].clone();
        
        self.service.copy_item(&self.nav.current_project_path(), self.nav.active_tab, target).await?;

        // Update in-memory state to reflect last_copied
        self.load_prompts().await?;
        self.notify("Copied to clipboard!", ToastType::Success);
        
        Ok(())
    }

    pub async fn restore_selected(&mut self) -> contracts::Result<()> {
        if self.nav.active_tab != Tab::Archive || self.nav.prompts.is_empty() {
            return Ok(());
        }

        self.nav.push_history();
        let target = self.nav.prompts[self.nav.selected_index].clone();

        self.service.restore_item(&self.nav.current_project_path(), target).await?;

        self.load_prompts().await?;
        self.notify("Prompt restored", ToastType::Success);
        Ok(())
    }

    pub async fn update_autocomplete(&mut self) -> contracts::Result<()> {
        let snippets = self.storage.get_global_snippets().await?;
        self.editor.update_autocomplete(
            snippets,
            &self.settings,
            self.nav.current_path.clone(),
            &self.file_search_tx
        ).await
    }

    pub fn move_suggestion_down(&mut self) {
        self.editor.move_suggestion_down();
    }

    pub fn move_suggestion_up(&mut self) {
        self.editor.move_suggestion_up();
    }

    pub fn select_suggestion(&mut self) {
        self.editor.select_suggestion();
    }

    pub async fn toggle_setting(&mut self) -> contracts::Result<()> {
        if self.nav.active_tab != Tab::Settings {
            return Ok(());
        }

        let tabs = Tab::all();
        if self.nav.selected_index < tabs.len() {
            let tab = tabs[self.nav.selected_index];
            let current = self.settings.tab_visibility.get(&tab).copied().unwrap_or(true);
            self.settings.tab_visibility.insert(tab, !current);
            self.storage.save_settings(self.settings.clone()).await?;
            self.notify(format!("Toggled visibility for {tab:?}"), ToastType::Info);
        } else if self.nav.selected_index >= tabs.len() && self.nav.selected_index < tabs.len() + self.settings.slash_commands.len() + 1 {
             // Slash commands - maybe edit?
        } else if self.nav.selected_index == tabs.len() + self.settings.slash_commands.len() + 1 {
            self.settings.enable_claude_commands = !self.settings.enable_claude_commands;
            self.storage.save_settings(self.settings.clone()).await?;
            self.notify(format!("Claude commands: {}", if self.settings.enable_claude_commands { "ON" } else { "OFF" }), ToastType::Info);
        } else if self.nav.selected_index == tabs.len() + self.settings.slash_commands.len() + 2 {
            self.settings.use_nerd_font = !self.settings.use_nerd_font;
            self.storage.save_settings(self.settings.clone()).await?;
            self.notify(format!("Use Nerd Font Icons: {}", if self.settings.use_nerd_font { "ON" } else { "OFF" }), ToastType::Info);
        } else if self.nav.selected_index == tabs.len() + self.settings.slash_commands.len() + 3 {
            self.mode = Mode::ThemePicker;
        }

        Ok(())
    }

    pub async fn save_and_stage_editor(&mut self) -> contracts::Result<()> {
        self.save_editor().await?;
        self.stage_selected().await?;
        Ok(())
    }

    pub async fn search_all(&mut self, query: String) -> contracts::Result<()> {
        let path = self.nav.current_project_path();
        let query_lower = query.to_lowercase();
        
        let mut results = Vec::new();
        
        // Search all sources
        let prompts = self.storage.get_project_prompts(&path).await?;
        let notes = self.storage.get_project_notes(&path).await?;
        let archive = self.storage.get_project_archive(&path).await?;
        let canned = self.storage.get_global_canned().await?;
        let snippets = self.storage.get_global_snippets().await?;
        
        let mut all = prompts;
        all.extend(notes);
        all.extend(archive);
        all.extend(canned);
        all.extend(snippets);
        
        for p in all {
            if p.text.to_lowercase().contains(&query_lower) || 
               p.name.as_deref().unwrap_or("").to_lowercase().contains(&query_lower) {
                results.push(p);
            }
        }
        
        self.nav.prompts = results;
        self.nav.selected_index = 0;
        self.notify(format!("Global search found {} results", self.nav.prompts.len()), ToastType::Info);
        Ok(())
    }
}

pub fn walk_files(base_dir: &std::path::Path, current_dir: &std::path::Path, query: &str, results: &mut Vec<contracts::Prompt>) {
    if results.len() >= 100 { // Increased limit for fuzzy matching
        return;
    }
    
    let matcher = SkimMatcherV2::default();
    let query_normalized = query.replace('\\', "/").to_lowercase();

    fn walk_recursive(base_dir: &std::path::Path, current_dir: &std::path::Path, query: &str, query_normalized: &str, matcher: &SkimMatcherV2, results: &mut Vec<(i64, contracts::Prompt)>) {
        if results.len() >= 1000 { // Internal limit to avoid excessive recursion/matching
            return;
        }
        if let Ok(entries) = std::fs::read_dir(current_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name == "target" || name == ".git" || name == "node_modules" || name.starts_with('.') {
                            continue;
                        }
                    }
                    walk_recursive(base_dir, &path, query, query_normalized, matcher, results);
                } else {
                    let relative_path = path.strip_prefix(base_dir)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .to_string();
                    
                    let path_normalized = relative_path.replace('\\', "/");
                    let path_lower = path_normalized.to_lowercase();
                    
                    // Fuzzy match against the normalized relative path
                    if let Some(score) = matcher.fuzzy_match(&path_lower, query_normalized) {
                        let mut final_score = score;
                        
                        // Bonus for exact back-to-back match (case-insensitive)
                        if path_lower.contains(query_normalized) {
                            final_score += 100;
                        }
                        
                        results.push((final_score, contracts::Prompt::new(
                            path.to_string_lossy().to_string(),
                            contracts::PromptType::Note,
                            None,
                            Some(relative_path),
                        )));
                    }
                }
            }
        }
    }

    let mut scored_results = Vec::new();
    walk_recursive(base_dir, current_dir, query, &query_normalized, &matcher, &mut scored_results);
    
    // Sort by score descending
    scored_results.sort_by_key(|b| std::cmp::Reverse(b.0));
    
    for (_, p) in scored_results.into_iter().take(20) {
        results.push(p);
    }
}
