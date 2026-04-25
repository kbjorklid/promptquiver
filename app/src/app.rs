use contracts::{Clipboard, Git, Prompt, PromptType, Storage, Tab};
use ratatui_textarea::TextArea;
use ratatui_toaster::{ToastBuilder, ToastType, ToastEngine, ToastMessage, ToastPosition};
use std::sync::Arc;

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    List,
    Editor,
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
    pub editing_id: Option<uuid::Uuid>,
    pub current_branch: Option<String>,
    pub autocomplete_open: bool,
    pub suggestions: Vec<Prompt>,
    pub suggestion_index: usize,
    pub toaster: Option<ToastEngine<ToastMessage>>,
}


impl<'a> fmt::Debug for App<'a> {
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

impl<'a> App<'a> {
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
            editing_id: None,
            current_branch: None,
            autocomplete_open: false,
            suggestions: Vec::new(),
            suggestion_index: 0,
            toaster: None,
        }
    }

    pub fn tick(&mut self) {
        // Handle background tasks or state updates
    }

    pub fn notify(&mut self, message: impl Into<String>, kind: ToastType) {
        if let Some(ref mut toaster) = self.toaster {
            let toast = ToastBuilder::new(message.into().into())
                .toast_type(kind)
                .position(ToastPosition::BottomRight);
            toaster.show_toast(toast);
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

    pub fn set_tab(&mut self, tab: Tab) {
        self.active_tab = tab;
    }

    pub fn move_down(&mut self) {
        if !self.prompts.is_empty() && self.selected_index < self.prompts.len() - 1 {
            self.selected_index += 1;
        }
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub async fn load_prompts(&mut self) -> contracts::Result<()> {
        let path = self.current_project_path();
        
        self.prompts = match self.active_tab {
            Tab::Prompts => self.storage.get_project_prompts(&path).await?,
            Tab::Notes => self.storage.get_project_notes(&path).await?,
            Tab::Archive => self.storage.get_project_archive(&path).await?,
            Tab::Canned => self.storage.get_global_canned().await?,
            Tab::Snippets => self.storage.get_global_snippets().await?,
            Tab::Settings => Vec::new(),
        };
        
        if self.selected_index >= self.prompts.len() && !self.prompts.is_empty() {
            self.selected_index = self.prompts.len() - 1;
        }
        
        Ok(())
    }

    fn current_project_path(&self) -> String {
        std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    }

    pub async fn stage_selected(&mut self) -> contracts::Result<()> {
        if self.prompts.is_empty() {
            return Ok(());
        }

        let path = self.current_project_path();
        let target_idx = self.selected_index;
        let is_staged = self.prompts[target_idx].staged;

        if is_staged {
            // Un-stage
            self.prompts[target_idx].staged = false;
            self.notify("Prompt un-staged", ToastType::Info);
        } else {
            // Stage
            // 1. Un-stage and Archive others
            let mut prompts = self.storage.get_project_prompts(&path).await?;
            let mut notes = self.storage.get_project_notes(&path).await?;
            let mut snippets = self.storage.get_global_snippets().await?;
            let mut archive = self.storage.get_project_archive(&path).await?;

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
                    notes.iter_mut().for_each(|p| if p.id == target.id { p.staged = true; });
                }
                Tab::Snippets => {
                    snippets.iter_mut().for_each(|p| if p.id == target.id { p.staged = true; });
                }
                Tab::Canned => {
                    let mut canned = self.storage.get_global_canned().await?;
                    canned.iter_mut().for_each(|p| if p.id == target.id { p.staged = true; });
                    self.storage.save_global_canned(canned).await?;
                }
                _ => {}
            }

            // Save all lists
            self.storage.save_project_prompts(&path, prompts).await?;
            self.storage.save_project_notes(&path, notes).await?;
            self.storage.save_project_archive(&path, archive).await?;
            self.storage.save_global_snippets(snippets.clone()).await?;

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
        self.editing_id = id;
    }

    pub fn exit_editor(&mut self) {
        self.mode = Mode::List;
        self.editing_id = None;
        self.autocomplete_open = false;
        self.suggestions.clear();
    }

    pub async fn save_editor(&mut self) -> contracts::Result<()> {
        let text = self.textarea.lines().join("\n");
        let path = self.current_project_path();

        if let Some(id) = self.editing_id {
            // Edit existing
            match self.active_tab {
                Tab::Prompts => {
                    let mut list = self.storage.get_project_prompts(&path).await?;
                    if let Some(p) = list.iter_mut().find(|p| p.id == id) {
                        p.text = text;
                        p.updated_at = chrono::Utc::now();
                    }
                    self.storage.save_project_prompts(&path, list).await?;
                }
                Tab::Notes => {
                    let mut list = self.storage.get_project_notes(&path).await?;
                    if let Some(p) = list.iter_mut().find(|p| p.id == id) {
                        p.text = text;
                        p.updated_at = chrono::Utc::now();
                    }
                    self.storage.save_project_notes(&path, list).await?;
                }
                Tab::Canned => {
                    let mut list = self.storage.get_global_canned().await?;
                    if let Some(p) = list.iter_mut().find(|p| p.id == id) {
                        p.text = text;
                        p.updated_at = chrono::Utc::now();
                    }
                    self.storage.save_global_canned(list).await?;
                }
                Tab::Snippets => {
                    let mut list = self.storage.get_global_snippets().await?;
                    if let Some(p) = list.iter_mut().find(|p| p.id == id) {
                        p.text = text;
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
            let prompt = Prompt::new(text, r#type, None, None);
            
            match self.active_tab {
                Tab::Prompts => {
                    let mut list = self.storage.get_project_prompts(&path).await?;
                    list.push(prompt);
                    self.storage.save_project_prompts(&path, list).await?;
                }
                Tab::Notes => {
                    let mut list = self.storage.get_project_notes(&path).await?;
                    list.push(prompt);
                    self.storage.save_project_notes(&path, list).await?;
                }
                Tab::Canned => {
                    let mut list = self.storage.get_global_canned().await?;
                    list.push(prompt);
                    self.storage.save_global_canned(list).await?;
                }
                Tab::Snippets => {
                    let mut list = self.storage.get_global_snippets().await?;
                    list.push(prompt);
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
        if self.prompts.is_empty() {
            return Ok(());
        }

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
            // 1. Remove from current list
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
                    if trigger == "$" && pos > 0 && &before_cursor[pos-1..pos+1] == "$$" {
                        continue;
                    }
                    best_trigger = Some(trigger);
                    best_pos = pos;
                }
            }
        }

        if let Some(trigger) = best_trigger {
            let query = &before_cursor[best_pos + trigger.len()..];
            
            match trigger {
                "$" | "$$" => {
                    let snippets = self.storage.get_global_snippets().await?;
                    self.suggestions = snippets
                        .into_iter()
                        .filter(|s| s.name.as_deref().unwrap_or(&s.text).contains(query))
                        .collect();
                }
                "@" => {
                    // File search - mock for now or implement basic walk
                    self.suggestions = Vec::new(); 
                }
                "/" => {
                    // Slash commands - mock for now
                    self.suggestions = Vec::new();
                }
                _ => self.suggestions = Vec::new(),
            }
            
            if !self.suggestions.is_empty() {
                self.autocomplete_open = true;
                if self.suggestion_index >= self.suggestions.len() {
                    self.suggestion_index = 0;
                }
            } else {
                self.autocomplete_open = false;
            }
        } else {
            self.autocomplete_open = false;
        }

        Ok(())
    }

    pub fn move_suggestion_down(&mut self) {
        if !self.suggestions.is_empty() && self.suggestion_index < self.suggestions.len() - 1 {
            self.suggestion_index += 1;
        }
    }

    pub fn move_suggestion_up(&mut self) {
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
                        if trigger == "$" && pos > 0 && &before_cursor[pos-1..pos+1] == "$$" {
                            continue;
                        }
                        best_trigger = Some(trigger);
                        best_pos = pos;
                    }
                }
            }

            if let Some(trigger) = best_trigger {
                let replacement = match trigger {
                    "$$" => format!("$${}", name),
                    "$" => snippet.text.clone(),
                    "@" => name.to_string(),
                    "/" => format!("/{}", name),
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
}
