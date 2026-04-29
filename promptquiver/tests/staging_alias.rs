mod common;
use common::setup_app;
use contracts::{Storage, Clipboard, Tab, Prompt, PromptType, PromptFilter};

#[tokio::test]
async fn test_note_staging_alias() {
    let (mut app, storage, clipboard, _) = setup_app();
    
    let n1 = Prompt::new("N1".to_string(), PromptType::Note, Some(common::TEST_PATH.to_string()), None, None);
    storage.save_prompt(n1).await.unwrap();

    app.set_tab(Tab::Notes);
    app.load_prompts().await.unwrap();

    // Now, this should NOT stage the note, but SHOULD copy it
    app.stage_selected().await.unwrap();
    
    assert!(!app.nav.prompts[0].staged, "Notes should not be stageable");
    assert_eq!(clipboard.paste().await.unwrap(), "N1", "Should still copy to clipboard");
}

#[tokio::test]
async fn test_snippet_staging_alias() {
    let (mut app, storage, clipboard, _) = setup_app();
    
    let s1 = Prompt::new("S1".to_string(), PromptType::Snippet, None, None, None);
    storage.save_prompt(s1).await.unwrap();

    app.set_tab(Tab::Snippets);
    app.load_prompts().await.unwrap();

    // Now, this should NOT stage the snippet, but SHOULD copy it
    app.stage_selected().await.unwrap();
    
    assert!(!app.nav.prompts[0].staged, "Snippets should not be stageable");
    assert_eq!(clipboard.paste().await.unwrap(), "S1", "Should still copy to clipboard");
}

#[tokio::test]
async fn test_prompt_staging_does_not_archive_notes_or_snippets() {
    let (mut app, storage, _, _) = setup_app();
    
    // Setup a staged prompt, a note (not staged), and a snippet (not staged)
    
    let mut n1 = Prompt::new("N1".to_string(), PromptType::Note, Some(common::TEST_PATH.to_string()), None, None);
    n1.staged = true; // Forced staged state in storage
    storage.save_prompt(n1.clone()).await.unwrap();
    
    let p1 = Prompt::new("P1".to_string(), PromptType::Prompt, Some(common::TEST_PATH.to_string()), None, None);
    storage.save_prompt(p1).await.unwrap();

    app.set_tab(Tab::Prompts);
    app.load_prompts().await.unwrap();

    // Stage the prompt
    app.stage_selected().await.unwrap();
    
    // Check if N1 was archived (it should NOT be, because we removed notes from staging logic)
    let notes = storage.get_prompts(PromptFilter { folder: Some(common::TEST_PATH.to_string()), tab: Some(Tab::Notes), ..Default::default() }).await.unwrap();
    assert_eq!(notes.len(), 1, "Note should still be in notes list");
    assert!(notes[0].staged, "Note staged status preserved (though UI hides it)");

    let archive = storage.get_prompts(PromptFilter { folder: Some(common::TEST_PATH.to_string()), tab: Some(Tab::Archive), ..Default::default() }).await.unwrap();
    assert!(archive.is_empty(), "Archive should be empty");
}
