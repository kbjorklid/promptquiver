mod common;
use common::setup_app;
use contracts::Tab;
use promptquiver::app::AppMessage;

#[tokio::test]
async fn test_tab_visibility_toggle() {
    let (mut app, _, _, _) = setup_app();

    // 1. Initially all tabs are visible (conceptually)
    // We navigate to Settings
    app.set_tab(Tab::Settings);
    app.load_prompts().await.unwrap();

    // 2. Toggle visibility for Canned tab (index 1 in Tab::all())
    app.nav.selected_index = 1; 
    app.handle_message(AppMessage::ToggleSetting).await.unwrap();

    assert_eq!(app.settings.tab_visibility.get(&Tab::Canned), Some(&false));

    // 3. Try to navigate to Canned tab using next_tab
    // It should skip Canned and go from Prompts to Notes
    app.set_tab(Tab::Prompts);
    app.next_tab();
    
    // If bug is present, it will be Canned. If fixed, it should be Notes.
    assert_ne!(app.nav.active_tab, Tab::Canned, "Canned tab should be hidden and skipped");
    assert_eq!(app.nav.active_tab, Tab::Notes, "Should skip Canned and go to Notes");
}

#[tokio::test]
async fn test_settings_tab_always_visible() {
    let (mut app, _, _, _) = setup_app();

    // Attempt to hide Settings tab (index 5 in Tab::all())
    app.set_tab(Tab::Settings);
    app.nav.selected_index = 5; 
    app.handle_message(AppMessage::ToggleSetting).await.unwrap();

    // Even if toggled (if possible), it should still be in visible_tabs
    // For now, let's just check if it's still accessible
    app.set_tab(Tab::Prompts);
    for _ in 0..10 {
        app.next_tab();
        if app.nav.active_tab == Tab::Settings {
            return; // Success
        }
    }
    
    panic!("Settings tab should always be reachable");
}
