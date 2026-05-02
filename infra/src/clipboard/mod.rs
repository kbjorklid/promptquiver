use async_trait::async_trait;
use contracts::{Clipboard, Result};
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct MockClipboard {
    content: RwLock<String>,
}

impl MockClipboard {
    #[must_use]
    pub fn new() -> Self {
        Self { content: RwLock::new(String::new()) }
    }
}

impl Default for MockClipboard {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Clipboard for MockClipboard {
    async fn copy(&self, text: String) -> Result<()> {
        let mut content = self.content.write().await;
        *content = text;
        drop(content);
        Ok(())
    }

    async fn paste(&self) -> Result<String> {
        Ok(self.content.read().await.clone())
    }
}

#[derive(Debug)]
pub struct RealClipboard;

impl RealClipboard {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for RealClipboard {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Clipboard for RealClipboard {
    async fn copy(&self, text: String) -> Result<()> {
        tokio::task::spawn_blocking(move || {
            let mut clipboard = arboard::Clipboard::new()
                .map_err(|e| contracts::Error::Clipboard(e.to_string()))?;
            clipboard.set_text(text)
                .map_err(|e| contracts::Error::Clipboard(e.to_string()))?;
            Ok(())
        })
        .await
        .map_err(|e| contracts::Error::Clipboard(e.to_string()))?
    }

    async fn paste(&self) -> Result<String> {
        tokio::task::spawn_blocking(|| {
            let mut clipboard = arboard::Clipboard::new()
                .map_err(|e| contracts::Error::Clipboard(e.to_string()))?;
            clipboard.get_text()
                .map_err(|e| contracts::Error::Clipboard(e.to_string()))
        })
        .await
        .map_err(|e| contracts::Error::Clipboard(e.to_string()))?
    }
}
