use contracts::{Prompt, Result, Storage, Tab};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub tab: Tab,
    pub prompts: Vec<Prompt>,
}

#[derive(Debug, Default)]
pub struct HistoryManager {
    pub undo_stack: Vec<HistoryEntry>,
    pub redo_stack: Vec<HistoryEntry>,
}

impl HistoryManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, tab: Tab, prompts: Vec<Prompt>) {
        let entry = HistoryEntry { tab, prompts };
        self.undo_stack.push(entry);
        self.redo_stack.clear();

        // Limit stack size
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
    }

    /// Undoes the last change.
    ///
    /// # Errors
    /// Returns an error if the storage cannot be accessed when saving the restored state.
    pub async fn undo(
        &mut self,
        current_tab: Tab,
        current_prompts: Vec<Prompt>,
        storage: &Arc<dyn Storage>,
    ) -> Result<Option<HistoryEntry>> {
        if let Some(entry) = self.undo_stack.pop() {
            let current = HistoryEntry { tab: current_tab, prompts: current_prompts };
            self.redo_stack.push(current);

            self.save_list(&entry.prompts, storage).await?;
            return Ok(Some(entry));
        }
        Ok(None)
    }

    /// Redoes the last undone change.
    ///
    /// # Errors
    /// Returns an error if the storage cannot be accessed when saving the restored state.
    pub async fn redo(
        &mut self,
        current_tab: Tab,
        current_prompts: Vec<Prompt>,
        storage: &Arc<dyn Storage>,
    ) -> Result<Option<HistoryEntry>> {
        if let Some(entry) = self.redo_stack.pop() {
            let current = HistoryEntry { tab: current_tab, prompts: current_prompts };
            self.undo_stack.push(current);

            self.save_list(&entry.prompts, storage).await?;
            return Ok(Some(entry));
        }
        Ok(None)
    }

    async fn save_list(&self, prompts: &[Prompt], storage: &Arc<dyn Storage>) -> Result<()> {
        let mut prompts = prompts.to_vec();
        for (i, p) in prompts.iter_mut().enumerate() {
            p.order_index = i32::try_from(i).unwrap_or(i32::MAX);
        }
        storage.save_prompts(prompts).await?;
        Ok(())
    }
}
