use contracts::{Clipboard, Git, Prompt, PromptType, Storage, Tab};
use ratatui_textarea::TextArea;
use ratatui_toaster::{ToastBuilder, ToastType, ToastEngine, ToastMessage, ToastPosition};
use std::sync::Arc;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

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

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub tab: Tab,
    pub prompts: Vec<Prompt>,
}

#[derive(Debug)]
pub struct AutocompleteState {
    pub open: bool,
    pub suggestions: Vec<Prompt>,
    pub index: usize,
    pub list_state: ratatui::widgets::ListState,
}

impl Default for AutocompleteState {
    fn default() -> Self {
        Self {
            open: false,
            suggestions: Vec::new(),
            index: 0,
            list_state: ratatui::widgets::ListState::default().with_selected(Some(0)),
        }
    }
}

#[derive(Debug)]
pub struct EditorState<'a> {
    pub textarea: TextArea<'a>,
    pub title_textarea: TextArea<'a>,
    pub title_focused: bool,
    pub editing_id: Option<uuid::Uuid>,
    pub insert_index: Option<usize>,
    pub original_text: String,
    pub autocomplete: AutocompleteState,
}

impl Default for EditorState<'_> {
    fn default() -> Self {
        Self {
            textarea: TextArea::default(),
            title_textarea: TextArea::default(),
            title_focused: false,
            editing_id: None,
            insert_index: None,
            original_text: String::new(),
            autocomplete: AutocompleteState::default(),
        }
    }
}

#[derive(Debug)]
pub struct NavigationState {
    pub active_tab: Tab,
    pub prompts: Vec<Prompt>,
    pub selected_index: usize,
    pub list_state: ratatui::widgets::ListState,
    pub settings_slash_list_state: ratatui::widgets::ListState,
    pub theme_list_state: ratatui::widgets::ListState,
    pub undo_stack: Vec<HistoryEntry>,
    pub redo_stack: Vec<HistoryEntry>,
    pub branch_filter: bool,
    pub search_query: String,
    pub global_search_query: String,
    pub current_path: String,
    pub original_theme: Option<String>,
}

impl Default for NavigationState {
    fn default() -> Self {
        Self {
            active_tab: Tab::Prompts,
            prompts: Vec::new(),
            selected_index: 0,
            list_state: ratatui::widgets::ListState::default().with_selected(Some(0)),
            settings_slash_list_state: ratatui::widgets::ListState::default().with_selected(Some(0)),
            theme_list_state: ratatui::widgets::ListState::default().with_selected(Some(0)),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            branch_filter: false,
            search_query: String::new(),
            global_search_query: String::new(),
            current_path: std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .to_string_lossy()
                .into_owned(),
            original_theme: None,
        }
    }
}

pub struct App<'a> {
    pub storage: Arc<dyn Storage>,
    pub clipboard: Arc<dyn Clipboard>,
    pub git: Arc<dyn Git>,
    pub service: Arc<dyn contracts::AppService>,
    pub should_quit: bool,
    pub mode: Mode,
    pub nav: NavigationState,
    pub editor: EditorState<'a>,
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

impl App<'_> {
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
            nav: NavigationState::default(),
            editor: EditorState::default(),
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
        self.nav.active_tab = self.nav.active_tab.next();
        self.nav.selected_index = 0;
        self.nav.list_state.select(Some(0));
    }

    pub fn prev_tab(&mut self) {
        self.nav.active_tab = self.nav.active_tab.prev();
        self.nav.selected_index = 0;
        self.nav.list_state.select(Some(0));
    }

    pub fn set_tab(&mut self, tab: Tab) {
        self.nav.active_tab = tab;
        self.nav.selected_index = 0;
        self.nav.list_state.select(Some(0));
    }

    pub fn move_down(&mut self) {
        if self.nav.active_tab == Tab::Settings {
            let tabs_len = Tab::all().len();
            let slash_len = self.settings.slash_commands.len();
            let total_settings = tabs_len + slash_len + 4; // tabs + slash commands + Add New + 3 advanced
            if self.nav.selected_index < total_settings - 1 {
                self.nav.selected_index += 1;
                self.nav.list_state.select(Some(self.nav.selected_index));
                
                // Update slash list state
                if self.nav.selected_index >= tabs_len && self.nav.selected_index <= tabs_len + slash_len {
                    self.nav.settings_slash_list_state.select(Some(self.nav.selected_index - tabs_len));
                } else {
                    self.nav.settings_slash_list_state.select(None);
                }
            }
        } else if !self.nav.prompts.is_empty() && self.nav.selected_index < self.nav.prompts.len() - 1 {
            self.nav.selected_index += 1;
            self.nav.list_state.select(Some(self.nav.selected_index));
        }
    }

    pub fn move_up(&mut self) {
        if self.nav.selected_index > 0 {
            self.nav.selected_index -= 1;
            self.nav.list_state.select(Some(self.nav.selected_index));

            if self.nav.active_tab == Tab::Settings {
                let tabs_len = Tab::all().len();
                let slash_len = self.settings.slash_commands.len();
                if self.nav.selected_index >= tabs_len && self.nav.selected_index <= tabs_len + slash_len {
                    self.nav.settings_slash_list_state.select(Some(self.nav.selected_index - tabs_len));
                } else {
                    self.nav.settings_slash_list_state.select(None);
                }
            }
        }
    }

    pub fn move_to_top(&mut self) {
        self.nav.selected_index = 0;
        self.nav.list_state.select(Some(0));
    }

    pub fn move_to_bottom(&mut self) {
        if self.nav.active_tab == Tab::Settings {
            let tabs_len = Tab::all().len();
            let slash_len = self.settings.slash_commands.len();
            let total_settings = tabs_len + slash_len + 4;
            self.nav.selected_index = total_settings - 1;
            self.nav.list_state.select(Some(self.nav.selected_index));
        } else if !self.nav.prompts.is_empty() {
            self.nav.selected_index = self.nav.prompts.len() - 1;
            self.nav.list_state.select(Some(self.nav.selected_index));
        }
    }

    pub async fn move_item_up(&mut self) -> contracts::Result<()> {
        if self.nav.selected_index > 0 && !self.nav.prompts.is_empty() {
            self.push_history();
            self.nav.prompts.swap(self.nav.selected_index, self.nav.selected_index - 1);
            self.nav.selected_index -= 1;
            self.save_current_list().await?;
        }
        Ok(())
    }

    pub async fn move_item_down(&mut self) -> contracts::Result<()> {
        if !self.nav.prompts.is_empty() && self.nav.selected_index < self.nav.prompts.len() - 1 {
            self.push_history();
            self.nav.prompts.swap(self.nav.selected_index, self.nav.selected_index + 1);
            self.nav.selected_index += 1;
            self.save_current_list().await?;
        }
        Ok(())
    }

    pub async fn load_prompts(&mut self) -> contracts::Result<()> {
        let path = self.current_project_path();
        
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

    fn current_project_path(&self) -> String {
        self.nav.current_path.clone()
    }

    pub fn push_history(&mut self) {
        let entry = HistoryEntry {
            tab: self.nav.active_tab,
            prompts: self.nav.prompts.clone(),
        };
        self.nav.undo_stack.push(entry);
        self.nav.redo_stack.clear();
        
        // Limit stack size
        if self.nav.undo_stack.len() > 100 {
            self.nav.undo_stack.remove(0);
        }
    }

    pub async fn undo(&mut self) -> contracts::Result<()> {
        if let Some(entry) = self.nav.undo_stack.pop() {
            let current = HistoryEntry {
                tab: self.nav.active_tab,
                prompts: self.nav.prompts.clone(),
            };
            self.nav.redo_stack.push(current);

            self.nav.active_tab = entry.tab;
            self.nav.prompts = entry.prompts;
            
            self.save_current_list().await?;
            self.notify("Undo", ToastType::Info);
        }
        Ok(())
    }

    pub async fn redo(&mut self) -> contracts::Result<()> {
        if let Some(entry) = self.nav.redo_stack.pop() {
            let current = HistoryEntry {
                tab: self.nav.active_tab,
                prompts: self.nav.prompts.clone(),
            };
            self.nav.undo_stack.push(current);

            self.nav.active_tab = entry.tab;
            self.nav.prompts = entry.prompts;

            self.save_current_list().await?;
            self.notify("Redo", ToastType::Info);
        }
        Ok(())
    }

    async fn save_current_list(&mut self) -> contracts::Result<()> {
        let path = self.current_project_path();
        match self.nav.active_tab {
            Tab::Prompts => self.storage.save_project_prompts(&path, self.nav.prompts.clone()).await?,
            Tab::Notes => self.storage.save_project_notes(&path, self.nav.prompts.clone()).await?,
            Tab::Archive => self.storage.save_project_archive(&path, self.nav.prompts.clone()).await?,
            Tab::Canned => self.storage.save_global_canned(self.nav.prompts.clone()).await?,
            Tab::Snippets => self.storage.save_global_snippets(self.nav.prompts.clone()).await?,
            Tab::Settings => {}
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

        self.push_history();
        self.service.stage_item(&self.current_project_path(), self.nav.active_tab, item).await?;

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
        self.editor.textarea = TextArea::new(text.lines().map(String::from).collect());
        self.editor.textarea.set_wrap_mode(ratatui_textarea::WrapMode::WordOrGlyph);
        
        let title = if self.nav.active_tab == Tab::Snippets {
            if let Some(id) = id {
                self.nav.prompts.iter().find(|p| p.id == id).and_then(|p| p.name.clone()).unwrap_or_default()
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        self.editor.title_textarea = TextArea::new(vec![title]);
        self.editor.title_focused = self.nav.active_tab == Tab::Snippets;
        
        self.editor.editing_id = id;
        self.editor.insert_index = None;
        self.editor.original_text = text;
    }

    pub fn enter_editor_before(&mut self, text: String, index: usize) {
        self.nav.search_query.clear();
        self.nav.global_search_query.clear();
        self.mode = Mode::Editor;
        self.editor.textarea = TextArea::new(text.lines().map(String::from).collect());
        self.editor.textarea.set_wrap_mode(ratatui_textarea::WrapMode::WordOrGlyph);

        let title = String::new();        self.editor.title_textarea = TextArea::new(vec![title]);
        self.editor.title_focused = self.nav.active_tab == Tab::Snippets;

        self.editor.editing_id = None;
        self.editor.insert_index = Some(index);
        self.editor.original_text = text;
    }

    pub fn exit_editor(&mut self) {
        self.mode = Mode::List;
        self.editor.editing_id = None;
        self.editor.insert_index = None;
        self.editor.autocomplete.open = false;
        self.editor.autocomplete.suggestions.clear();
        self.editor.title_textarea = TextArea::default();
        self.editor.title_focused = false;
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
            self.editor.textarea = TextArea::new(vec![text.clone()]);
            self.editor.textarea.set_wrap_mode(ratatui_textarea::WrapMode::WordOrGlyph);
            self.editor.title_textarea = TextArea::default();
            self.editor.title_focused = false;
            self.editor.editing_id = None; // We'll use selected_index to know which one
            self.editor.original_text = text;
        } else if self.nav.selected_index == tabs_len + slash_len {
            // Add New Slash Command
            self.mode = Mode::Editor;
            self.editor.textarea = TextArea::default();
            self.editor.textarea.set_wrap_mode(ratatui_textarea::WrapMode::WordOrGlyph);
            self.editor.title_textarea = TextArea::default();
            self.editor.title_focused = false;
            self.editor.editing_id = None;
            self.editor.original_text = String::new();
        }
    }

    pub async fn save_editor(&mut self) -> contracts::Result<()> {
        let text = self.editor.textarea.lines().join("\n");
        let path = self.current_project_path();

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

        self.push_history();

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

        self.push_history();
        let target = self.nav.prompts[self.nav.selected_index].clone();

        self.service.archive_item(&self.current_project_path(), self.nav.active_tab, target).await?;

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

        self.push_history();
        let target = self.nav.prompts[self.nav.selected_index].clone();

        if let Some(new_prompt) = self.service.duplicate_item(&self.current_project_path(), self.nav.active_tab, target).await? {
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
        
        self.service.copy_item(&self.current_project_path(), self.nav.active_tab, target).await?;

        // Update in-memory state to reflect last_copied
        self.load_prompts().await?;
        self.notify("Copied to clipboard!", ToastType::Success);
        
        Ok(())
    }

    pub async fn restore_selected(&mut self) -> contracts::Result<()> {
        if self.nav.active_tab != Tab::Archive || self.nav.prompts.is_empty() {
            return Ok(());
        }

        self.push_history();
        let target = self.nav.prompts[self.nav.selected_index].clone();

        self.service.restore_item(&self.current_project_path(), target).await?;

        self.load_prompts().await?;
        self.notify("Prompt restored", ToastType::Success);
        Ok(())
    }

    pub fn get_current_autocomplete_query(&self) -> Option<(String, String)> {
        let cursor = self.editor.textarea.cursor();
        let row = cursor.0;
        let col = cursor.1;
        
        if row >= self.editor.textarea.lines().len() {
            return None;
        }
        
        let line = &self.editor.textarea.lines()[row];
        let byte_col = line.char_indices().nth(col).map(|(i, _)| i).unwrap_or(line.len());
        let before_cursor = &line[..byte_col];

        let triggers = ["$$", "$", "@", "/"];
        let mut best_trigger = None;
        let mut best_pos = 0;

        for trigger in triggers {
            if let Some(pos) = before_cursor.rfind(trigger) {
                // Check if it's a valid trigger position
                let is_valid = match trigger {
                    "/" => {
                        // Slash is only a trigger if it's at the start or preceded by space
                        pos == 0 || (pos > 0 && before_cursor.as_bytes()[pos - 1] == b' ')
                    }
                    _ => true,
                };
                if !is_valid {
                    continue;
                }

                if trigger == "$" && pos > 0 && before_cursor.as_bytes()[pos - 1] == b'$' {
                    continue;
                }

                if best_trigger.is_none() || pos > best_pos {
                    best_trigger = Some(trigger);
                    best_pos = pos;
                }
            }
        }

        if let Some(trigger) = best_trigger {
            let query = &before_cursor[best_pos + trigger.len()..];
            if query.contains(' ') {
                return None;
            }
            return Some((trigger.to_string(), query.to_string()));
        }
        None
    }

    pub async fn update_autocomplete(&mut self) -> contracts::Result<()> {
        if let Some((trigger, query)) = self.get_current_autocomplete_query() {
            let matcher = SkimMatcherV2::default();
            
            match trigger.as_str() {
                "$" | "$$" => {
                    let snippets = self.storage.get_global_snippets().await?;
                    let query_lower = query.to_lowercase();
                    let mut scored_suggestions: Vec<(i64, Prompt)> = snippets
                        .into_iter()
                        .filter_map(|s| {
                            let text = s.name.as_deref().unwrap_or(&s.text);
                            // Ignore case for snippet suggestions
                            matcher.fuzzy_match(&text.to_lowercase(), &query_lower).map(|score| (score, s))
                        })
                        .collect();
                    
                    scored_suggestions.sort_by_key(|b| std::cmp::Reverse(b.0));
                    self.editor.autocomplete.suggestions = scored_suggestions.into_iter().map(|(_, s)| s).collect();
                }
                "@" => {
                    self.editor.autocomplete.suggestions.clear();
                    self.editor.autocomplete.open = false;
                    if let Some(tx) = &self.file_search_tx {
                        // self.notify(format!("Searching files for: '{}'", query), contracts::ToastType::Info);
                        let _ = tx.try_send((self.nav.current_path.clone(), query.to_string()));
                    }
                    return Ok(());
                }
                "/" => {
                    // Slash commands from settings
                    let query_lower = query.to_lowercase();
                    let mut scored_suggestions: Vec<(i64, Prompt)> = self.settings.slash_commands
                        .iter()
                        .filter_map(|cmd| {
                            matcher.fuzzy_match(&cmd.to_lowercase(), &query_lower).map(|score| (score, Prompt::new(cmd.clone(), PromptType::Prompt, None, Some(cmd.clone()))))
                        })
                        .collect();
                        
                    scored_suggestions.sort_by_key(|b| std::cmp::Reverse(b.0));
                    self.editor.autocomplete.suggestions = scored_suggestions.into_iter().map(|(_, s)| s).collect();
                }
                _ => self.editor.autocomplete.suggestions = Vec::new(),
            }
            
            if self.editor.autocomplete.suggestions.is_empty() {
                self.editor.autocomplete.open = false;
            } else {
                self.editor.autocomplete.open = true;
                if self.editor.autocomplete.index >= self.editor.autocomplete.suggestions.len() {
                    self.editor.autocomplete.index = 0;
                }
            }
        } else {
            self.editor.autocomplete.open = false;
            self.editor.autocomplete.suggestions.clear();
        }

        Ok(())
    }

    pub const fn move_suggestion_down(&mut self) {
        if !self.editor.autocomplete.suggestions.is_empty() && self.editor.autocomplete.index < self.editor.autocomplete.suggestions.len() - 1 {
            self.editor.autocomplete.index += 1;
        }
    }

    pub const fn move_suggestion_up(&mut self) {
        if self.editor.autocomplete.index > 0 {
            self.editor.autocomplete.index -= 1;
        }
    }

    pub fn select_suggestion(&mut self) {
        if !self.editor.autocomplete.suggestions.is_empty() && self.editor.autocomplete.open {
            let snippet = &self.editor.autocomplete.suggestions[self.editor.autocomplete.index];
            let name = snippet.name.as_deref().unwrap_or(&snippet.text);
            
            let cursor = self.editor.textarea.cursor();
            let row = cursor.0;
            let col = cursor.1;
            let line = self.editor.textarea.lines()[row].clone();
            let byte_col = line.char_indices().nth(col).map(|(i, _)| i).unwrap_or(line.len());
            let before_cursor = &line[..byte_col];
            
            let triggers = ["$$", "$", "@", "/"];
            let mut best_trigger = None;
            let mut best_pos = 0;

            for trigger in triggers {
                if let Some(pos) = before_cursor.rfind(trigger) {
                    // Check if it's a valid trigger position
                    let is_valid = match trigger {
                        "/" => {
                            // Slash is only a trigger if it's at the start or preceded by space
                            pos == 0 || (pos > 0 && before_cursor.as_bytes()[pos - 1] == b' ')
                        }
                        _ => true,
                    };

                    if !is_valid {
                        continue;
                    }

                    if trigger == "$" && pos > 0 && before_cursor.as_bytes()[pos - 1] == b'$' {
                        continue;
                    }

                    if best_trigger.is_none() || pos > best_pos {
                        best_trigger = Some(trigger);
                        best_pos = pos;
                    }
                }
            }
            if let Some(trigger) = best_trigger {
                let replacement = match trigger {
                    "$$" => format!("$${name}"),
                    "$" => snippet.text.clone(),
                    "@" => format!("@{name}"),
                    "/" => format!("/{name}"),
                    _ => name.to_string(),
                };

                let mut new_line = line[..best_pos].to_string();
                new_line.push_str(&replacement);
                new_line.push_str(&line[byte_col..]);
                
                let new_col = line[..best_pos].chars().count() + replacement.chars().count();
                
                // This is a bit hacky with ratatui-textarea but works for simple cases
                self.editor.textarea.move_cursor(ratatui_textarea::CursorMove::Jump(row as u16, 0));
                self.editor.textarea.delete_line_by_end();
                self.editor.textarea.insert_str(&new_line);
                self.editor.textarea.move_cursor(ratatui_textarea::CursorMove::Jump(row as u16, new_col as u16));
            }
            
            self.editor.autocomplete.open = false;
            self.editor.autocomplete.suggestions.clear();
            self.editor.autocomplete.index = 0;
        }
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
        let path = self.current_project_path();
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
