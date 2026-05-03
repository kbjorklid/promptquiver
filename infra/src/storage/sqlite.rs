use async_trait::async_trait;
use contracts::{ProjectInfo, Prompt, Project, Result, Settings, Storage, PromptFilter, Tab, PromptType};
use std::path::PathBuf;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rusqlite::OptionalExtension;

#[derive(Debug)]
pub struct SqliteStorage {
    db_path: PathBuf,
}

impl SqliteStorage {
    /// Creates a new `SqliteStorage`
    /// 
    /// # Panics
    /// 
    /// Panics if the database cannot be initialized.
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
            "CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL UNIQUE,
                created_at TEXT NOT NULL
            )",
            [],
        ).map_err(|e| contracts::Error::Storage(e.to_string()))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS prompts (
                id TEXT PRIMARY KEY,
                text TEXT NOT NULL,
                type TEXT NOT NULL,
                folder TEXT,
                project_id TEXT,
                branch TEXT,
                name TEXT,
                staged BOOLEAN NOT NULL DEFAULT 0,
                last_copied BOOLEAN NOT NULL DEFAULT 0,
                is_archived BOOLEAN NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                order_index INTEGER NOT NULL DEFAULT 0
            )",
            [],
        ).map_err(|e| contracts::Error::Storage(e.to_string()))?;

        // Migrations
        let _ = conn.execute("ALTER TABLE prompts ADD COLUMN order_index INTEGER NOT NULL DEFAULT 0", []);
        let _ = conn.execute("ALTER TABLE prompts RENAME COLUMN project TO project_id", []);

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
            
            let mut query = "SELECT id, text, type, folder, project_id, branch, name, staged, last_copied, is_archived, created_at, updated_at, order_index FROM prompts WHERE 1=1".to_string();
            let mut params: Vec<String> = Vec::new();

            if let Some(folder) = filter.folder {
                query.push_str(" AND folder = ?");
                params.push(folder);
            }
            
            if filter.project_filter {
                if let Some(id) = filter.project_id {
                    query.push_str(" AND project_id = ?");
                    params.push(id.to_string());
                } else {
                    query.push_str(" AND project_id IS NULL");
                }
            }

            if let Some(branch) = filter.branch {
                query.push_str(" AND branch = ?");
                params.push(branch);
            }

            if let Some(staged) = filter.staged {
                query.push_str(" AND staged = ?");
                params.push(if staged { "1".to_string() } else { "0".to_string() });
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

            query.push_str(" ORDER BY order_index ASC, created_at DESC");

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
                    project_id: row.get::<_, Option<String>>(4)?.map(|s| Uuid::parse_str(&s).unwrap()),
                    branch: row.get(5)?,
                    name: row.get(6)?,
                    staged: row.get(7)?,
                    last_copied: row.get(8)?,
                    is_archived: row.get(9)?,
                    created_at: row.get::<_, String>(10)?.parse::<DateTime<Utc>>().unwrap(),
                    updated_at: row.get::<_, String>(11)?.parse::<DateTime<Utc>>().unwrap(),
                    order_index: row.get(12)?,
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
        let changed = tokio::task::spawn_blocking(move || {
            let conn = rusqlite::Connection::open(db_path)
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            
            conn.execute(
                "INSERT INTO prompts (id, text, type, folder, project_id, branch, name, staged, last_copied, is_archived, created_at, updated_at, order_index)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                 ON CONFLICT(id) DO UPDATE SET
                    text=excluded.text,
                    type=excluded.type,
                    folder=excluded.folder,
                    project_id=excluded.project_id,
                    branch=excluded.branch,
                    name=excluded.name,
                    staged=excluded.staged,
                    last_copied=excluded.last_copied,
                    is_archived=excluded.is_archived,
                    updated_at=excluded.updated_at,
                    order_index=excluded.order_index
                 WHERE text != excluded.text OR type != excluded.type OR folder IS NOT excluded.folder 
                    OR project_id IS NOT excluded.project_id OR branch IS NOT excluded.branch OR name IS NOT excluded.name 
                    OR staged != excluded.staged OR last_copied != excluded.last_copied OR is_archived != excluded.is_archived
                    OR order_index != excluded.order_index",
                rusqlite::params![
                    prompt.id.to_string(),
                    prompt.text,
                    format!("{:?}", prompt.r#type),
                    prompt.folder,
                    prompt.project_id.map(|id| id.to_string()),
                    prompt.branch,
                    prompt.name,
                    prompt.staged,
                    prompt.last_copied,
                    prompt.is_archived,
                    prompt.created_at.to_rfc3339(),
                    prompt.updated_at.to_rfc3339(),
                    prompt.order_index,
                ],
            ).map_err(|e| contracts::Error::Storage(e.to_string()))?;
            Ok::<bool, contracts::Error>(conn.changes() > 0)
        }).await.map_err(|e| contracts::Error::Storage(e.to_string()))??;
        
        if changed {
            self.increment_data_version().await?;
        }
        Ok(())
    }

    async fn save_prompts(&self, prompts: Vec<Prompt>) -> Result<()> {
        let db_path = self.db_path.clone();
        let changed = tokio::task::spawn_blocking(move || {
            let mut conn = rusqlite::Connection::open(db_path)
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            
            let tx = conn.transaction().map_err(|e| contracts::Error::Storage(e.to_string()))?;
            let mut any_changed = false;

            for prompt in prompts {
                tx.execute(
                    "INSERT INTO prompts (id, text, type, folder, project_id, branch, name, staged, last_copied, is_archived, created_at, updated_at, order_index)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                     ON CONFLICT(id) DO UPDATE SET
                        text=excluded.text,
                        type=excluded.type,
                        folder=excluded.folder,
                        project_id=excluded.project_id,
                        branch=excluded.branch,
                        name=excluded.name,
                        staged=excluded.staged,
                        last_copied=excluded.last_copied,
                        is_archived=excluded.is_archived,
                        updated_at=excluded.updated_at,
                        order_index=excluded.order_index
                     WHERE text != excluded.text OR type != excluded.type OR folder IS NOT excluded.folder 
                        OR project_id IS NOT excluded.project_id OR branch IS NOT excluded.branch OR name IS NOT excluded.name 
                        OR staged != excluded.staged OR last_copied != excluded.last_copied OR is_archived != excluded.is_archived
                        OR order_index != excluded.order_index",
                    rusqlite::params![
                        prompt.id.to_string(),
                        prompt.text,
                        format!("{:?}", prompt.r#type),
                        prompt.folder,
                        prompt.project_id.map(|id| id.to_string()),
                        prompt.branch,
                        prompt.name,
                        prompt.staged,
                        prompt.last_copied,
                        prompt.is_archived,
                        prompt.created_at.to_rfc3339(),
                        prompt.updated_at.to_rfc3339(),
                        prompt.order_index,
                    ],
                ).map_err(|e| contracts::Error::Storage(e.to_string()))?;
                if tx.changes() > 0 {
                    any_changed = true;
                }
            }
            tx.commit().map_err(|e| contracts::Error::Storage(e.to_string()))?;
            Ok::<bool, contracts::Error>(any_changed)
        }).await.map_err(|e| contracts::Error::Storage(e.to_string()))??;

        if changed {
            self.increment_data_version().await?;
        }
        Ok(())
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

    async fn get_projects(&self) -> Result<Vec<Project>> {
        let db_path = self.db_path.clone();
        tokio::task::spawn_blocking(move || {
            let conn = rusqlite::Connection::open(db_path)
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            let mut stmt = conn.prepare("SELECT id, title, created_at FROM projects ORDER BY title ASC")
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            let project_iter = stmt.query_map([], |row| {
                Ok(Project {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    title: row.get(1)?,
                    created_at: row.get::<_, String>(2)?.parse::<DateTime<Utc>>().unwrap(),
                })
            }).map_err(|e| contracts::Error::Storage(e.to_string()))?;

            let mut projects = Vec::new();
            for project in project_iter {
                projects.push(project.map_err(|e| contracts::Error::Storage(e.to_string()))?);
            }
            Ok(projects)
        }).await.map_err(|e| contracts::Error::Storage(e.to_string()))?
    }

    async fn save_project(&self, project: Project) -> Result<()> {
        let db_path = self.db_path.clone();
        let changed = tokio::task::spawn_blocking(move || {
            let conn = rusqlite::Connection::open(db_path)
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            conn.execute(
                "INSERT INTO projects (id, title, created_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(id) DO UPDATE SET title=excluded.title WHERE title != excluded.title",
                rusqlite::params![
                    project.id.to_string(),
                    project.title,
                    project.created_at.to_rfc3339(),
                ],
            ).map_err(|e| contracts::Error::Storage(e.to_string()))?;
            Ok::<bool, contracts::Error>(conn.changes() > 0)
        }).await.map_err(|e| contracts::Error::Storage(e.to_string()))??;

        if changed {
            self.increment_data_version().await?;
        }
        Ok(())
    }

    async fn delete_project(&self, id: Uuid) -> Result<()> {
        let db_path = self.db_path.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = rusqlite::Connection::open(db_path)
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            let tx = conn.transaction().map_err(|e| contracts::Error::Storage(e.to_string()))?;
            
            tx.execute("DELETE FROM projects WHERE id = ?", [id.to_string()])
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            
            tx.execute("UPDATE prompts SET project_id = NULL WHERE project_id = ?", [id.to_string()])
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            
            tx.commit().map_err(|e| contracts::Error::Storage(e.to_string()))?;
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
            
            data.map_or_else(|| Ok(ProjectInfo::default()), |d| serde_json::from_str(&d).map_err(|e| contracts::Error::Storage(e.to_string())))
        }).await.map_err(|e| contracts::Error::Storage(e.to_string()))?
    }

    async fn save_project_info(&self, folder: &str, info: ProjectInfo) -> Result<()> {
        let db_path = self.db_path.clone();
        let folder = folder.to_string();
        let changed = tokio::task::spawn_blocking(move || {
            let conn = rusqlite::Connection::open(db_path)
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            let data = serde_json::to_string(&info).map_err(|e| contracts::Error::Storage(e.to_string()))?;
            conn.execute(
                "INSERT INTO project_info (folder, data) VALUES (?1, ?2)
                 ON CONFLICT(folder) DO UPDATE SET data=excluded.data WHERE data != excluded.data",
                rusqlite::params![folder, data],
            ).map_err(|e| contracts::Error::Storage(e.to_string()))?;
            Ok::<bool, contracts::Error>(conn.changes() > 0)
        }).await.map_err(|e| contracts::Error::Storage(e.to_string()))??;
        
        if changed {
            self.increment_data_version().await?;
        }
        Ok(())
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
            
            data.map_or_else(|| Ok(Settings::default()), |d| serde_json::from_str(&d).map_err(|e| contracts::Error::Storage(e.to_string())))
        }).await.map_err(|e| contracts::Error::Storage(e.to_string()))?
    }

    async fn save_settings(&self, settings: Settings) -> Result<()> {
        let db_path = self.db_path.clone();
        let changed = tokio::task::spawn_blocking(move || {
            let conn = rusqlite::Connection::open(db_path)
                .map_err(|e| contracts::Error::Storage(e.to_string()))?;
            let data = serde_json::to_string(&settings).map_err(|e| contracts::Error::Storage(e.to_string()))?;
            conn.execute(
                "INSERT INTO settings (id, data) VALUES (1, ?1)
                 ON CONFLICT(id) DO UPDATE SET data=excluded.data WHERE data != excluded.data",
                rusqlite::params![data],
            ).map_err(|e| contracts::Error::Storage(e.to_string()))?;
            Ok::<bool, contracts::Error>(conn.changes() > 0)
        }).await.map_err(|e| contracts::Error::Storage(e.to_string()))??;
        
        if changed {
            self.increment_data_version().await?;
        }
        Ok(())
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
