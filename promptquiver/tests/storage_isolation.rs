use contracts::{Prompt, PromptType, Storage};
use infra::FileSystemStorage;
use std::sync::Arc;
use tempfile::tempdir;

#[tokio::test]
async fn test_storage_isolation_per_folder() {
    let base_temp = tempdir().unwrap();
    let base_dir = base_temp.path().to_path_buf();
    let storage = Arc::new(FileSystemStorage::new(Some(base_dir)));

    // Create two physical directories to allow canonicalization to work correctly
    let dir_a = base_temp.path().join("project_a");
    let dir_b = base_temp.path().join("project_b");
    std::fs::create_dir(&dir_a).unwrap();
    std::fs::create_dir(&dir_b).unwrap();

    let project_a = std::fs::canonicalize(&dir_a).unwrap().to_string_lossy().into_owned();
    let project_b = std::fs::canonicalize(&dir_b).unwrap().to_string_lossy().into_owned();

    // 1. Save local content to project A
    let prompt_a = Prompt::new("Prompt A".to_string(), PromptType::Prompt, None, None);
    let note_a = Prompt::new("Note A".to_string(), PromptType::Note, None, None);
    storage.save_project_prompts(&project_a, vec![prompt_a.clone()]).await.unwrap();
    storage.save_project_notes(&project_a, vec![note_a.clone()]).await.unwrap();

    // 2. Save global content
    let snippet_global = Prompt::new("Snippet Global".to_string(), PromptType::Snippet, None, Some("sg".to_string()));
    storage.save_global_snippets(vec![snippet_global.clone()]).await.unwrap();

    // 3. Verify Project A has its content AND global content
    assert_eq!(storage.get_project_prompts(&project_a).await.unwrap().len(), 1);
    assert_eq!(storage.get_project_notes(&project_a).await.unwrap().len(), 1);
    assert_eq!(storage.get_global_snippets().await.unwrap().len(), 1);

    // 4. Verify Project B is empty for local content but has global content
    assert_eq!(storage.get_project_prompts(&project_b).await.unwrap().len(), 0);
    assert_eq!(storage.get_project_notes(&project_b).await.unwrap().len(), 0);
    assert_eq!(storage.get_global_snippets().await.unwrap().len(), 1);

    // 5. Save content to Project B and verify Project A remains unchanged
    let prompt_b = Prompt::new("Prompt B".to_string(), PromptType::Prompt, None, None);
    storage.save_project_prompts(&project_b, vec![prompt_b.clone()]).await.unwrap();
    
    assert_eq!(storage.get_project_prompts(&project_a).await.unwrap().len(), 1);
    assert_eq!(storage.get_project_prompts(&project_a).await.unwrap()[0].text, "Prompt A");
    assert_eq!(storage.get_project_prompts(&project_b).await.unwrap().len(), 1);
    assert_eq!(storage.get_project_prompts(&project_b).await.unwrap()[0].text, "Prompt B");
}
