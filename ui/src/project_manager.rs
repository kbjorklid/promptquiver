use contracts::{Project, Storage, Result, Settings};
use std::sync::Arc;
use uuid::Uuid;
use chrono;

#[derive(Debug, Default)]
pub struct ProjectManager {
    pub projects: Vec<Project>,
    pub new_project_name: String,
    pub selecting_startup_project: bool,
    pub active_project_id: Option<Uuid>,
    pub project_list_state: ratatui::widgets::ListState,
}

impl ProjectManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads projects from storage.
    ///
    /// # Errors
    /// Returns an error if the storage cannot be accessed.
    pub async fn load_projects(&mut self, storage: &Arc<dyn Storage>) -> Result<()> {
        self.projects = storage.get_projects().await?;
        Ok(())
    }

    /// Adds a new project.
    ///
    /// # Errors
    /// Returns an error if the project cannot be saved.
    pub async fn add_project(&mut self, name: &str, storage: &Arc<dyn Storage>, settings: &mut Settings) -> Result<Project> {
        let project = Project {
            id: Uuid::new_v4(),
            title: name.to_string(),
            created_at: chrono::Utc::now(),
        };
        storage.save_project(project.clone()).await?;
        self.active_project_id = Some(project.id);
        settings.last_active_project_id = Some(project.id);
        storage.save_settings(settings.clone()).await?;
        self.load_projects(storage).await?;
        Ok(project)
    }

    /// Deletes a project by ID.
    ///
    /// # Errors
    /// Returns an error if the project cannot be deleted.
    pub async fn delete_project(&mut self, id: Uuid, storage: &Arc<dyn Storage>, settings: &mut Settings) -> Result<()> {
        storage.delete_project(id).await?;
        if self.active_project_id == Some(id) {
            self.active_project_id = None;
            settings.last_active_project_id = None;
            storage.save_settings(settings.clone()).await?;
        }
        self.load_projects(storage).await?;
        Ok(())
    }

    pub const fn select_project(&mut self, id: Option<Uuid>, settings: &mut Settings) -> bool {
        if self.selecting_startup_project {
            self.selecting_startup_project = false;
            settings.specific_project_id = id;
            true
        } else {
            self.active_project_id = id;
            settings.last_active_project_id = id;
            false
        }
    }
}
