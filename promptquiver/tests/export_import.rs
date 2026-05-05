mod common;
use common::setup_app;
use contracts::{Prompt, PromptType, DatabaseExport, Storage, AppService};
use infra::SqliteStorage;
use std::sync::Arc;
use tempfile::tempdir;

#[tokio::test]
async fn test_export_import_roundtrip() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test_export.db");
    let storage = Arc::new(SqliteStorage::new(db_path));
    
    let (mut app, _, clipboard, _) = setup_app();
    app.storage = storage.clone();
    let service = Arc::new(infra::RealAppService::new(storage.clone(), clipboard.clone()));
    app.service = service;
    let test_path = app.nav.current_project_path();

    // 1. Seed data
    let p1 = Prompt::new("Prompt 1".to_string(), PromptType::Prompt, Some(test_path.clone()), None, Some("P1".to_string()), None);
    let p2 = Prompt::new("Note 1".to_string(), PromptType::Note, Some(test_path.clone()), None, Some("N1".to_string()), None);
    let mut p3 = Prompt::new("Archived".to_string(), PromptType::Prompt, Some(test_path.clone()), None, Some("A1".to_string()), None);
    p3.is_archived = true;

    storage.save_prompt(p1.clone()).await.unwrap();
    storage.save_prompt(p2.clone()).await.unwrap();
    storage.save_prompt(p3.clone()).await.unwrap();

    // 2. Export (including archived)
    let toml_data = app.service.export_data(true).await.unwrap();
    
    // 3. Import into a NEW database
    let db_path_2 = dir.path().join("test_import.db");
    let storage_2 = Arc::new(SqliteStorage::new(db_path_2));
    let service_2 = Arc::new(infra::RealAppService::new(storage_2.clone(), app.clipboard.clone()));
    
    service_2.import_data(&toml_data).await.unwrap();
    
    // 4. Verify
    let filter = contracts::PromptFilter::default();
    let imported = storage_2.get_prompts(filter).await.unwrap();
    
    assert_eq!(imported.len(), 3);
    assert!(imported.iter().any(|p| p.text == "Prompt 1"));
    assert!(imported.iter().any(|p| p.text == "Note 1"));
    assert!(imported.iter().any(|p| p.text == "Archived" && p.is_archived));
}

#[tokio::test]
async fn test_export_without_archived() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test_export_no_archived.db");
    let storage = Arc::new(SqliteStorage::new(db_path));
    
    let (mut app, _, clipboard, _) = setup_app();
    app.storage = storage.clone();
    let service = Arc::new(infra::RealAppService::new(storage.clone(), clipboard.clone()));
    app.service = service;
    let test_path = app.nav.current_project_path();

    let p1 = Prompt::new("Prompt 1".to_string(), PromptType::Prompt, Some(test_path.clone()), None, None, None);
    let mut p2 = Prompt::new("Archived".to_string(), PromptType::Prompt, Some(test_path.clone()), None, None, None);
    p2.is_archived = true;

    storage.save_prompt(p1).await.unwrap();
    storage.save_prompt(p2).await.unwrap();

    let toml_data = app.service.export_data(false).await.unwrap();
    let export: DatabaseExport = toml::from_str(&toml_data).unwrap();
    
    assert_eq!(export.prompts.len(), 1);
    assert_eq!(export.prompts[0].text, "Prompt 1");
}
