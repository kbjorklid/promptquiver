use async_trait::async_trait;
use contracts::{Clipboard, Git, ProjectInfo, Prompt, Result, Settings, Storage, PromptFilter, Tab, PromptType};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rusqlite::OptionalExtension;

#[derive(Debug)]
pub struct InMemoryStorage {
    prompts: RwLock<Vec<Prompt>>,
    project_info: RwLock<HashMap<String, ProjectInfo>>,
    settings: RwLock<Settings>,
}

impl InMemoryStorage {
    #[must_use]
    pub fn new() -> Self {
        Self {
            prompts: RwLock::new(Vec::new()),
            project_info: RwLock::new(HashMap::new()),
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
    async fn get_prompts(&self, filter: PromptFilter) -> Result<Vec<Prompt>> {
        let prompts = self.prompts.read().await;
        let mut filtered: Vec<Prompt> = prompts.iter().cloned().collect();

        if let Some(folder) = filter.folder {
            filtered.retain(|p| p.folder.as_deref() == Some(&folder));
        }
        if let Some(project) = filter.project {
            filtered.retain(|p| p.project.as_deref() == Some(&project));
        }
        if let Some(branch) = filter.branch {
            filtered.retain(|p| p.branch.as_deref() == Some(&branch));
        }
        if let Some(tab) = filter.tab {
            match tab {
                Tab::Prompts => {
                    filtered.retain(|p| p.r#type == PromptType::Prompt && !p.is_archived && p.folder.is_some());
                }
                Tab::Canned => {
                    filtered.retain(|p| p.r#type == PromptType::Prompt && !p.is_archived && p.folder.is_none());
                }
                Tab::Notes => {
                    filtered.retain(|p| p.r#type == PromptType::Note && !p.is_archived);
                }
                Tab::Snippets => {
                    filtered.retain(|p| p.r#type == PromptType::Snippet && !p.is_archived);
                }
                Tab::Archive => {
                    filtered.retain(|p| p.is_archived);
                }
                Tab::Settings => {
                    filtered.clear();
                }
            }
        }
        
        // Sort by created_at DESC (mimic DB behavior)
        filtered.sort_by_key(|p| std::cmp::Reverse(p.created_at));

        Ok(filtered)
    }

    async fn save_prompt(&self, prompt: Prompt) -> Result<()> {
        let mut prompts = self.prompts.write().await;
        if let Some(p) = prompts.iter_mut().find(|p| p.id == prompt.id) {
            *p = prompt;
        } else {
            prompts.push(prompt);
        }
        Ok(())
    }

    async fn delete_prompt(&self, id: Uuid) -> Result<()> {
        let mut prompts = self.prompts.write().await;
        prompts.retain(|p| p.id != id);
        Ok(())
    }

    async fn get_project_info(&self, folder: &str) -> Result<ProjectInfo> {
        let info = self.project_info.read().await;
        Ok(info.get(folder).cloned().unwrap_or_default())
    }

    async fn save_project_info(&self, folder: &str, info: ProjectInfo) -> Result<()> {
        self.project_info.write().await.insert(folder.to_string(), info);
        Ok(())
    }

    async fn get_settings(&self) -> Result<Settings> {
        Ok(self.settings.read().await.clone())
    }

    async fn save_settings(&self, settings: Settings) -> Result<()> {
        let mut s = self.settings.write().await;
        *s = settings;
        Ok(())
    }

    async fn get_data_version(&self) -> Result<u32> {
        Ok(0)
    }
}

#[derive(Debug)]
pub struct SqliteStorage {
    db_path: PathBuf,
}

impl SqliteStorage {
    pub fn new(db_path: PathBuf) -> Self {
        let storage = Self { db_path };
        storage.init().expect("Failed to initialize database");
        storage
    }

    fn init(&self) -> Result<()> {
        let conn = rusqlite::Connection::open(&self.db_path)
            .map_err(|e| contracts::Error::Storage(e.to_string()))?;
        
        conn.execute("PRAGMA journal_mode=WAL", []).ok();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS prompts (
                id TEXT PRIMARY KEY,
                text TEXT NOT NULL,
                type TEXT NOT NULL,
                folder TEXT,
                project TEXT,
                branch TEXT,
                name TEXT,
                staged BOOLEAN NOT NULL DEFAULT 0,
                last_copied BOOLEAN NOT NULL DEFAULT 0,
                is_archived BOOLEAN NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        ).map_err(|e| contracts::Error::Storage(e.to_string()))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS project_info (
                folder TEXT PRIMARY KEY,
                data TEXT NOT NULL
            )",
            [],
        ).map_err(|e| contracts::Error::Storage(e.to_string()))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                data TEXT NOT NULL
            )",
            [],
        ).map_err(|e| contracts::Error::Storage(e.to_string()))?;

        Ok(())
    }

    async fn increment_data_version(&self) -> Result<()> {
        let db_path = self.db_path.clone();
        tokio::task::spawn_blocking(move || {
            let conn = rusqlite::Connection::open(db_path)
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            let current: u32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            conn.execute(&format!("PRAGMA user_version = {}", current + 1), [])
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            Ok::<(), contracts::Error>(())
        }).await.map_err(|e| contracts::Error::Storage(e.to_string()))?
    }
}

#[async_trait]
impl Storage for SqliteStorage {
    async fn get_prompts(&self, filter: PromptFilter) -> Result<Vec<Prompt>> {
        let db_path = self.db_path.clone();
        tokio::task::spawn_blocking(move || {
            let conn = rusqlite::Connection::open(db_path)
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            
            let mut query = "SELECT id, text, type, folder, project, branch, name, staged, last_copied, is_archived, created_at, updated_at FROM prompts WHERE 1=1".to_string();
            let mut params: Vec<String> = Vec::new();

            if let Some(folder) = filter.folder {
                query.push_str(" AND folder = ?");
                params.push(folder);
            }
            if let Some(project) = filter.project {
                query.push_str(" AND project = ?");
                params.push(project);
            }
            if let Some(branch) = filter.branch {
                query.push_str(" AND branch = ?");
                params.push(branch);
            }
            
            if let Some(tab) = filter.tab {
                match tab {
                    Tab::Prompts => {
                        query.push_str(" AND type = 'Prompt' AND is_archived = 0 AND folder IS NOT NULL");
                    }
                    Tab::Canned => {
                        query.push_str(" AND type = 'Prompt' AND is_archived = 0 AND folder IS NULL");
                    }
                    Tab::Notes => {
                        query.push_str(" AND type = 'Note' AND is_archived = 0");
                    }
                    Tab::Snippets => {
                        query.push_str(" AND type = 'Snippet' AND is_archived = 0");
                    }
                    Tab::Archive => {
                        query.push_str(" AND is_archived = 1");
                    }
                    Tab::Settings => {
                        return Ok(Vec::new());
                    }
                }
            }

            query.push_str(" ORDER BY created_at DESC");

            let mut stmt = conn.prepare(&query).map_err(|e| contracts::Error::Storage(e.to_string()))?;
            
            let prompt_iter = stmt.query_map(rusqlite::params_from_iter(params), |row| {
                Ok(Prompt {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    text: row.get(1)?,
                    r#type: match row.get::<_, String>(2)?.as_str() {
                        "Note" => PromptType::Note,
                        "Snippet" => PromptType::Snippet,
                        _ => PromptType::Prompt,
                    },
                    folder: row.get(3)?,
                    project: row.get(4)?,
                    branch: row.get(5)?,
                    name: row.get(6)?,
                    staged: row.get(7)?,
                    last_copied: row.get(8)?,
                    is_archived: row.get(9)?,
                    created_at: row.get::<_, String>(10)?.parse::<DateTime<Utc>>().unwrap(),
                    updated_at: row.get::<_, String>(11)?.parse::<DateTime<Utc>>().unwrap(),
                })
            }).map_err(|e| contracts::Error::Storage(e.to_string()))?;

            let mut prompts = Vec::new();
            for prompt in prompt_iter {
                prompts.push(prompt.map_err(|e| contracts::Error::Storage(e.to_string()))?);
            }
            Ok(prompts)
        }).await.map_err(|e| contracts::Error::Storage(e.to_string()))?
    }

    async fn save_prompt(&self, prompt: Prompt) -> Result<()> {
        let db_path = self.db_path.clone();
        tokio::task::spawn_blocking(move || {
            let conn = rusqlite::Connection::open(db_path)
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            
            conn.execute(
                "INSERT INTO prompts (id, text, type, folder, project, branch, name, staged, last_copied, is_archived, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                 ON CONFLICT(id) DO UPDATE SET
                    text=excluded.text,
                    type=excluded.type,
                    folder=excluded.folder,
                    project=excluded.project,
                    branch=excluded.branch,
                    name=excluded.name,
                    staged=excluded.staged,
                    last_copied=excluded.last_copied,
                    is_archived=excluded.is_archived,
                    updated_at=excluded.updated_at",
                rusqlite::params![
                    prompt.id.to_string(),
                    prompt.text,
                    format!("{:?}", prompt.r#type),
                    prompt.folder,
                    prompt.project,
                    prompt.branch,
                    prompt.name,
                    prompt.staged,
                    prompt.last_copied,
                    prompt.is_archived,
                    prompt.created_at.to_rfc3339(),
                    prompt.updated_at.to_rfc3339(),
                ],
            ).map_err(|e| contracts::Error::Storage(e.to_string()))?;
            Ok::<(), contracts::Error>(())
        }).await.map_err(|e| contracts::Error::Storage(e.to_string()))??;
        self.increment_data_version().await
    }

    async fn delete_prompt(&self, id: Uuid) -> Result<()> {
        let db_path = self.db_path.clone();
        tokio::task::spawn_blocking(move || {
            let conn = rusqlite::Connection::open(db_path)
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            conn.execute("DELETE FROM prompts WHERE id = ?", [id.to_string()])
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            Ok::<(), contracts::Error>(())
        }).await.map_err(|e| contracts::Error::Storage(e.to_string()))??;
        self.increment_data_version().await
    }

    async fn get_project_info(&self, folder: &str) -> Result<ProjectInfo> {
        let db_path = self.db_path.clone();
        let folder = folder.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = rusqlite::Connection::open(db_path)
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            let mut stmt = conn.prepare("SELECT data FROM project_info WHERE folder = ?")
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            let data: Option<String> = stmt.query_row([folder], |row| row.get(0)).optional()
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            
            if let Some(d) = data {
                serde_json::from_str(&d).map_err(|e| contracts::Error::Storage(e.to_string()))
            } else {
                Ok(ProjectInfo::default())
            }
        }).await.map_err(|e| contracts::Error::Storage(e.to_string()))?
    }

    async fn save_project_info(&self, folder: &str, info: ProjectInfo) -> Result<()> {
        let db_path = self.db_path.clone();
        let folder = folder.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = rusqlite::Connection::open(db_path)
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            let data = serde_json::to_string(&info).map_err(|e| contracts::Error::Storage(e.to_string()))?;
            conn.execute(
                "INSERT INTO project_info (folder, data) VALUES (?1, ?2)
                 ON CONFLICT(folder) DO UPDATE SET data=excluded.data",
                rusqlite::params![folder, data],
            ).map_err(|e| contracts::Error::Storage(e.to_string()))?;
            Ok::<(), contracts::Error>(())
        }).await.map_err(|e| contracts::Error::Storage(e.to_string()))??;
        self.increment_data_version().await
    }

    async fn get_settings(&self) -> Result<Settings> {
        let db_path = self.db_path.clone();
        tokio::task::spawn_blocking(move || {
            let conn = rusqlite::Connection::open(db_path)
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            let mut stmt = conn.prepare("SELECT data FROM settings WHERE id = 1")
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            let data: Option<String> = stmt.query_row([], |row| row.get(0)).optional()
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            
            if let Some(d) = data {
                serde_json::from_str(&d).map_err(|e| contracts::Error::Storage(e.to_string()))
            } else {
                Ok(Settings::default())
            }
        }).await.map_err(|e| contracts::Error::Storage(e.to_string()))?
    }

    async fn save_settings(&self, settings: Settings) -> Result<()> {
        let db_path = self.db_path.clone();
        tokio::task::spawn_blocking(move || {
            let conn = rusqlite::Connection::open(db_path)
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            let data = serde_json::to_string(&settings).map_err(|e| contracts::Error::Storage(e.to_string()))?;
            conn.execute(
                "INSERT INTO settings (id, data) VALUES (1, ?1)
                 ON CONFLICT(id) DO UPDATE SET data=excluded.data",
                rusqlite::params![data],
            ).map_err(|e| contracts::Error::Storage(e.to_string()))?;
            Ok::<(), contracts::Error>(())
        }).await.map_err(|e| contracts::Error::Storage(e.to_string()))??;
        self.increment_data_version().await
    }

    async fn get_data_version(&self) -> Result<u32> {
        let db_path = self.db_path.clone();
        tokio::task::spawn_blocking(move || {
            let conn = rusqlite::Connection::open(db_path)
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            let version: u32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            Ok(version)
        }).await.map_err(|e| contracts::Error::Storage(e.to_string()))?
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

pub mod service;
pub use service::RealAppService;

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

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::PromptType;
    use rusqlite::OptionalExtension;

    #[tokio::test]
    async fn test_in_memory_storage() {
        let storage = InMemoryStorage::new();
        let prompt = Prompt::new("test".to_string(), PromptType::Prompt, Some("path".to_string()), None, None);

        storage.save_prompt(prompt.clone()).await.unwrap();
        let loaded = storage.get_prompts(PromptFilter { folder: Some("path".to_string()), ..Default::default() }).await.unwrap();

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
    async fn test_sqlite_storage() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = SqliteStorage::new(db_path);

        let prompt = Prompt::new("sqlite test".to_string(), contracts::PromptType::Prompt, Some("/path/to/project".to_string()), Some("main".to_string()), None);
        
        storage.save_prompt(prompt.clone()).await.unwrap();
        let loaded = storage.get_prompts(PromptFilter { folder: Some("/path/to/project".to_string()), ..Default::default() }).await.unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].text, "sqlite test");
        assert_eq!(loaded[0].branch.as_deref(), Some("main"));
        
        // Test update
        let mut updated = loaded[0].clone();
        updated.text = "updated".to_string();
        storage.save_prompt(updated).await.unwrap();
        
        let loaded_updated = storage.get_prompts(PromptFilter { folder: Some("/path/to/project".to_string()), ..Default::default() }).await.unwrap();
        assert_eq!(loaded_updated.len(), 1);
        assert_eq!(loaded_updated[0].text, "updated");
        
        // Test delete
        storage.delete_prompt(prompt.id).await.unwrap();
        let loaded_deleted = storage.get_prompts(PromptFilter { folder: Some("/path/to/project".to_string()), ..Default::default() }).await.unwrap();
        assert_eq!(loaded_deleted.len(), 0);
    }
}
