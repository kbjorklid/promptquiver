use async_trait::async_trait;
use contracts::{Git, Result};
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct MockGit {
    branch: RwLock<Option<String>>,
}

impl MockGit {
    #[must_use]
    pub fn new(branch: Option<String>) -> Self {
        Self { branch: RwLock::new(branch) }
    }

    pub async fn set_branch(&self, branch: Option<String>) {
        let mut b = self.branch.write().await;
        *b = branch;
    }
}

#[async_trait]
impl Git for MockGit {
    async fn get_current_branch(&self, _path: &str) -> Result<Option<String>> {
        Ok(self.branch.read().await.clone())
    }
}
