use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PromptType {
    #[serde(rename = "prompt")]
    Prompt,
    #[serde(rename = "note")]
    Note,
    #[serde(rename = "snippet")]
    Snippet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    pub id: Uuid,
    pub text: String,
    pub r#type: PromptType,
    pub folder: Option<String>,
    pub project_id: Option<Uuid>,
    pub branch: Option<String>,
    pub name: Option<String>,
    pub staged: bool,
    pub last_copied: bool,
    pub is_archived: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub order_index: i32,
}

impl Prompt {
    pub fn new(
        text: String,
        r#type: PromptType,
        folder: Option<String>,
        branch: Option<String>,
        name: Option<String>,
        project_id: Option<Uuid>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            text,
            r#type,
            folder,
            project_id,
            branch,
            name,
            staged: false,
            last_copied: false,
            is_archived: false,
            created_at: now,
            updated_at: now,
            order_index: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Project {
    pub id: Uuid,
    pub title: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum PreviewMode {
    #[default]
    Bottom,
    Side,
    Hidden,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum StartupBehavior {
    #[default]
    Ask,
    LastActivated,
    Specific,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct Settings {
    pub tab_visibility: HashMap<Tab, bool>,
    pub slash_commands: Vec<String>,
    pub enable_claude_commands: bool,
    #[serde(default)]
    pub enable_claude_builtin_commands: bool,
    pub use_nerd_font: bool,
    pub theme_name: Option<String>,
    pub preview_mode: PreviewMode,
    pub startup_behavior: StartupBehavior,
    pub last_active_project_id: Option<Uuid>,
    pub specific_project_id: Option<Uuid>,
    #[serde(default)]
    pub show_wide_view: bool,
}

impl Settings {
    pub fn visible_tabs(&self) -> Vec<Tab> {
        Tab::all()
            .into_iter()
            .filter(|t| {
                if *t == Tab::Settings {
                    return true;
                }
                *self.tab_visibility.get(t).unwrap_or(&true)
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectInfo {
    pub path: String,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Clipboard error: {0}")]
    Clipboard(String),
    #[error("Git error: {0}")]
    Git(String),
    #[error("Not found")]
    NotFound,
}

pub type Result<T> = std::result::Result<T, Error>;

pub mod processor;
pub use processor::Processor;

pub mod service;
pub use service::{AppService, SaveItemArgs};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tab {
    Prompts,
    Canned,
    Notes,
    Snippets,
    Archive,
    Settings,
}

impl Tab {
    pub fn all() -> Vec<Self> {
        vec![
            Self::Prompts,
            Self::Canned,
            Self::Notes,
            Self::Snippets,
            Self::Archive,
            Self::Settings,
        ]
    }

    pub fn settings_display_len() -> usize {
        Self::all().into_iter().filter(|&t| t != Self::Settings).count()
    }
}

#[derive(Debug, Clone, Default)]
pub struct PromptFilter {
    pub folder: Option<String>,
    pub project_id: Option<Uuid>,
    pub branch: Option<String>,
    pub tab: Option<Tab>,
    pub project_filter: bool,
    pub staged: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseExport {
    pub prompts: Vec<Prompt>,
    pub projects: Vec<Project>,
    pub project_info: HashMap<String, ProjectInfo>,
    pub settings: Settings,
}

#[async_trait]
pub trait Storage: Send + Sync {
    /// Gets prompts matching the filter.
    ///
    /// # Errors
    /// Returns a `Storage` error if the data cannot be retrieved.
    async fn get_prompts(&self, filter: PromptFilter) -> Result<Vec<Prompt>>;

    /// Saves a single prompt.
    ///
    /// # Errors
    /// Returns a `Storage` error if the prompt cannot be saved.
    async fn save_prompt(&self, prompt: Prompt) -> Result<()>;

    /// Saves multiple prompts.
    ///
    /// # Errors
    /// Returns a `Storage` error if the prompts cannot be saved.
    async fn save_prompts(&self, prompts: Vec<Prompt>) -> Result<()>;

    /// Deletes a prompt by ID.
    ///
    /// # Errors
    /// Returns a `Storage` error if the prompt cannot be deleted.
    async fn delete_prompt(&self, id: Uuid) -> Result<()>;

    /// Gets all projects.
    ///
    /// # Errors
    /// Returns a `Storage` error if projects cannot be retrieved.
    async fn get_projects(&self) -> Result<Vec<Project>>;

    /// Saves a project.
    ///
    /// # Errors
    /// Returns a `Storage` error if the project cannot be saved.
    async fn save_project(&self, project: Project) -> Result<()>;

    /// Deletes a project and disassociates its prompts.
    ///
    /// # Errors
    /// Returns a `Storage` error if the project cannot be deleted.
    async fn delete_project(&self, id: Uuid) -> Result<()>;

    /// Gets project info for a folder.
    ///
    /// # Errors
    /// Returns a `Storage` error if project info cannot be retrieved.
    async fn get_project_info(&self, folder: &str) -> Result<ProjectInfo>;

    /// Saves project info for a folder.
    ///
    /// # Errors
    /// Returns a `Storage` error if project info cannot be saved.
    async fn save_project_info(&self, folder: &str, info: ProjectInfo) -> Result<()>;

    /// Gets application settings.
    ///
    /// # Errors
    /// Returns a `Storage` error if settings cannot be retrieved.
    async fn get_settings(&self) -> Result<Settings>;

    /// Saves application settings.
    ///
    /// # Errors
    /// Returns a `Storage` error if settings cannot be saved.
    async fn save_settings(&self, settings: Settings) -> Result<()>;

    /// Gets the data version (for synchronization).
    ///
    /// # Errors
    /// Returns a `Storage` error if the version cannot be retrieved.
    async fn get_data_version(&self) -> Result<u32>;

    /// Gets all data for export.
    ///
    /// # Errors
    /// Returns a `Storage` error if data cannot be retrieved.
    async fn get_all_data(&self) -> Result<DatabaseExport>;

    /// Restores all data from an export.
    ///
    /// # Errors
    /// Returns a `Storage` error if data cannot be restored.
    async fn restore_all_data(&self, data: DatabaseExport) -> Result<()>;
}

#[async_trait]
pub trait Clipboard: Send + Sync {
    /// Copies text to the system clipboard.
    ///
    /// # Errors
    /// Returns a `Clipboard` error if the copy operation fails.
    async fn copy(&self, text: String) -> Result<()>;

    /// Pastes text from the system clipboard.
    ///
    /// # Errors
    /// Returns a `Clipboard` error if the paste operation fails.
    async fn paste(&self) -> Result<String>;
}

#[async_trait]
pub trait Git: Send + Sync {
    /// Gets the current branch name for a given path.
    ///
    /// # Errors
    /// Returns a `Git` error if the branch name cannot be retrieved.
    async fn get_current_branch(&self, path: &str) -> Result<Option<String>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn show_wide_view_defaults_to_false() {
        let s = Settings::default();
        assert!(!s.show_wide_view);
    }
}
