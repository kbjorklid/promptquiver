use crate::{Prompt, Result, Tab};
use async_trait::async_trait;

#[async_trait]
pub trait AppService: Send + Sync {
    async fn stage_item(&self, project_path: &str, tab: Tab, item: Prompt) -> Result<()>;
    async fn archive_item(&self, project_path: &str, tab: Tab, item: Prompt) -> Result<()>;
    async fn restore_item(&self, project_path: &str, item: Prompt) -> Result<()>;
    async fn duplicate_item(&self, project_path: &str, tab: Tab, item: Prompt) -> Result<Option<Prompt>>;
    async fn copy_item(&self, project_path: &str, tab: Tab, item: Prompt) -> Result<()>;
    async fn save_item(&self, project_path: &str, tab: Tab, text: String, title: Option<String>, id: Option<uuid::Uuid>, insert_index: Option<usize>, branch: Option<String>, project_id: Option<uuid::Uuid>) -> Result<()>;
    async fn search_files(&self, base_dir: &str, query: &str) -> Result<Vec<Prompt>>;
}
