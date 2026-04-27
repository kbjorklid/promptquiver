use promptquiver::app::App;
use infra::{InMemoryStorage, MockClipboard, MockGit};
use std::sync::Arc;

pub const TEST_PATH: &str = "test_project";

pub fn setup_app() -> (App<'static>, Arc<InMemoryStorage>, Arc<MockClipboard>, Arc<MockGit>) {
    let storage = Arc::new(InMemoryStorage::new());
    let clipboard = Arc::new(MockClipboard::new());
    let git = Arc::new(MockGit::new(None));
    let mut app = App::new(storage.clone(), clipboard.clone(), git.clone());
    app.current_path = TEST_PATH.to_string();
    (app, storage, clipboard, git)
}
