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
}

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub tab: Tab,
    pub prompts: Vec<Prompt>,
}

pub struct App<'a> {
    pub storage: Arc<dyn Storage>,
    pub clipboard: Arc<dyn Clipboard>,
    pub git: Arc<dyn Git>,
    pub should_quit: bool,
    pub active_tab: Tab,
    pub prompts: Vec<Prompt>,
    pub selected_index: usize,
    pub mode: Mode,
    pub textarea: TextArea<'a>,
    pub title_textarea: TextArea<'a>,
    pub title_focused: bool,
    pub editing_id: Option<uuid::Uuid>,
    pub insert_index: Option<usize>,
    pub current_branch: Option<String>,
    pub autocomplete_open: bool,
    pub suggestions: Vec<Prompt>,
    pub suggestion_index: usize,
    pub toaster: Option<ToastEngine<ToastMessage>>,
    pub settings: contracts::Settings,
    pub undo_stack: Vec<HistoryEntry>,
    pub redo_stack: Vec<HistoryEntry>,
    pub branch_filter: bool,
    pub search_query: String,
    pub original_text: String,
    pub global_search_query: String,
    pub last_notification_time: Option<std::time::Instant>,
    pub current_path: String,
}


impl fmt::Debug for App<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("App")
            .field("should_quit", &self.should_quit)
            .field("active_tab", &self.active_tab)
            .field("prompts_count", &self.prompts.len())
            .field("selected_index", &self.selected_index)
            .field("mode", &self.mode)
            .field("current_branch", &self.current_branch)
            .field("autocomplete_open", &self.autocomplete_open)
            .finish_non_exhaustive()
    }
}

impl App<'_> {
    #[must_use]
    pub fn new(
        storage: Arc<dyn Storage>,
        clipboard: Arc<dyn Clipboard>,
        git: Arc<dyn Git>,
    ) -> Self {
        Self {
            storage,
            clipboard,
            git,
            should_quit: false,
            active_tab: Tab::Prompts,
            prompts: Vec::new(),
            selected_index: 0,
            mode: Mode::List,
            textarea: TextArea::default(),
            title_textarea: TextArea::default(),
            title_focused: false,
            editing_id: None,
            insert_index: None,
            current_branch: None,
            autocomplete_open: false,
            suggestions: Vec::new(),
            suggestion_index: 0,
            toaster: None,
            settings: contracts::Settings::default(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            branch_filter: false,
            search_query: String::new(),
            original_text: String::new(),
            global_search_query: String::new(),
            last_notification_time: None,
            current_path: std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .to_string_lossy()
                .into_owned(),
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
        self.active_tab = self.active_tab.next();
        self.selected_index = 0;
    }

    pub fn prev_tab(&mut self) {
        self.active_tab = self.active_tab.prev();
        self.selected_index = 0;
    }

    pub const fn set_tab(&mut self, tab: Tab) {
        self.active_tab = tab;
    }

    pub fn move_down(&mut self) {
        if self.active_tab == Tab::Settings {
            let tabs_len = Tab::all().len();
            let slash_len = self.settings.slash_commands.len();
            let total_settings = tabs_len + slash_len + 2; // tabs + slash commands + Add New + advanced
            if self.selected_index < total_settings - 1 {
                self.selected_index += 1;
            }
        } else if !self.prompts.is_empty() && self.selected_index < self.prompts.len() - 1 {
            self.selected_index += 1;
        }
    }

    pub const fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub const fn move_to_top(&mut self) {
        self.selected_index = 0;
    }

    pub fn move_to_bottom(&mut self) {
        if self.active_tab == Tab::Settings {
            let tabs_len = Tab::all().len();
            let slash_len = self.settings.slash_commands.len();
            let total_settings = tabs_len + slash_len + 2;
            self.selected_index = total_settings - 1;
        } else if !self.prompts.is_empty() {
            self.selected_index = self.prompts.len() - 1;
        }
    }

    pub async fn move_item_up(&mut self) -> contracts::Result<()> {
        if self.selected_index > 0 && !self.prompts.is_empty() {
            self.push_history();
            self.prompts.swap(self.selected_index, self.selected_index - 1);
            self.selected_index -= 1;
            self.save_current_list().await?;
        }
        Ok(())
    }

    pub async fn move_item_down(&mut self) -> contracts::Result<()> {
        if !self.prompts.is_empty() && self.selected_index < self.prompts.len() - 1 {
            self.push_history();
            self.prompts.swap(self.selected_index, self.selected_index + 1);
            self.selected_index += 1;
            self.save_current_list().await?;
        }
        Ok(())
    }

    pub async fn load_prompts(&mut self) -> contracts::Result<()> {
        let path = self.current_project_path();
        
        // Ensure project info is saved
        let _ = self.storage.save_project_info(&path, contracts::ProjectInfo { path: path.clone() }).await;

        self.settings = self.storage.get_settings().await.unwrap_or_default();

        let mut prompts = match self.active_tab {
            Tab::Prompts => self.storage.get_project_prompts(&path).await?,
            Tab::Notes => self.storage.get_project_notes(&path).await?,
            Tab::Archive => self.storage.get_project_archive(&path).await?,
            Tab::Canned => self.storage.get_global_canned().await?,
            Tab::Snippets => self.storage.get_global_snippets().await?,
            Tab::Settings => Vec::new(),
        };

        if self.branch_filter {
            if let Some(ref branch) = self.current_branch {
                prompts.retain(|p| p.branch.as_deref() == Some(branch));
            }
        }

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
        
        Ok(())
    }

    fn current_project_path(&self) -> String {
        self.current_path.clone()
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

    pub async fn undo(&mut self) -> contracts::Result<()> {
        if let Some(entry) = self.undo_stack.pop() {
            let current = HistoryEntry {
                tab: self.active_tab,
                prompts: self.prompts.clone(),
            };
            self.redo_stack.push(current);

            self.active_tab = entry.tab;
            self.prompts = entry.prompts;
            
            self.save_current_list().await?;
            self.notify("Undo", ToastType::Info);
        }
        Ok(())
    }

    pub async fn redo(&mut self) -> contracts::Result<()> {
        if let Some(entry) = self.redo_stack.pop() {
            let current = HistoryEntry {
                tab: self.active_tab,
                prompts: self.prompts.clone(),
            };
            self.undo_stack.push(current);

            self.active_tab = entry.tab;
            self.prompts = entry.prompts;

            self.save_current_list().await?;
            self.notify("Redo", ToastType::Info);
        }
        Ok(())
    }

    async fn save_current_list(&mut self) -> contracts::Result<()> {
        let path = self.current_project_path();
        match self.active_tab {
            Tab::Prompts => self.storage.save_project_prompts(&path, self.prompts.clone()).await?,
            Tab::Notes => self.storage.save_project_notes(&path, self.prompts.clone()).await?,
            Tab::Archive => self.storage.save_project_archive(&path, self.prompts.clone()).await?,
            Tab::Canned => self.storage.save_global_canned(self.prompts.clone()).await?,
            Tab::Snippets => self.storage.save_global_snippets(self.prompts.clone()).await?,
            Tab::Settings => {}
        }
        Ok(())
    }

    pub async fn stage_selected(&mut self) -> contracts::Result<()> {
        if self.active_tab == Tab::Settings {
            return Ok(());
        }
        if self.prompts.is_empty() {
            return Ok(());
        }

        self.push_history();
        let path = self.current_project_path();
        let target_idx = self.selected_index;
        let is_staged = self.prompts[target_idx].staged;

        if is_staged {
            // Un-stage
            self.prompts[target_idx].staged = false;
            self.save_current_list().await?;
            self.notify("Prompt un-staged", ToastType::Info);
        } else {
            // Stage
            // 1. Un-stage and Archive others
            let mut prompts = self.storage.get_project_prompts(&path).await?;
            let mut notes = self.storage.get_project_notes(&path).await?;
            let mut snippets = self.storage.get_global_snippets().await?;
            let mut archive = self.storage.get_project_archive(&path).await?;
            let mut canned = self.storage.get_global_canned().await?;

            let mut to_archive = Vec::new();

            for p in &mut prompts {
                if p.staged {
                    p.staged = false;
                    to_archive.push(p.clone());
                }
            }
            for p in &mut notes {
                if p.staged {
                    p.staged = false;
                    to_archive.push(p.clone());
                }
            }
            for p in &mut snippets {
                if p.staged {
                    p.staged = false;
                    to_archive.push(p.clone());
                }
            }
            for p in &mut canned {
                if p.staged {
                    p.staged = false;
                }
            }

            // Remove archived items from their original lists
            prompts.retain(|p| !to_archive.iter().any(|a| a.id == p.id));
            notes.retain(|p| !to_archive.iter().any(|a| a.id == p.id));
            snippets.retain(|p| !to_archive.iter().any(|a| a.id == p.id));

            // Add to archive (to the top)
            for mut p in to_archive {
                p.staged = false;
                archive.insert(0, p);
            }

            // 2. Set target to staged
            let mut target = self.prompts[target_idx].clone();
            target.staged = true;
            
            // Update the list we are currently in
            match self.active_tab {
                Tab::Prompts => {
                    prompts.iter_mut().for_each(|p| if p.id == target.id { p.staged = true; });
                }
                Tab::Notes => {
                    for p in notes.iter_mut() { if p.id == target.id { p.staged = true; } }
                }
                Tab::Snippets => {
                    snippets.iter_mut().for_each(|p| if p.id == target.id { p.staged = true; });
                }
                Tab::Canned => {
                    canned.iter_mut().for_each(|p| if p.id == target.id { p.staged = true; });
                }
                _ => {}
            }

            // Save all lists
            self.storage.save_project_prompts(&path, prompts).await?;
            self.storage.save_project_notes(&path, notes).await?;
            self.storage.save_project_archive(&path, archive).await?;
            self.storage.save_global_snippets(snippets.clone()).await?;
            self.storage.save_global_canned(canned).await?;

            // Process text before copying
            let processed_text = contracts::Processor::process(&target.text, &snippets);
            self.clipboard.copy(processed_text).await?;
            self.notify("Prompt staged and copied to clipboard!", ToastType::Success);
        }

        // Re-load current view
        self.load_prompts().await?;

        Ok(())
    }

    pub fn enter_editor(&mut self, text: String, id: Option<uuid::Uuid>) {
        self.mode = Mode::Editor;
        self.textarea = TextArea::new(text.lines().map(String::from).collect());
        
        let title = if self.active_tab == Tab::Snippets {
            if let Some(id) = id {
                self.prompts.iter().find(|p| p.id == id).and_then(|p| p.name.clone()).unwrap_or_default()
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        self.title_textarea = TextArea::new(vec![title]);
        self.title_focused = self.active_tab == Tab::Snippets;
        
        self.editing_id = id;
        self.insert_index = None;
        self.original_text = text;
    }

    pub fn enter_editor_before(&mut self, text: String, index: usize) {
        self.mode = Mode::Editor;
        self.textarea = TextArea::new(text.lines().map(String::from).collect());
        
        let title = String::new();
        self.title_textarea = TextArea::new(vec![title]);
        self.title_focused = self.active_tab == Tab::Snippets;

        self.editing_id = None;
        self.insert_index = Some(index);
        self.original_text = text;
    }

    pub fn exit_editor(&mut self) {
        self.mode = Mode::List;
        self.editing_id = None;
        self.insert_index = None;
        self.autocomplete_open = false;
        self.suggestions.clear();
        self.title_textarea = TextArea::default();
        self.title_focused = false;
    }

    pub fn edit_setting(&mut self) {
        if self.active_tab != Tab::Settings {
            return;
        }
        let tabs_len = Tab::all().len();
        let slash_len = self.settings.slash_commands.len();

        if self.selected_index >= tabs_len && self.selected_index < tabs_len + slash_len {
            // Edit existing Slash Command
            let idx = self.selected_index - tabs_len;
            let text = self.settings.slash_commands[idx].clone();
            self.mode = Mode::Editor;
            self.textarea = TextArea::new(vec![text.clone()]);
            self.title_textarea = TextArea::default();
            self.title_focused = false;
            self.editing_id = None; // We'll use selected_index to know which one
            self.original_text = text;
        } else if self.selected_index == tabs_len + slash_len {
            // Add New Slash Command
            self.mode = Mode::Editor;
            self.textarea = TextArea::default();
            self.title_textarea = TextArea::default();
            self.title_focused = false;
            self.editing_id = None;
            self.original_text = String::new();
        }
    }

    pub async fn save_editor(&mut self) -> contracts::Result<()> {
        let text = self.textarea.lines().join("\n");
        let path = self.current_project_path();

        if self.active_tab == Tab::Settings {
            let tabs_len = Tab::all().len();
            let slash_len = self.settings.slash_commands.len();

            let re = regex::Regex::new("^[a-zA-Z0-9_-]+$").unwrap();
            let trimmed = text.trim();
            if !trimmed.is_empty() && !re.is_match(trimmed) {
                self.notify("Slash command must match [a-zA-Z0-9_-]+", ToastType::Error);
                return Ok(());
            }

            if self.selected_index >= tabs_len && self.selected_index < tabs_len + slash_len {
                // Update existing
                let idx = self.selected_index - tabs_len;
                self.settings.slash_commands[idx] = trimmed.to_string();
                self.storage.save_settings(self.settings.clone()).await?;
            } else if self.selected_index == tabs_len + slash_len {
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

        let title = if self.active_tab == Tab::Snippets {
            let t = self.title_textarea.lines().join("");
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

        if let Some(id) = self.editing_id {
            match self.active_tab {
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
            let r#type = match self.active_tab {
                Tab::Notes => PromptType::Note,
                Tab::Snippets => PromptType::Snippet,
                _ => PromptType::Prompt,
            };
            
            let current_branch = self.git.get_current_branch(&path).await.unwrap_or_default();
            let prompt = Prompt::new(text, r#type, current_branch, title);
            
            match self.active_tab {
                Tab::Prompts => {
                    let mut list = self.storage.get_project_prompts(&path).await?;
                    if let Some(idx) = self.insert_index {
                        list.insert(idx, prompt);
                    } else {
                        list.push(prompt);
                    }
                    self.storage.save_project_prompts(&path, list).await?;
                }
                Tab::Notes => {
                    let mut list = self.storage.get_project_notes(&path).await?;
                    if let Some(idx) = self.insert_index {
                        list.insert(idx, prompt);
                    } else {
                        list.push(prompt);
                    }
                    self.storage.save_project_notes(&path, list).await?;
                }
                Tab::Canned => {
                    let mut list = self.storage.get_global_canned().await?;
                    if let Some(idx) = self.insert_index {
                        list.insert(idx, prompt);
                    } else {
                        list.push(prompt);
                    }
                    self.storage.save_global_canned(list).await?;
                }
                Tab::Snippets => {
                    let mut list = self.storage.get_global_snippets().await?;
                    if let Some(idx) = self.insert_index {
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
        if self.active_tab == Tab::Settings {
            let tabs_len = Tab::all().len();
            let slash_len = self.settings.slash_commands.len();
            if self.selected_index >= tabs_len && self.selected_index < tabs_len + slash_len {
                let idx = self.selected_index - tabs_len;
                self.settings.slash_commands.remove(idx);
                self.storage.save_settings(self.settings.clone()).await?;
                self.notify("Slash command deleted", ToastType::Warning);
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            return Ok(());
        }

        if self.prompts.is_empty() {
            return Ok(());
        }

        self.push_history();
        let path = self.current_project_path();
        let target = self.prompts[self.selected_index].clone();

        if self.active_tab == Tab::Archive {
            // Permanent delete
            let mut archive = self.storage.get_project_archive(&path).await?;
            archive.retain(|p| p.id != target.id);
            self.storage.save_project_archive(&path, archive).await?;
            self.notify("Prompt deleted permanently", ToastType::Warning);
        } else {
            // Move to archive
            // 1. Remove from original list
            match self.active_tab {
                Tab::Prompts => {
                    let mut list = self.storage.get_project_prompts(&path).await?;
                    list.retain(|p| p.id != target.id);
                    self.storage.save_project_prompts(&path, list).await?;
                }
                Tab::Notes => {
                    let mut list = self.storage.get_project_notes(&path).await?;
                    list.retain(|p| p.id != target.id);
                    self.storage.save_project_notes(&path, list).await?;
                }
                Tab::Canned => {
                    let mut list = self.storage.get_global_canned().await?;
                    list.retain(|p| p.id != target.id);
                    self.storage.save_global_canned(list).await?;
                }
                Tab::Snippets => {
                    let mut list = self.storage.get_global_snippets().await?;
                    list.retain(|p| p.id != target.id);
                    self.storage.save_global_snippets(list).await?;
                }
                _ => {}
            }

            // 2. Add to archive
            let mut archive = self.storage.get_project_archive(&path).await?;
            let mut archived_item = target;
            archived_item.staged = false;
            archive.insert(0, archived_item);
            self.storage.save_project_archive(&path, archive).await?;
            self.notify("Prompt moved to archive", ToastType::Info);
        }

        self.load_prompts().await?;
        Ok(())
    }

    pub async fn copy_selected(&mut self) -> contracts::Result<()> {
        if self.prompts.is_empty() {
            return Ok(());
        }

        let target = &self.prompts[self.selected_index];
        let snippets = self.storage.get_global_snippets().await?;
        
        let processed_text = contracts::Processor::process(&target.text, &snippets);
        self.clipboard.copy(processed_text).await?;
        self.notify("Copied to clipboard!", ToastType::Success);
        
        Ok(())
    }

    pub async fn restore_selected(&mut self) -> contracts::Result<()> {
        if self.active_tab != Tab::Archive || self.prompts.is_empty() {
            return Ok(());
        }

        self.push_history();
        let path = self.current_project_path();
        let target = self.prompts[self.selected_index].clone();

        // 1. Remove from Archive
        let mut archive = self.storage.get_project_archive(&path).await?;
        archive.retain(|p| p.id != target.id);
        self.storage.save_project_archive(&path, archive).await?;

        // 2. Add back to original list based on type
        match target.r#type {
            PromptType::Prompt => {
                let mut list = self.storage.get_project_prompts(&path).await?;
                list.push(target);
                self.storage.save_project_prompts(&path, list).await?;
            }
            PromptType::Note => {
                let mut list = self.storage.get_project_notes(&path).await?;
                list.push(target);
                self.storage.save_project_notes(&path, list).await?;
            }
            PromptType::Snippet => {
                let mut list = self.storage.get_global_snippets().await?;
                list.push(target);
                self.storage.save_global_snippets(list).await?;
            }
        }

        self.load_prompts().await?;
        self.notify("Prompt restored", ToastType::Success);
        Ok(())
    }

    pub async fn update_autocomplete(&mut self) -> contracts::Result<()> {
        let cursor = self.textarea.cursor();
        let row = cursor.0;
        let col = cursor.1;
        
        if row >= self.textarea.lines().len() {
            return Ok(());
        }
        
        let line = &self.textarea.lines()[row];
        if col > line.len() {
            return Ok(());
        }
        
        let before_cursor = &line[..col];

        // Find the last trigger before cursor
        let triggers = ["$$", "$", "@", "/"];
        let mut best_trigger = None;
        let mut best_pos = 0;

        for trigger in triggers {
            if let Some(pos) = before_cursor.rfind(trigger) {
                // Check if it's the latest trigger
                if best_trigger.is_none() || pos > best_pos {
                    // Special case for $$ vs $
                    if trigger == "$" && pos > 0 && &before_cursor[(pos - 1)..=pos] == "$$" {
                        continue;
                    }
                    best_trigger = Some(trigger);
                    best_pos = pos;
                }
            }
        }

        if let Some(trigger) = best_trigger {
            let query = &before_cursor[best_pos + trigger.len()..];
            
            // If there's a space after the trigger, we abort autocomplete.
            if query.contains(' ') {
                self.autocomplete_open = false;
                self.suggestions.clear();
                return Ok(());
            }

            let matcher = SkimMatcherV2::default();
            
            match trigger {
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
                    self.suggestions = scored_suggestions.into_iter().map(|(_, s)| s).collect();
                }
                "@" => {
                    let mut files = Vec::new();
                    let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                    self.walk_files(&current_dir, query, &mut files);
                    
                    let mut scored_suggestions: Vec<(i64, Prompt)> = files
                        .into_iter()
                        .filter_map(|f| {
                            let text = f.name.as_deref().unwrap_or(&f.text);
                            matcher.fuzzy_match(text, query).map(|score| (score, f))
                        })
                        .collect();
                    
                    scored_suggestions.sort_by_key(|b| std::cmp::Reverse(b.0));
                    self.suggestions = scored_suggestions.into_iter().map(|(_, f)| f).collect();
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
                    self.suggestions = scored_suggestions.into_iter().map(|(_, s)| s).collect();
                }
                _ => self.suggestions = Vec::new(),
            }
            
            if self.suggestions.is_empty() {
                self.autocomplete_open = false;
            } else {
                self.autocomplete_open = true;
                if self.suggestion_index >= self.suggestions.len() {
                    self.suggestion_index = 0;
                }
            }
        } else {
            self.autocomplete_open = false;
        }

        Ok(())
    }

    pub const fn move_suggestion_down(&mut self) {
        if !self.suggestions.is_empty() && self.suggestion_index < self.suggestions.len() - 1 {
            self.suggestion_index += 1;
        }
    }

    pub const fn move_suggestion_up(&mut self) {
        if self.suggestion_index > 0 {
            self.suggestion_index -= 1;
        }
    }

    pub fn select_suggestion(&mut self) {
        if !self.suggestions.is_empty() && self.autocomplete_open {
            let snippet = &self.suggestions[self.suggestion_index];
            let name = snippet.name.as_deref().unwrap_or(&snippet.text);
            
            let cursor = self.textarea.cursor();
            let row = cursor.0;
            let col = cursor.1;
            let line = self.textarea.lines()[row].clone();
            let before_cursor = &line[..col];
            
            let triggers = ["$$", "$", "@", "/"];
            let mut best_trigger = None;
            let mut best_pos = 0;

            for trigger in triggers {
                if let Some(pos) = before_cursor.rfind(trigger) {
                    if best_trigger.is_none() || pos > best_pos {
                        if trigger == "$" && pos > 0 && &before_cursor[(pos - 1)..=pos] == "$$" {
                            continue;
                        }
                        best_trigger = Some(trigger);
                        best_pos = pos;
                    }
                }
            }

            if let Some(trigger) = best_trigger {
                let replacement = match trigger {
                    "$$" => format!("$${name}"),
                    "$" => snippet.text.clone(),
                    "@" => name.to_string(),
                    "/" => format!("/{name}"),
                    _ => name.to_string(),
                };

                let mut new_line = line[..best_pos].to_string();
                new_line.push_str(&replacement);
                new_line.push_str(&line[col..]);
                
                let new_col = best_pos + replacement.len();
                
                // This is a bit hacky with ratatui-textarea but works for simple cases
                self.textarea.move_cursor(ratatui_textarea::CursorMove::Jump(row as u16, 0));
                self.textarea.delete_line_by_end();
                self.textarea.insert_str(&new_line);
                self.textarea.move_cursor(ratatui_textarea::CursorMove::Jump(row as u16, new_col as u16));
            }
            
            self.autocomplete_open = false;
            self.suggestions.clear();
            self.suggestion_index = 0;
        }
    }

    pub async fn toggle_setting(&mut self) -> contracts::Result<()> {
        if self.active_tab != Tab::Settings {
            return Ok(());
        }

        let tabs = Tab::all();
        if self.selected_index < tabs.len() {
            let tab = tabs[self.selected_index];
            let current = self.settings.tab_visibility.get(&tab).copied().unwrap_or(true);
            self.settings.tab_visibility.insert(tab, !current);
            self.storage.save_settings(self.settings.clone()).await?;
            self.notify(format!("Toggled visibility for {tab:?}"), ToastType::Info);
        } else if self.selected_index >= tabs.len() && self.selected_index < tabs.len() + self.settings.slash_commands.len() + 1 {
             // Slash commands - maybe edit?
        } else if self.selected_index == tabs.len() + self.settings.slash_commands.len() + 1 {
            self.settings.enable_claude_commands = !self.settings.enable_claude_commands;
            self.storage.save_settings(self.settings.clone()).await?;
            self.notify(format!("Claude commands: {}", if self.settings.enable_claude_commands { "ON" } else { "OFF" }), ToastType::Info);
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
        
        self.prompts = results;
        self.selected_index = 0;
        self.notify(format!("Global search found {} results", self.prompts.len()), ToastType::Info);
        Ok(())
    }

    fn walk_files(&self, dir: &std::path::Path, query: &str, results: &mut Vec<Prompt>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name == "target" || name == ".git" || name == "node_modules" {
                            continue;
                        }
                    }
                    self.walk_files(&path, query, results);
                } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.contains(query) {
                        results.push(Prompt::new(
                            path.to_string_lossy().to_string(),
                            PromptType::Note,
                            None,
                            Some(name.to_string()),
                        ));
                    }
                }
                if results.len() > 20 { break; }
            }
        }
    }
}
