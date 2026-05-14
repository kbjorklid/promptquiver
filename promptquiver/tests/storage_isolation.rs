use contracts::{Prompt, PromptFilter, PromptType, Storage, Tab};
use std::sync::Arc;

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_storage_isolation_per_folder() {
    let storage = Arc::new(infra::InMemoryStorage::new());

    let project_a = "/path/a";
    let project_b = "/path/b";

    // 1. Save local content to project A
    let prompt_a = Prompt::new(
        "Prompt A".to_string(),
        PromptType::Prompt,
        Some(project_a.to_string()),
        None,
        None,
        None,
    );
    let note_a = Prompt::new(
        "Note A".to_string(),
        PromptType::Note,
        Some(project_a.to_string()),
        None,
        None,
        None,
    );
    storage.save_prompt(prompt_a.clone()).await.unwrap();
    storage.save_prompt(note_a.clone()).await.unwrap();

    // 2. Save global content
    let snippet_global = Prompt::new(
        "Snippet Global".to_string(),
        PromptType::Snippet,
        None,
        None,
        Some("sg".to_string()),
        None,
    );
    storage.save_prompt(snippet_global.clone()).await.unwrap();

    // 3. Verify Project A has its content AND global content
    assert_eq!(
        storage
            .get_prompts(PromptFilter {
                folder: Some(project_a.to_string()),
                tab: Some(Tab::Prompts),
                ..Default::default()
            })
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        storage
            .get_prompts(PromptFilter {
                folder: Some(project_a.to_string()),
                tab: Some(Tab::Notes),
                ..Default::default()
            })
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        storage
            .get_prompts(PromptFilter { tab: Some(Tab::Snippets), ..Default::default() })
            .await
            .unwrap()
            .len(),
        1
    );

    // 4. Verify Project B is empty for local content but has global content
    assert_eq!(
        storage
            .get_prompts(PromptFilter {
                folder: Some(project_b.to_string()),
                tab: Some(Tab::Prompts),
                ..Default::default()
            })
            .await
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        storage
            .get_prompts(PromptFilter {
                folder: Some(project_b.to_string()),
                tab: Some(Tab::Notes),
                ..Default::default()
            })
            .await
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        storage
            .get_prompts(PromptFilter { tab: Some(Tab::Snippets), ..Default::default() })
            .await
            .unwrap()
            .len(),
        1
    );

    // 5. Save content to Project B and verify Project A remains unchanged
    let prompt_b = Prompt::new(
        "Prompt B".to_string(),
        PromptType::Prompt,
        Some(project_b.to_string()),
        None,
        None,
        None,
    );
    storage.save_prompt(prompt_b.clone()).await.unwrap();

    let stored_a = storage
        .get_prompts(PromptFilter {
            folder: Some(project_a.to_string()),
            tab: Some(Tab::Prompts),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(stored_a.len(), 1);
    assert_eq!(stored_a[0].text, "Prompt A");

    let stored_b = storage
        .get_prompts(PromptFilter {
            folder: Some(project_b.to_string()),
            tab: Some(Tab::Prompts),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(stored_b.len(), 1);
    assert_eq!(stored_b[0].text, "Prompt B");
}
