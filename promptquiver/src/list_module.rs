use contracts::{Prompt, Tab, Storage, Result};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub tab: Tab,
    pub prompts: Vec<Prompt>,
}

#[derive(Debug)]
pub struct ListModule {
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

impl Default for ListModule {
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

impl ListModule {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next_tab(&mut self) {
        self.active_tab = self.active_tab.next();
        self.selected_index = 0;
        self.list_state.select(Some(0));
    }

    pub fn prev_tab(&mut self) {
        self.active_tab = self.active_tab.prev();
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
            let tabs_len = Tab::all().len();
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
                let tabs_len = Tab::all().len();
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
            let tabs_len = Tab::all().len();
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
        let path = self.current_path.clone();
        match self.active_tab {
            Tab::Prompts => storage.save_project_prompts(&path, self.prompts.clone()).await?,
            Tab::Notes => storage.save_project_notes(&path, self.prompts.clone()).await?,
            Tab::Archive => storage.save_project_archive(&path, self.prompts.clone()).await?,
            Tab::Canned => storage.save_global_canned(self.prompts.clone()).await?,
            Tab::Snippets => storage.save_global_snippets(self.prompts.clone()).await?,
            Tab::Settings => {}
        }
        Ok(())
    }

    pub fn current_project_path(&self) -> String {
        self.current_path.clone()
    }
}
