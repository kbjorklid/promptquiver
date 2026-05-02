use async_trait::async_trait;
use contracts::{Git, Result};

#[derive(Debug)]
pub struct RealGit;

impl RealGit {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for RealGit {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Git for RealGit {
    async fn get_current_branch(&self, path: &str) -> Result<Option<String>> {
        let output = tokio::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(path)
            .output()
            .await
            .map_err(|e| contracts::Error::Git(e.to_string()))?;

        if output.status.success() {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if branch == "HEAD" || branch.is_empty() {
                Ok(None)
            } else {
                Ok(Some(branch))
            }
        } else {
            Ok(None)
        }
    }
}
