use async_trait::async_trait;
use contracts::{Clipboard, Git, ProjectInfo, Prompt, Result, Settings, Storage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct InMemoryStorage {
    project_prompts: RwLock<HashMap<String, Vec<Prompt>>>,
    project_notes: RwLock<HashMap<String, Vec<Prompt>>>,
    project_archive: RwLock<HashMap<String, Vec<Prompt>>>,
    project_info: RwLock<HashMap<String, ProjectInfo>>,
    global_canned: RwLock<Vec<Prompt>>,
    global_snippets: RwLock<Vec<Prompt>>,
    settings: RwLock<Settings>,
}

impl InMemoryStorage {
    #[must_use]
    pub fn new() -> Self {
        Self {
            project_prompts: RwLock::new(HashMap::new()),
            project_notes: RwLock::new(HashMap::new()),
            project_archive: RwLock::new(HashMap::new()),
            project_info: RwLock::new(HashMap::new()),
            global_canned: RwLock::new(Vec::new()),
            global_snippets: RwLock::new(Vec::new()),
            settings: RwLock::new(Settings::default()),
        }
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Storage for InMemoryStorage {
    async fn get_project_prompts(&self, project_path: &str) -> Result<Vec<Prompt>> {
        let prompts = self.project_prompts.read().await;
        Ok(prompts.get(project_path).cloned().unwrap_or_default())
    }

    async fn get_project_notes(&self, project_path: &str) -> Result<Vec<Prompt>> {
        let notes = self.project_notes.read().await;
        Ok(notes.get(project_path).cloned().unwrap_or_default())
    }

    async fn get_project_archive(&self, project_path: &str) -> Result<Vec<Prompt>> {
        let archive = self.project_archive.read().await;
        Ok(archive.get(project_path).cloned().unwrap_or_default())
    }

    async fn get_project_info(&self, project_path: &str) -> Result<ProjectInfo> {
        let info = self.project_info.read().await;
        Ok(info.get(project_path).cloned().unwrap_or_default())
    }

    async fn save_project_prompts(&self, project_path: &str, prompts: Vec<Prompt>) -> Result<()> {
        self.project_prompts.write().await.insert(project_path.to_string(), prompts);
        Ok(())
    }

    async fn save_project_notes(&self, project_path: &str, prompts: Vec<Prompt>) -> Result<()> {
        self.project_notes.write().await.insert(project_path.to_string(), prompts);
        Ok(())
    }

    async fn save_project_archive(&self, project_path: &str, prompts: Vec<Prompt>) -> Result<()> {
        self.project_archive.write().await.insert(project_path.to_string(), prompts);
        Ok(())
    }

    async fn save_project_info(&self, project_path: &str, info: ProjectInfo) -> Result<()> {
        self.project_info.write().await.insert(project_path.to_string(), info);
        Ok(())
    }

    async fn get_global_canned(&self) -> Result<Vec<Prompt>> {
        Ok(self.global_canned.read().await.clone())
    }

    async fn get_global_snippets(&self) -> Result<Vec<Prompt>> {
        Ok(self.global_snippets.read().await.clone())
    }

    async fn get_settings(&self) -> Result<Settings> {
        Ok(self.settings.read().await.clone())
    }

    async fn save_global_canned(&self, prompts: Vec<Prompt>) -> Result<()> {
        let mut global = self.global_canned.write().await;
        *global = prompts;
        Ok(())
    }

    async fn save_global_snippets(&self, prompts: Vec<Prompt>) -> Result<()> {
        let mut global = self.global_snippets.write().await;
        *global = prompts;
        Ok(())
    }

    async fn save_settings(&self, settings: Settings) -> Result<()> {
        let mut s = self.settings.write().await;
        *s = settings;
        Ok(())
    }
}

#[derive(Debug)]
pub struct FileSystemStorage {
    base_dir: PathBuf,
}

impl FileSystemStorage {
    pub fn new(base_dir: Option<PathBuf>) -> Self {
        let base_dir = base_dir.unwrap_or_else(|| {
            directories::ProjectDirs::from("", "", "promptquiver")
                .map(|d| d.data_dir().to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."))
        });
        
        // Ensure data directory exists
        if !base_dir.exists() {
            let _ = std::fs::create_dir_all(&base_dir);
        }
        
        Self { base_dir }
    }

    fn global_path(&self) -> PathBuf {
        self.base_dir.join("common.toml")
    }

    fn project_path(&self, project_path: &str) -> PathBuf {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(project_path.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        let filename = format!("{}.toml", &hash[..8]);
        
        let projects_dir = self.base_dir.join("projects");
        if !projects_dir.exists() {
            let _ = std::fs::create_dir_all(&projects_dir);
        }
        
        projects_dir.join(filename)
    }

    async fn read_toml<T: serde::de::DeserializeOwned>(&self, path: PathBuf) -> Result<T> {
        if !path.exists() {
            return Err(contracts::Error::NotFound);
        }
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| contracts::Error::Storage(e.to_string()))?;
        serde_toml::from_str(&content).map_err(|e| contracts::Error::Storage(e.to_string()))
    }

    async fn write_toml<T: serde::Serialize>(&self, path: PathBuf, data: &T) -> Result<()> {
        let content = serde_toml::to_string_pretty(data)
            .map_err(|e| contracts::Error::Storage(e.to_string()))?;
        
        let temp_path = path.with_extension("tmp");
        tokio::fs::write(&temp_path, content)
            .await
            .map_err(|e| contracts::Error::Storage(e.to_string()))?;
        
        tokio::fs::rename(&temp_path, &path)
            .await
            .map_err(|e| contracts::Error::Storage(e.to_string()))?;
        
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct ProjectFile {
    #[serde(default)]
    info: ProjectInfo,
    #[serde(default)]
    main: Vec<Prompt>,
    #[serde(default)]
    notes: Vec<Prompt>,
    #[serde(default)]
    archive: Vec<Prompt>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct GlobalFile {
    #[serde(default)]
    canned: Vec<Prompt>,
    #[serde(default)]
    snippets: Vec<Prompt>,
    #[serde(default)]
    settings: Settings,
}

#[async_trait]
impl Storage for FileSystemStorage {
    async fn get_project_prompts(&self, project_path: &str) -> Result<Vec<Prompt>> {
        let file: ProjectFile = self.read_toml(self.project_path(project_path)).await.unwrap_or_default();
        Ok(file.main)
    }

    async fn get_project_notes(&self, project_path: &str) -> Result<Vec<Prompt>> {
        let file: ProjectFile = self.read_toml(self.project_path(project_path)).await.unwrap_or_default();
        Ok(file.notes)
    }

    async fn get_project_archive(&self, project_path: &str) -> Result<Vec<Prompt>> {
        let file: ProjectFile = self.read_toml(self.project_path(project_path)).await.unwrap_or_default();
        Ok(file.archive)
    }

    async fn get_project_info(&self, project_path: &str) -> Result<ProjectInfo> {
        let file: ProjectFile = self.read_toml(self.project_path(project_path)).await.unwrap_or_default();
        Ok(file.info)
    }

    async fn save_project_prompts(&self, project_path: &str, prompts: Vec<Prompt>) -> Result<()> {
        let path = self.project_path(project_path);
        let mut file: ProjectFile = self.read_toml(path.clone()).await.unwrap_or_default();
        file.main = prompts;
        self.write_toml(path, &file).await
    }

    async fn save_project_notes(&self, project_path: &str, prompts: Vec<Prompt>) -> Result<()> {
        let path = self.project_path(project_path);
        let mut file: ProjectFile = self.read_toml(path.clone()).await.unwrap_or_default();
        file.notes = prompts;
        self.write_toml(path, &file).await
    }

    async fn save_project_archive(&self, project_path: &str, prompts: Vec<Prompt>) -> Result<()> {
        let path = self.project_path(project_path);
        let mut file: ProjectFile = self.read_toml(path.clone()).await.unwrap_or_default();
        file.archive = prompts;
        self.write_toml(path, &file).await
    }

    async fn save_project_info(&self, project_path: &str, info: ProjectInfo) -> Result<()> {
        let path = self.project_path(project_path);
        let mut file: ProjectFile = self.read_toml(path.clone()).await.unwrap_or_default();
        file.info = info;
        self.write_toml(path, &file).await
    }

    async fn get_global_canned(&self) -> Result<Vec<Prompt>> {
        let file: GlobalFile = self.read_toml(self.global_path()).await.unwrap_or_default();
        Ok(file.canned)
    }

    async fn get_global_snippets(&self) -> Result<Vec<Prompt>> {
        let file: GlobalFile = self.read_toml(self.global_path()).await.unwrap_or_default();
        Ok(file.snippets)
    }

    async fn get_settings(&self) -> Result<Settings> {
        let file: GlobalFile = self.read_toml(self.global_path()).await.unwrap_or_default();
        Ok(file.settings)
    }

    async fn save_global_canned(&self, prompts: Vec<Prompt>) -> Result<()> {
        let path = self.global_path();
        let mut file: GlobalFile = self.read_toml(path.clone()).await.unwrap_or_default();
        file.canned = prompts;
        self.write_toml(path, &file).await
    }

    async fn save_global_snippets(&self, prompts: Vec<Prompt>) -> Result<()> {
        let path = self.global_path();
        let mut file: GlobalFile = self.read_toml(path.clone()).await.unwrap_or_default();
        file.snippets = prompts;
        self.write_toml(path, &file).await
    }

    async fn save_settings(&self, settings: Settings) -> Result<()> {
        let path = self.global_path();
        let mut file: GlobalFile = self.read_toml(path.clone()).await.unwrap_or_default();
        file.settings = settings;
        self.write_toml(path, &file).await
    }
}

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
        Ok(())
    }

    async fn paste(&self) -> Result<String> {
        Ok(self.content.read().await.clone())
    }
}

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

pub struct RealClipboard;

impl RealClipboard {
    #[must_use]
    pub fn new() -> Self {
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

pub struct RealGit;

impl RealGit {
    #[must_use]
    pub fn new() -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::PromptType;

    #[tokio::test]
    async fn test_in_memory_storage() {
        let storage = InMemoryStorage::new();
        let prompt = Prompt::new("test".to_string(), PromptType::Prompt, None, None);

        storage.save_project_prompts("path", vec![prompt.clone()]).await.unwrap();
        let loaded = storage.get_project_prompts("path").await.unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].text, "test");
    }

    #[tokio::test]
    async fn test_mock_clipboard() {
        let clipboard = MockClipboard::new();
        clipboard.copy("hello".to_string()).await.unwrap();
        assert_eq!(clipboard.paste().await.unwrap(), "hello");
    }

    #[tokio::test]
    async fn test_mock_git() {
        let git = MockGit::new(Some("main".to_string()));
        assert_eq!(git.get_current_branch("any").await.unwrap(), Some("main".to_string()));

        git.set_branch(None).await;
        assert_eq!(git.get_current_branch("any").await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_file_system_storage() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base_dir = temp_dir.path().to_path_buf();
        let storage = FileSystemStorage::new(Some(base_dir));

        let prompt = Prompt::new("test".to_string(), contracts::PromptType::Prompt, None, None);
        let project_path = temp_dir.path().to_str().unwrap();

        storage.save_project_prompts(project_path, vec![prompt.clone()]).await.unwrap();
        let loaded = storage.get_project_prompts(project_path).await.unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].text, "test");

        // Global
        storage.save_global_snippets(vec![prompt.clone()]).await.unwrap();
        let loaded_global = storage.get_global_snippets().await.unwrap();
        assert_eq!(loaded_global.len(), 1);
    }
}
