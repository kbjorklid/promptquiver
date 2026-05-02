use contracts::Tab;
mod common;

#[tokio::test]
async fn test_draft_identification_and_display() {
    let (mut app, _, _, _) = common::setup_app();
    
    // 1. Prompt with -- Draft Title
    app.service.save_item(common::TEST_PATH, Tab::Prompts, "-- Draft Fix welcome email\n\nContent".to_string(), Some("Draft Fix welcome email".to_string()), None, None, None, None).await.unwrap();
    
    // 2. Prompt with [Draft] in title
    app.service.save_item(common::TEST_PATH, Tab::Prompts, "-- Fix welcome email [Draft]\n\nContent".to_string(), Some("Fix welcome email [Draft]".to_string()), None, None, None, None).await.unwrap();
    
    // 3. Prompt with [DRAFT] (case insensitive)
    app.service.save_item(common::TEST_PATH, Tab::Prompts, "-- [draft] Fix welcome email\n\nContent".to_string(), Some("[draft] Fix welcome email".to_string()), None, None, None, None).await.unwrap();

    // 4. Regular prompt
    app.service.save_item(common::TEST_PATH, Tab::Prompts, "-- Fix welcome email\n\nContent".to_string(), Some("Fix welcome email".to_string()), None, None, None, None).await.unwrap();

    app.load_prompts().await.unwrap();
    
    // Check identification logic (to be implemented)
    let prompts = &app.nav.prompts;
    assert_eq!(prompts.len(), 4);
    
    // We'll test the actual logic in contracts::Processor
}

#[tokio::test]
async fn test_cannot_stage_draft_prompt() {
    let (mut app, _, _, _) = common::setup_app();
    
    // Add a draft prompt
    app.service.save_item(common::TEST_PATH, Tab::Prompts, "-- Draft My Prompt\n\nContent".to_string(), Some("Draft My Prompt".to_string()), None, None, None, None).await.unwrap();
    app.load_prompts().await.unwrap();
    
    // Try to stage it
    let prompt = app.nav.prompts[0].clone();
    let res = app.service.stage_item(common::TEST_PATH, Tab::Prompts, prompt).await;
    
    // Currently this will succeed because we haven't implemented the block.
    // The test should eventually check for an error.
    assert!(res.is_err(), "Should not be able to stage a draft prompt");
}

#[tokio::test]
async fn test_single_line_draft() {
    let (mut app, _, _, _) = common::setup_app();
    
    // Single line draft prompt
    app.service.save_item(common::TEST_PATH, Tab::Prompts, "-- Draft Single Line".to_string(), Some("Draft Single Line".to_string()), None, None, None, None).await.unwrap();
    app.load_prompts().await.unwrap();
    
    // Try to stage it
    let prompt = app.nav.prompts[0].clone();
    let res = app.service.stage_item(common::TEST_PATH, Tab::Prompts, prompt).await;
    
    assert!(res.is_err(), "Single line draft should be blocked from staging");
}
