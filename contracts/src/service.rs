use crate::{Prompt, Result, Tab};
use async_trait::async_trait;

#[async_trait]
pub trait AppService: Send + Sync {
    async fn stage_item(&self, project_path: &str, tab: Tab, item: Prompt) -> Result<()>;
    async fn archive_item(&self, project_path: &str, tab: Tab, item: Prompt) -> Result<()>;
    async fn restore_item(&self, project_path: &str, item: Prompt) -> Result<()>;
    async fn duplicate_item(&self, project_path: &str, tab: Tab, item: Prompt) -> Result<Option<Prompt>>;
    async fn copy_item(&self, project_path: &str, tab: Tab, item: Prompt) -> Result<()>;
}
