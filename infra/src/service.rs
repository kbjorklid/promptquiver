use async_trait::async_trait;
use contracts::{AppService, Clipboard, Prompt, Result, Storage, Tab, Tab::*, Processor};
use std::sync::Arc;
use uuid;
use chrono;

pub struct RealAppService {
    storage: Arc<dyn Storage>,
    clipboard: Arc<dyn Clipboard>,
}

impl RealAppService {
    pub fn new(storage: Arc<dyn Storage>, clipboard: Arc<dyn Clipboard>) -> Self {
        Self { storage, clipboard }
    }

    async fn clear_all_last_copied(&self, project_path: &str) -> Result<()> {
        let mut prompts = self.storage.get_project_prompts(project_path).await?;
        let mut notes = self.storage.get_project_notes(project_path).await?;
        let mut snippets = self.storage.get_global_snippets().await?;
        let mut canned = self.storage.get_global_canned().await?;
        let mut archive = self.storage.get_project_archive(project_path).await?;

        let mut changed = false;
        for p in &mut prompts { if p.last_copied { p.last_copied = false; changed = true; } }
        for p in &mut notes { if p.last_copied { p.last_copied = false; changed = true; } }
        for p in &mut snippets { if p.last_copied { p.last_copied = false; changed = true; } }
        for p in &mut canned { if p.last_copied { p.last_copied = false; changed = true; } }
        for p in &mut archive { if p.last_copied { p.last_copied = false; changed = true; } }

        if changed {
            self.storage.save_project_prompts(project_path, prompts).await?;
            self.storage.save_project_notes(project_path, notes).await?;
            self.storage.save_global_snippets(snippets).await?;
            self.storage.save_global_canned(canned).await?;
            self.storage.save_project_archive(project_path, archive).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl AppService for RealAppService {
    async fn stage_item(&self, project_path: &str, tab: Tab, item: Prompt) -> Result<()> {
        if tab == Settings {
            return Ok(());
        }

        // Alias for Notes and Snippets: they cannot be staged anymore
        if tab == Notes || tab == Snippets {
            let snippets = self.storage.get_global_snippets().await?;
            let processed_text = Processor::process(&item.text, &snippets);
            self.clipboard.copy(processed_text).await?;
            return Ok(());
        }

        if item.staged {
            // Un-stage
            let mut current_list = match tab {
                Prompts => self.storage.get_project_prompts(project_path).await?,
                Canned => self.storage.get_global_canned().await?,
                _ => return Ok(()),
            };

            if let Some(p) = current_list.iter_mut().find(|p| p.id == item.id) {
                p.staged = false;
            }

            match tab {
                Prompts => self.storage.save_project_prompts(project_path, current_list).await?,
                Canned => self.storage.save_global_canned(current_list).await?,
                _ => {},
            }
        } else {
            // Stage
            let mut prompts = self.storage.get_project_prompts(project_path).await?;
            let mut archive = self.storage.get_project_archive(project_path).await?;
            let mut canned = self.storage.get_global_canned().await?;

            let mut to_archive = Vec::new();

            for p in &mut prompts {
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

            // Add to archive (to the top)
            for mut p in to_archive {
                p.staged = false;
                archive.insert(0, p);
            }

            // Set target to staged
            match tab {
                Prompts => {
                    if let Some(p) = prompts.iter_mut().find(|p| p.id == item.id) {
                        p.staged = true;
                    }
                }
                Canned => {
                    if let Some(p) = canned.iter_mut().find(|p| p.id == item.id) {
                        p.staged = true;
                    }
                }
                _ => {}
            }

            // Save affected lists
            self.storage.save_project_prompts(project_path, prompts).await?;
            self.storage.save_project_archive(project_path, archive).await?;
            self.storage.save_global_canned(canned).await?;

            // Clear last_copied for all when staging
            self.clear_all_last_copied(project_path).await?;

            // Process text before copying
            let snippets = self.storage.get_global_snippets().await?;
            let processed_text = Processor::process(&item.text, &snippets);
            self.clipboard.copy(processed_text).await?;
        }

        Ok(())
    }

    async fn archive_item(&self, project_path: &str, tab: Tab, item: Prompt) -> Result<()> {
        if tab == Settings {
            return Ok(());
        }

        if tab == Archive {
            // Permanent delete
            let mut archive = self.storage.get_project_archive(project_path).await?;
            archive.retain(|p| p.id != item.id);
            self.storage.save_project_archive(project_path, archive).await?;
        } else {
            // Move to archive
            let mut current_list = match tab {
                Prompts => self.storage.get_project_prompts(project_path).await?,
                Notes => self.storage.get_project_notes(project_path).await?,
                Canned => self.storage.get_global_canned().await?,
                Snippets => self.storage.get_global_snippets().await?,
                _ => return Ok(()),
            };

            if let Some(mut p) = current_list.iter().find(|p| p.id == item.id).cloned() {
                current_list.retain(|p| p.id != item.id);
                p.staged = false;

                let mut archive = self.storage.get_project_archive(project_path).await?;
                archive.insert(0, p);

                self.storage.save_project_archive(project_path, archive).await?;
                
                match tab {
                    Prompts => self.storage.save_project_prompts(project_path, current_list).await?,
                    Notes => self.storage.save_project_notes(project_path, current_list).await?,
                    Canned => self.storage.save_global_canned(current_list).await?,
                    Snippets => self.storage.save_global_snippets(current_list).await?,
                    _ => {},
                }
            }
        }
        Ok(())
    }

    async fn restore_item(&self, project_path: &str, item: Prompt) -> Result<()> {
        let mut archive = self.storage.get_project_archive(project_path).await?;
        if let Some(p) = archive.iter().find(|p| p.id == item.id).cloned() {
            archive.retain(|p| p.id != item.id);
            self.storage.save_project_archive(project_path, archive).await?;

            match p.r#type {
                contracts::PromptType::Prompt => {
                    let mut prompts = self.storage.get_project_prompts(project_path).await?;
                    prompts.insert(0, p);
                    self.storage.save_project_prompts(project_path, prompts).await?;
                }
                contracts::PromptType::Note => {
                    let mut notes = self.storage.get_project_notes(project_path).await?;
                    notes.insert(0, p);
                    self.storage.save_project_notes(project_path, notes).await?;
                }
                contracts::PromptType::Snippet => {
                    let mut snippets = self.storage.get_global_snippets().await?;
                    snippets.insert(0, p);
                    self.storage.save_global_snippets(snippets).await?;
                }
            }
        }
        Ok(())
    }

    async fn duplicate_item(&self, project_path: &str, tab: Tab, item: Prompt) -> Result<Option<Prompt>> {
        if tab == Settings {
            return Ok(None);
        }

        let mut p = item.clone();
        p.id = uuid::Uuid::new_v4();
        p.staged = false;
        p.created_at = chrono::Utc::now();
        p.updated_at = p.created_at;

        let mut current_list = match tab {
            Prompts => self.storage.get_project_prompts(project_path).await?,
            Notes => self.storage.get_project_notes(project_path).await?,
            Archive => self.storage.get_project_archive(project_path).await?,
            Canned => self.storage.get_global_canned().await?,
            Snippets => self.storage.get_global_snippets().await?,
            _ => return Ok(None),
        };

        // Find index of original item to insert after it
        if let Some(pos) = current_list.iter().position(|i| i.id == item.id) {
            current_list.insert(pos + 1, p.clone());
        } else {
            current_list.push(p.clone());
        }

        match tab {
            Prompts => self.storage.save_project_prompts(project_path, current_list).await?,
            Notes => self.storage.save_project_notes(project_path, current_list).await?,
            Archive => self.storage.save_project_archive(project_path, current_list).await?,
            Canned => self.storage.save_global_canned(current_list).await?,
            Snippets => self.storage.save_global_snippets(current_list).await?,
            _ => {},
        }

        Ok(Some(p))
    }

    async fn copy_item(&self, project_path: &str, tab: Tab, item: Prompt) -> Result<()> {
        if tab == Settings {
            return Ok(());
        }

        // 1. Clear all
        self.clear_all_last_copied(project_path).await?;

        // 2. Mark current as last_copied in its original list
        match tab {
            Prompts => {
                let mut list = self.storage.get_project_prompts(project_path).await?;
                if let Some(p) = list.iter_mut().find(|p| p.id == item.id) { p.last_copied = true; }
                self.storage.save_project_prompts(project_path, list).await?;
            }
            Notes => {
                let mut list = self.storage.get_project_notes(project_path).await?;
                if let Some(p) = list.iter_mut().find(|p| p.id == item.id) { p.last_copied = true; }
                self.storage.save_project_notes(project_path, list).await?;
            }
            Canned => {
                let mut list = self.storage.get_global_canned().await?;
                if let Some(p) = list.iter_mut().find(|p| p.id == item.id) { p.last_copied = true; }
                self.storage.save_global_canned(list).await?;
            }
            Snippets => {
                let mut list = self.storage.get_global_snippets().await?;
                if let Some(p) = list.iter_mut().find(|p| p.id == item.id) { p.last_copied = true; }
                self.storage.save_global_snippets(list).await?;
            }
            Archive => {
                let mut list = self.storage.get_project_archive(project_path).await?;
                if let Some(p) = list.iter_mut().find(|p| p.id == item.id) { p.last_copied = true; }
                self.storage.save_project_archive(project_path, list).await?;
            }
            _ => {}
        }

        // 3. Process and copy
        let snippets = self.storage.get_global_snippets().await?;
        let processed_text = Processor::process(&item.text, &snippets);
        self.clipboard.copy(processed_text).await?;

        Ok(())
    }
}
