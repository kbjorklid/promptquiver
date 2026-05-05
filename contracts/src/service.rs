use crate::{Prompt, Result, Tab};
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct SaveItemArgs {
    pub project_path: String,
    pub tab: Tab,
    pub text: String,
    pub title: Option<String>,
    pub id: Option<uuid::Uuid>,
    pub insert_index: Option<usize>,
    pub branch: Option<String>,
    pub project_id: Option<uuid::Uuid>,
}

#[async_trait]
pub trait AppService: Send + Sync {
    /// Stages an item and copies it to the clipboard.
    ///
    /// # Errors
    /// Returns an error if the item cannot be staged or copied.
    async fn stage_item(&self, project_path: &str, tab: Tab, item: Prompt) -> Result<()>;

    /// Archives an item or deletes it permanently if already in Archive.
    ///
    /// # Errors
    /// Returns an error if the item cannot be archived or deleted.
    async fn archive_item(&self, project_path: &str, tab: Tab, item: Prompt) -> Result<()>;

    /// Restores an item from the archive.
    ///
    /// # Errors
    /// Returns an error if the item cannot be restored.
    async fn restore_item(&self, project_path: &str, item: Prompt) -> Result<()>;

    /// Duplicates an item.
    ///
    /// # Errors
    /// Returns an error if the item cannot be duplicated.
    async fn duplicate_item(&self, project_path: &str, tab: Tab, item: Prompt) -> Result<Option<Prompt>>;

    /// Copies an item's processed text to the clipboard.
    ///
    /// # Errors
    /// Returns an error if the item cannot be copied.
    async fn copy_item(&self, project_path: &str, tab: Tab, item: Prompt) -> Result<()>;

    /// Saves an item (new or existing).
    ///
    /// # Errors
    /// Returns an error if the item cannot be saved.
    async fn save_item(&self, args: SaveItemArgs) -> Result<uuid::Uuid>;

    /// Searches for files in the base directory matching the query.
    ///
    /// # Errors
    /// Returns an error if the search fails.
    async fn search_files(&self, base_dir: &str, query: &str) -> Result<Vec<Prompt>>;

    /// Exports all application data to a TOML string.
    ///
    /// # Errors
    /// Returns an error if the data cannot be exported.
    async fn export_data(&self, include_archived: bool) -> Result<String>;

    /// Imports application data from a TOML string.
    ///
    /// # Errors
    /// Returns an error if the data cannot be imported.
    async fn import_data(&self, toml_data: &str) -> Result<()>;
}
