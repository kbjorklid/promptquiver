mod common;
use common::setup_app;
use contracts::{Storage, Clipboard, Tab, Prompt, PromptType};

#[tokio::test]
async fn test_note_staging_alias() {
    let (mut app, storage, clipboard, _) = setup_app();
    
    let n1 = Prompt::new("N1".to_string(), PromptType::Note, None, None);
    storage.save_project_notes(common::TEST_PATH, vec![n1]).await.unwrap();

    app.set_tab(Tab::Notes);
    app.load_prompts().await.unwrap();

    // Now, this should NOT stage the note, but SHOULD copy it
    app.stage_selected().await.unwrap();
    
    assert!(!app.prompts[0].staged, "Notes should not be stageable");
    assert_eq!(clipboard.paste().await.unwrap(), "N1", "Should still copy to clipboard");
}

#[tokio::test]
async fn test_snippet_staging_alias() {
    let (mut app, storage, clipboard, _) = setup_app();
    
    let s1 = Prompt::new("S1".to_string(), PromptType::Snippet, None, None);
    storage.save_global_snippets(vec![s1]).await.unwrap();

    app.set_tab(Tab::Snippets);
    app.load_prompts().await.unwrap();

    // Now, this should NOT stage the snippet, but SHOULD copy it
    app.stage_selected().await.unwrap();
    
    assert!(!app.prompts[0].staged, "Snippets should not be stageable");
    assert_eq!(clipboard.paste().await.unwrap(), "S1", "Should still copy to clipboard");
}

#[tokio::test]
async fn test_prompt_staging_does_not_archive_notes_or_snippets() {
    let (mut app, storage, _, _) = setup_app();
    
    // Setup a staged prompt, a note (not staged), and a snippet (not staged)
    // Actually, we can't stage notes anymore, so let's try to "force" it in storage
    // to see if the archive logic ignores it now.
    
    let mut n1 = Prompt::new("N1".to_string(), PromptType::Note, None, None);
    n1.staged = true; // Forced staged state in storage
    storage.save_project_notes(common::TEST_PATH, vec![n1.clone()]).await.unwrap();
    
    let p1 = Prompt::new("P1".to_string(), PromptType::Prompt, None, None);
    storage.save_project_prompts(common::TEST_PATH, vec![p1]).await.unwrap();

    app.set_tab(Tab::Prompts);
    app.load_prompts().await.unwrap();

    // Stage the prompt
    app.stage_selected().await.unwrap();
    
    // Check if N1 was archived (it should NOT be, because we removed notes from staging logic)
    let notes = storage.get_project_notes(common::TEST_PATH).await.unwrap();
    assert_eq!(notes.len(), 1, "Note should still be in notes list");
    assert!(notes[0].staged, "Note staged status preserved (though UI hides it)");

    let archive = storage.get_project_archive(common::TEST_PATH).await.unwrap();
    assert!(archive.is_empty(), "Archive should be empty");
}
