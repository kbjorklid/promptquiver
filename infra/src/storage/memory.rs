use async_trait::async_trait;
use contracts::{
    Project, ProjectInfo, Prompt, PromptFilter, PromptType, Result, Settings, Storage, Tab,
};
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug)]
pub struct InMemoryStorage {
    prompts: RwLock<Vec<Prompt>>,
    projects: RwLock<Vec<Project>>,
    project_info: RwLock<HashMap<String, ProjectInfo>>,
    settings: RwLock<Settings>,
}

impl InMemoryStorage {
    #[must_use]
    pub fn new() -> Self {
        Self {
            prompts: RwLock::new(Vec::new()),
            projects: RwLock::new(Vec::new()),
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
        drop(prompts);

        if let Some(folder) = filter.folder {
            filtered.retain(|p| p.folder.as_deref() == Some(&folder));
        }

        if filter.project_filter {
            filtered.retain(|p| p.project_id == filter.project_id);
        }

        if let Some(branch) = filter.branch {
            filtered.retain(|p| p.branch.as_deref() == Some(&branch));
        }

        if let Some(staged) = filter.staged {
            filtered.retain(|p| p.staged == staged);
        }

        if let Some(tab) = filter.tab {
            match tab {
                Tab::Prompts => {
                    filtered.retain(|p| {
                        p.r#type == PromptType::Prompt && !p.is_archived && p.folder.is_some()
                    });
                }
                Tab::Canned => {
                    filtered.retain(|p| {
                        p.r#type == PromptType::Prompt && !p.is_archived && p.folder.is_none()
                    });
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

        // Sort by order_index ASC, created_at DESC (mimic DB behavior)
        filtered.sort_by(|a, b| match a.order_index.cmp(&b.order_index) {
            std::cmp::Ordering::Equal => b.created_at.cmp(&a.created_at),
            other => other,
        });

        Ok(filtered)
    }

    async fn save_prompt(&self, prompt: Prompt) -> Result<()> {
        let mut prompts = self.prompts.write().await;
        if let Some(p) = prompts.iter_mut().find(|p| p.id == prompt.id) {
            *p = prompt;
        } else {
            prompts.push(prompt);
        }
        drop(prompts);
        Ok(())
    }

    async fn save_prompts(&self, prompts_to_save: Vec<Prompt>) -> Result<()> {
        let mut prompts = self.prompts.write().await;
        for p_to_save in prompts_to_save {
            if let Some(p) = prompts.iter_mut().find(|p| p.id == p_to_save.id) {
                *p = p_to_save;
            } else {
                prompts.push(p_to_save);
            }
        }
        drop(prompts);
        Ok(())
    }

    async fn delete_prompt(&self, id: Uuid) -> Result<()> {
        let mut prompts = self.prompts.write().await;
        prompts.retain(|p| p.id != id);
        drop(prompts);
        Ok(())
    }

    async fn get_projects(&self) -> Result<Vec<Project>> {
        Ok(self.projects.read().await.clone())
    }

    async fn save_project(&self, project: Project) -> Result<()> {
        let mut projects = self.projects.write().await;
        if let Some(p) = projects.iter_mut().find(|p| p.id == project.id) {
            *p = project;
        } else {
            projects.push(project);
        }
        drop(projects);
        Ok(())
    }

    async fn delete_project(&self, id: Uuid) -> Result<()> {
        let mut projects = self.projects.write().await;
        projects.retain(|p| p.id != id);
        drop(projects);

        let mut prompts = self.prompts.write().await;
        for p in prompts.iter_mut() {
            if p.project_id == Some(id) {
                p.project_id = None;
            }
        }
        drop(prompts);
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
        drop(s);
        Ok(())
    }

    async fn get_data_version(&self) -> Result<u32> {
        Ok(0)
    }

    async fn get_all_data(&self) -> Result<contracts::DatabaseExport> {
        Ok(contracts::DatabaseExport {
            prompts: self.prompts.read().await.clone(),
            projects: self.projects.read().await.clone(),
            project_info: self.project_info.read().await.clone(),
            settings: self.settings.read().await.clone(),
        })
    }

    async fn restore_all_data(&self, data: contracts::DatabaseExport) -> Result<()> {
        *self.prompts.write().await = data.prompts;
        *self.projects.write().await = data.projects;
        *self.project_info.write().await = data.project_info;
        *self.settings.write().await = data.settings;
        Ok(())
    }
}
