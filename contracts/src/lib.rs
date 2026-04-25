use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

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
    pub branch: Option<String>,
    pub name: Option<String>,
    pub staged: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Prompt {
    pub fn new(
        text: String,
        r#type: PromptType,
        branch: Option<String>,
        name: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            text,
            r#type,
            branch,
            name,
            staged: false,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Storage error: {0}")]
    Storage(String),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Prompts,
    Canned,
    Notes,
    Snippets,
    Archive,
    Settings,
}

impl Tab {
    pub fn all() -> Vec<Tab> {
        vec![
            Tab::Prompts,
            Tab::Canned,
            Tab::Notes,
            Tab::Snippets,
            Tab::Archive,
            Tab::Settings,
        ]
    }

    pub fn next(self) -> Self {
        let all = Self::all();
        let pos = all.iter().position(|&t| t == self).unwrap();
        all[(pos + 1) % all.len()]
    }

    pub fn prev(self) -> Self {
        let all = Self::all();
        let pos = all.iter().position(|&t| t == self).unwrap();
        all[(pos + all.len() - 1) % all.len()]
    }
}

#[async_trait]
pub trait Storage: Send + Sync {
    async fn get_project_prompts(&self, project_path: &str) -> Result<Vec<Prompt>>;
    async fn get_project_notes(&self, project_path: &str) -> Result<Vec<Prompt>>;
    async fn get_project_archive(&self, project_path: &str) -> Result<Vec<Prompt>>;
    
    async fn save_project_prompts(&self, project_path: &str, prompts: Vec<Prompt>) -> Result<()>;
    async fn save_project_notes(&self, project_path: &str, prompts: Vec<Prompt>) -> Result<()>;
    async fn save_project_archive(&self, project_path: &str, prompts: Vec<Prompt>) -> Result<()>;

    async fn get_global_canned(&self) -> Result<Vec<Prompt>>;
    async fn get_global_snippets(&self) -> Result<Vec<Prompt>>;
    
    async fn save_global_canned(&self, prompts: Vec<Prompt>) -> Result<()>;
    async fn save_global_snippets(&self, prompts: Vec<Prompt>) -> Result<()>;
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
