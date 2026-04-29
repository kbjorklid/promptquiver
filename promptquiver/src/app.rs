use contracts::{Clipboard, Git, Storage, Tab};
use ratatui_toaster::{ToastBuilder, ToastType, ToastEngine, ToastMessage, ToastPosition};
use std::sync::Arc;
pub use ui::{Mode, AppMessage, UpdateContext, ListModule, EditorModule};

use std::fmt;

pub struct App<'a> {
    pub storage: Arc<dyn Storage>,
    pub clipboard: Arc<dyn Clipboard>,
    pub git: Arc<dyn Git>,
    pub service: Arc<dyn contracts::AppService>,
    pub should_quit: bool,
    pub mode: Mode,
    pub nav: ListModule,
    pub editor: EditorModule<'a>,
    pub current_branch: Option<String>,
    pub toaster: Option<ToastEngine<ToastMessage>>,
    pub settings: contracts::Settings,
    pub last_notification_time: Option<std::time::Instant>,
    pub file_search_tx: Option<tokio::sync::mpsc::Sender<(String, String)>>,
}


impl fmt::Debug for App<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("App")
            .field("should_quit", &self.should_quit)
            .field("active_tab", &self.nav.active_tab)
            .field("prompts_count", &self.nav.prompts.len())
            .field("selected_index", &self.nav.selected_index)
            .field("mode", &self.mode)
            .field("current_branch", &self.current_branch)
            .field("autocomplete_open", &self.editor.autocomplete.open)
            .finish_non_exhaustive()
    }
}

impl App<'_> {
    pub async fn handle_message(&mut self, initial_msg: AppMessage) -> contracts::Result<()> {
        let mut msg = initial_msg;
        loop {
            let mut ctx = UpdateContext {
                storage: &self.storage,
                clipboard: &self.clipboard,
                git: &self.git,
                service: &self.service,
                settings: &mut self.settings,
                active_tab: self.nav.active_tab,
                selected_index: self.nav.selected_index,
            };

            let next_msg = match self.mode {
                Mode::Editor | Mode::ConfirmDiscard => {
                    self.editor.update(msg.clone(), &mut ctx, self.nav.current_path.clone(), &self.file_search_tx).await?
                }
                Mode::List | Mode::Move | Mode::Search | Mode::GlobalSearch | Mode::ThemePicker => {
                    self.nav.update(msg.clone(), &mut ctx).await?
                }
            };

            if let Some(m) = next_msg {
                msg = m;
                continue;
            }

            // Global/Final handlers
            match msg {
                AppMessage::Quit => self.quit(),
                AppMessage::Notify(m, kind) => self.notify(m, kind),
                AppMessage::EnterEditor(text, id) => self.enter_editor(text, id),
                AppMessage::EnterEditorBefore(text, index) => self.enter_editor_before(text, index),
                AppMessage::ExitEditor => self.exit_editor(),
                AppMessage::SaveEditor => self.save_editor().await?,
                AppMessage::SaveAndStageEditor => {
                    self.save_editor().await?;
                    msg = AppMessage::StageSelected;
                    continue;
                }
                AppMessage::EditSetting => self.edit_setting(),
                AppMessage::ConfirmDiscard => self.mode = Mode::ConfirmDiscard,
                AppMessage::CancelDiscard => self.mode = Mode::Editor,
                AppMessage::ToggleMoveMode => {
                    self.mode = if self.mode == Mode::Move { Mode::List } else { Mode::Move };
                }
                AppMessage::Search(_) => self.mode = Mode::Search,
                AppMessage::GlobalSearch(_) => self.mode = Mode::GlobalSearch,
                AppMessage::SearchInput(key) if key.code == crossterm::event::KeyCode::Esc || key.code == crossterm::event::KeyCode::Enter => {
                    self.mode = Mode::List;
                }
                AppMessage::GlobalSearchInput(key) if key.code == crossterm::event::KeyCode::Esc || key.code == crossterm::event::KeyCode::Enter => {
                    self.mode = Mode::List;
                }
                AppMessage::ThemePickerInput(key) if key.code == crossterm::event::KeyCode::Esc || key.code == crossterm::event::KeyCode::Enter || key.code == crossterm::event::KeyCode::Char(' ') => {
                    self.mode = Mode::List;
                }
                AppMessage::SelectTheme => self.mode = Mode::ThemePicker,
                _ => {}
            }
            break;
        }

        Ok(())
    }

    #[must_use]
    pub fn new(
        storage: Arc<dyn Storage>,
        clipboard: Arc<dyn Clipboard>,
        git: Arc<dyn Git>,
        service: Arc<dyn contracts::AppService>,
    ) -> Self {
        Self {
            storage,
            clipboard,
            git,
            service,
            should_quit: false,
            mode: Mode::List,
            nav: ListModule::default(),
            editor: EditorModule::default(),
            current_branch: None,
            toaster: None,
            settings: contracts::Settings::default(),
            last_notification_time: None,
            file_search_tx: None,
        }
    }

    pub fn tick(&mut self) {
        if let Some(last_time) = self.last_notification_time {
            if last_time.elapsed() >= std::time::Duration::from_secs(3) {
                if let Some(ref mut toaster) = self.toaster {
                    toaster.hide_toast();
                }
                self.last_notification_time = None;
            }
        }
    }

    pub fn notify(&mut self, message: impl Into<String>, kind: ToastType) {
        if let Some(ref mut toaster) = self.toaster {
            let toast = ToastBuilder::new(message.into().into())
                .toast_type(kind)
                .position(ToastPosition::BottomRight);
            toaster.show_toast(toast);
            self.last_notification_time = Some(std::time::Instant::now());
        }
    }

    pub const fn quit(&mut self) {
        self.should_quit = true;
    }

    pub async fn load_prompts(&mut self) -> contracts::Result<()> {
        self.settings = self.storage.get_settings().await.unwrap_or_default();
        self.nav.current_branch = self.current_branch.clone();
        self.nav.load_prompts(&self.storage).await
    }

    // Wrappers for tests
    pub fn next_tab(&mut self) { self.nav.next_tab(); }
    pub fn prev_tab(&mut self) { self.nav.prev_tab(); }
    pub fn set_tab(&mut self, tab: Tab) { self.nav.set_tab(tab); }
    pub fn move_down(&mut self) { self.nav.move_down(&self.settings); }
    pub fn move_up(&mut self) { self.nav.move_up(&self.settings); }
    pub fn move_to_top(&mut self) { self.nav.move_to_top(); }
    pub fn move_to_bottom(&mut self) { self.nav.move_to_bottom(&self.settings); }
    pub async fn stage_selected(&mut self) -> contracts::Result<()> {
        self.handle_message(AppMessage::StageSelected).await
    }
    pub async fn archive_selected(&mut self) -> contracts::Result<()> {
        self.handle_message(AppMessage::ArchiveSelected).await
    }
    pub async fn duplicate_selected(&mut self) -> contracts::Result<()> {
        self.handle_message(AppMessage::DuplicateSelected).await
    }
    pub async fn copy_selected(&mut self) -> contracts::Result<()> {
        if self.nav.prompts.is_empty() { return Ok(()); }
        let item = self.nav.prompts[self.nav.selected_index].clone();
        self.service.copy_item(&self.nav.current_project_path(), self.nav.active_tab, item).await?;
        self.load_prompts().await?;
        self.notify("Copied to clipboard!", ToastType::Success);
        Ok(())
    }
    pub async fn restore_selected(&mut self) -> contracts::Result<()> {
        self.handle_message(AppMessage::RestoreSelected).await
    }
    pub async fn update_autocomplete(&mut self) -> contracts::Result<()> {
        self.handle_message(AppMessage::UpdateAutocomplete).await
    }
    pub async fn select_suggestion(&mut self) -> contracts::Result<()> {
        self.handle_message(AppMessage::SelectSuggestion).await
    }

    pub fn enter_editor(&mut self, text: String, id: Option<uuid::Uuid>) {
        self.nav.search_query.clear();
        self.nav.global_search_query.clear();
        self.mode = Mode::Editor;
        
        let title = if self.nav.active_tab == Tab::Snippets {
            if let Some(id) = id {
                self.nav.prompts.iter().find(|p| p.id == id).and_then(|p| p.name.clone()).unwrap_or_default()
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        
        self.editor.enter(text, id, Some(title), self.nav.active_tab, None);
    }

    pub fn enter_editor_before(&mut self, text: String, index: usize) {
        self.nav.search_query.clear();
        self.nav.global_search_query.clear();
        self.mode = Mode::Editor;
        self.editor.enter(text, None, Some(String::new()), self.nav.active_tab, Some(index));
    }

    pub fn exit_editor(&mut self) {
        self.mode = Mode::List;
        self.editor.exit();
    }

    pub fn edit_setting(&mut self) {
        if self.nav.active_tab != Tab::Settings {
            return;
        }
        let tabs_len = Tab::all().len();
        let slash_len = self.settings.slash_commands.len();

        if self.nav.selected_index >= tabs_len && self.nav.selected_index < tabs_len + slash_len {
            // Edit existing Slash Command
            let idx = self.nav.selected_index - tabs_len;
            let text = self.settings.slash_commands[idx].clone();
            self.mode = Mode::Editor;
            self.editor.enter(text, None, None, self.nav.active_tab, None);
        } else if self.nav.selected_index == tabs_len + slash_len {
            // Add New Slash Command
            self.mode = Mode::Editor;
            self.editor.enter(String::new(), None, None, self.nav.active_tab, None);
        }
    }

    pub async fn save_editor(&mut self) -> contracts::Result<()> {
        let text = self.editor.textarea.lines().join("\n");
        let path = self.nav.current_project_path();

        if self.nav.active_tab == Tab::Settings {
            let tabs_len = Tab::all().len();
            let slash_len = self.settings.slash_commands.len();

            let re = regex::Regex::new("^[a-zA-Z0-9_-]+$").unwrap();
            let trimmed = text.trim();
            if !trimmed.is_empty() && !re.is_match(trimmed) {
                self.notify("Slash command must match [a-zA-Z0-9_-]+", ToastType::Error);
                return Ok(());
            }

            if self.nav.selected_index >= tabs_len && self.nav.selected_index < tabs_len + slash_len {
                // Update existing
                let idx = self.nav.selected_index - tabs_len;
                self.settings.slash_commands[idx] = trimmed.to_string();
                self.storage.save_settings(self.settings.clone()).await?;
            } else if self.nav.selected_index == tabs_len + slash_len {
                // Add new
                let new_cmd = trimmed.to_string();
                if !new_cmd.is_empty() {
                    self.settings.slash_commands.push(new_cmd);
                    self.storage.save_settings(self.settings.clone()).await?;
                }
            }
            self.exit_editor();
            self.notify("Settings saved!", ToastType::Success);
            return Ok(());
        }

        let title = if self.nav.active_tab == Tab::Snippets {
            let t = self.editor.title_textarea.lines().join("");
            let re = regex::Regex::new("^[a-zA-Z0-9_-]+$").unwrap();
            if !re.is_match(&t) {
                self.notify("Snippet name must match [a-zA-Z0-9_-]+", ToastType::Error);
                return Ok(());
            }
            Some(t)
        } else {
            contracts::Processor::extract_title(&text).0
        };

        self.nav.push_history();

        let branch = self.git.get_current_branch(&path).await.unwrap_or_default();
        self.service.save_item(
            &path, 
            self.nav.active_tab, 
            text, 
            title, 
            self.editor.editing_id, 
            self.editor.insert_index,
            branch
        ).await?;

        self.exit_editor();
        self.load_prompts().await?;
        self.notify("Prompt saved!", ToastType::Success);
        Ok(())
    }

    pub async fn save_and_stage_editor(&mut self) -> contracts::Result<()> {
        self.save_editor().await?;
        self.handle_message(AppMessage::StageSelected).await
    }
}
