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
    pub project: Option<String>,
    pub branch: Option<String>,
    pub name: Option<String>,
    pub staged: bool,
    pub last_copied: bool,
    pub is_archived: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Prompt {
    pub fn new(
        text: String,
        r#type: PromptType,
        folder: Option<String>,
        branch: Option<String>,
        name: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            text,
            r#type,
            folder,
            project: None,
            branch,
            name,
            staged: false,
            last_copied: false,
            is_archived: false,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum PreviewMode {
    #[default]
    Bottom,
    Side,
    Hidden,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    pub tab_visibility: HashMap<Tab, bool>,
    pub slash_commands: Vec<String>,
    pub enable_claude_commands: bool,
    pub use_nerd_font: bool,
    pub theme_name: Option<String>,
    pub preview_mode: PreviewMode,
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
pub use service::AppService;

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
    pub project: Option<String>,
    pub branch: Option<String>,
    pub tab: Option<Tab>,
}

#[async_trait]
pub trait Storage: Send + Sync {
    async fn get_prompts(&self, filter: PromptFilter) -> Result<Vec<Prompt>>;
    async fn save_prompt(&self, prompt: Prompt) -> Result<()>;
    async fn delete_prompt(&self, id: Uuid) -> Result<()>;

    async fn get_project_info(&self, folder: &str) -> Result<ProjectInfo>;
    async fn save_project_info(&self, folder: &str, info: ProjectInfo) -> Result<()>;

    async fn get_settings(&self) -> Result<Settings>;
    async fn save_settings(&self, settings: Settings) -> Result<()>;
}


#[async_trait]
pub trait Clipboard: Send + Sync {
    async fn copy(&self, text: String) -> Result<()>;
    async fn paste(&self) -> Result<String>;
}

#[async_trait]
pub trait Git: Send + Sync {
    async fn get_current_branch(&self, path: &str) -> Result<Option<String>>;
}
