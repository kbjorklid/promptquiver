pub mod clipboard;
pub mod git;
pub mod storage;
pub mod service;

pub use clipboard::{MockClipboard, RealClipboard};
pub use git::{MockGit, RealGit};
pub use storage::{InMemoryStorage, SqliteStorage};
pub use service::RealAppService;

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::{Prompt, PromptType, PromptFilter, Tab, Project, ProjectInfo, Settings, Storage, Clipboard, Git, AppService};
    use uuid::Uuid;
    use chrono::Utc;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_in_memory_storage() {
        let storage = InMemoryStorage::default();
        let project_id = Uuid::new_v4();
        let prompt = Prompt::new("test".to_string(), PromptType::Prompt, Some("path".to_string()), Some("main".to_string()), None, Some(project_id));

        storage.save_prompt(prompt.clone()).await.unwrap();
        
        // Test various filters
        let loaded = storage.get_prompts(PromptFilter { folder: Some("path".to_string()), ..Default::default() }).await.unwrap();
        assert_eq!(loaded.len(), 1);

        let filtered_project = storage.get_prompts(PromptFilter { project_id: Some(project_id), project_filter: true, ..Default::default() }).await.unwrap();
        assert_eq!(filtered_project.len(), 1);

        let filtered_branch = storage.get_prompts(PromptFilter { branch: Some("main".to_string()), ..Default::default() }).await.unwrap();
        assert_eq!(filtered_branch.len(), 1);

        // Test all tabs
        for tab in [Tab::Prompts, Tab::Canned, Tab::Notes, Tab::Snippets, Tab::Archive, Tab::Settings] {
            let _ = storage.get_prompts(PromptFilter { tab: Some(tab), ..Default::default() }).await.unwrap();
        }

        // Test save_prompts
        let prompts = vec![
            Prompt::new("p1".to_string(), PromptType::Prompt, None, None, None, None),
            Prompt::new("p2".to_string(), PromptType::Prompt, None, None, None, None),
        ];
        storage.save_prompts(prompts).await.unwrap();
        let all = storage.get_prompts(PromptFilter::default()).await.unwrap();
        assert!(all.len() >= 2);

        // Test project info
        let info = ProjectInfo {
            path: "some/path".to_string(),
        };
        storage.save_project_info("folder", info.clone()).await.unwrap();
        let loaded_info = storage.get_project_info("folder").await.unwrap();
        assert_eq!(loaded_info.path, "some/path");

        // Test settings
        let settings = Settings {
            enable_claude_commands: true,
            ..Settings::default()
        };
        storage.save_settings(settings.clone()).await.unwrap();
        let loaded_settings = storage.get_settings().await.unwrap();
        assert!(loaded_settings.enable_claude_commands);
        
        assert_eq!(storage.get_data_version().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_mock_clipboard() {
        let clipboard = MockClipboard::default();
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

        let project_id = Uuid::new_v4();
        let prompt = Prompt::new("sqlite test".to_string(), contracts::PromptType::Prompt, Some("/path/to/project".to_string()), Some("main".to_string()), None, Some(project_id));
        
        storage.save_prompt(prompt.clone()).await.unwrap();

        // Test various filters
        let loaded = storage.get_prompts(PromptFilter { folder: Some("/path/to/project".to_string()), ..Default::default() }).await.unwrap();
        assert_eq!(loaded.len(), 1);

        let filtered_project = storage.get_prompts(PromptFilter { project_id: Some(project_id), project_filter: true, ..Default::default() }).await.unwrap();
        assert_eq!(filtered_project.len(), 1);

        let filtered_branch = storage.get_prompts(PromptFilter { branch: Some("main".to_string()), ..Default::default() }).await.unwrap();
        assert_eq!(filtered_branch.len(), 1);

        // Test all tabs in SQLite
        for tab in [Tab::Prompts, Tab::Canned, Tab::Notes, Tab::Snippets, Tab::Archive, Tab::Settings] {
            let _ = storage.get_prompts(PromptFilter { tab: Some(tab), ..Default::default() }).await.unwrap();
        }

        // Test update
        let mut updated = loaded[0].clone();
        updated.text = "updated".to_string();
        storage.save_prompt(updated).await.unwrap();
        
        let loaded_updated = storage.get_prompts(PromptFilter { folder: Some("/path/to/project".to_string()), ..Default::default() }).await.unwrap();
        assert_eq!(loaded_updated.len(), 1);
        assert_eq!(loaded_updated[0].text, "updated");
        
        // Test save_prompts (plural)
        let prompts = vec![
            Prompt::new("bulk1".to_string(), PromptType::Prompt, None, None, None, None),
            Prompt::new("bulk2".to_string(), PromptType::Prompt, None, None, None, None),
        ];
        storage.save_prompts(prompts).await.unwrap();
        let bulk_loaded = storage.get_prompts(PromptFilter::default()).await.unwrap();
        assert!(bulk_loaded.iter().any(|p| p.text == "bulk1"));
        assert!(bulk_loaded.iter().any(|p| p.text == "bulk2"));

        // Test project info
        let info = ProjectInfo {
            path: "project/path".to_string(),
        };
        storage.save_project_info("folder1", info.clone()).await.unwrap();
        let loaded_info = storage.get_project_info("folder1").await.unwrap();
        assert_eq!(loaded_info.path, "project/path");

        // Test settings
        let settings = Settings {
            enable_claude_commands: true,
            ..Settings::default()
        };
        storage.save_settings(settings.clone()).await.unwrap();
        let loaded_settings = storage.get_settings().await.unwrap();
        assert!(loaded_settings.enable_claude_commands);

        // Test projects
        let project = Project {
            id: Uuid::new_v4(),
            title: "My Project".to_string(),
            created_at: Utc::now(),
        };
        storage.save_project(project.clone()).await.unwrap();
        let projects = storage.get_projects().await.unwrap();
        assert!(projects.iter().any(|p| p.title == "My Project"));

        // Test project deletion and prompt association
        let mut associated_prompt = Prompt::new("associated".to_string(), PromptType::Prompt, None, None, None, None);
        associated_prompt.project_id = Some(project.id);
        storage.save_prompt(associated_prompt.clone()).await.unwrap();
        
        storage.delete_project(project.id).await.unwrap();
        let projects_after = storage.get_projects().await.unwrap();
        assert!(!projects_after.iter().any(|p| p.id == project.id));
        
        let prompt_after = storage.get_prompts(PromptFilter::default()).await.unwrap()
            .into_iter().find(|p| p.text == "associated").unwrap();
        assert_eq!(prompt_after.project_id, None);

        // Test data version
        let version = storage.get_data_version().await.unwrap();
        assert!(version > 0);

        // Test delete
        storage.delete_prompt(prompt.id).await.unwrap();
        let loaded_deleted = storage.get_prompts(PromptFilter { folder: Some("/path/to/project".to_string()), ..Default::default() }).await.unwrap();
        assert_eq!(loaded_deleted.len(), 0);
    }

    #[tokio::test]
    async fn test_real_git_logic() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path();
        
        // Initialize a git repo
        let output = std::process::Command::new("git")
            .arg("init")
            .current_dir(path)
            .output();
        
        if output.is_err() {
            return; // Git not installed
        }
        
        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(path)
            .output()
            .unwrap();
        
        std::process::Command::new("git")
            .args(["config", "user.name", "test"])
            .current_dir(path)
            .output()
            .unwrap();

        // Create a commit so we have a HEAD
        std::fs::write(path.join("file.txt"), "hello").unwrap();
        std::process::Command::new("git")
            .arg("add")
            .arg(".")
            .current_dir(path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(path)
            .output()
            .unwrap();

        // Create and switch to a branch
        std::process::Command::new("git")
            .args(["checkout", "-b", "feature-test"])
            .current_dir(path)
            .output()
            .unwrap();

        let git = RealGit;
        let branch = git.get_current_branch(path.to_str().unwrap()).await.unwrap();
        assert_eq!(branch, Some("feature-test".to_string()));
    }

    #[tokio::test]
    async fn test_real_app_service_search_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path();
        
        std::fs::create_dir_all(path.join("subdir")).unwrap();
        std::fs::write(path.join("file1.txt"), "content").unwrap();
        std::fs::write(path.join("subdir/file2.txt"), "content").unwrap();

        let storage = Arc::new(InMemoryStorage::default());
        let clipboard = Arc::new(MockClipboard::default());
        let service = RealAppService::new(storage, clipboard);

        let results = service.search_files(path.to_str().unwrap(), "file").await.unwrap();
        // Should find file1.txt, subdir/file2.txt, and subdir/
        assert!(results.len() >= 2);
        
        let results_subdir = service.search_files(path.to_str().unwrap(), "subdir").await.unwrap();
        assert!(results_subdir.iter().any(|p| p.name.as_deref() == Some("subdir/")));
    }

    #[tokio::test]
    async fn test_real_clipboard_construction() {
        let clipboard = RealClipboard;
        // We can't easily test copy/paste in headless environment, 
        // but we can test it doesn't panic on construction.
        let debug_str = format!("{clipboard:?}");
        assert!(debug_str.contains("RealClipboard"));
    }
}
