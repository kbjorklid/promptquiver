use contracts::{Prompt, PromptFilter, PromptType, Tab};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use ratatui_textarea::TextArea;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct AutocompleteState {
    pub open: bool,
    pub suggestions: Vec<Prompt>,
    pub index: usize,
    pub list_state: ratatui::widgets::ListState,
}

impl AutocompleteState {
    pub fn clear(&mut self) {
        self.open = false;
        self.suggestions.clear();
        self.index = 0;
        self.list_state = ratatui::widgets::ListState::default();
    }

    pub const fn move_down(&mut self) {
        if !self.suggestions.is_empty() && self.index < self.suggestions.len() - 1 {
            self.index += 1;
        }
    }

    pub const fn move_up(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        }
    }

    /// Updates the autocomplete suggestions based on the query.
    ///
    /// # Errors
    /// Returns an error if storage or service operations fail.
    #[allow(clippy::too_many_arguments)]
    pub async fn update(
        &mut self,
        query_opt: Option<(String, String)>,
        storage: Arc<dyn contracts::Storage>,
        settings: &contracts::Settings,
        current_path: String,
        file_search_tx: &Option<tokio::sync::mpsc::Sender<(String, String)>>,
        snippet_cache: &mut Vec<Prompt>,
        claude_commands: &[Prompt],
    ) -> contracts::Result<()> {
        if let Some((trigger, query)) = query_opt {
            let mut matcher = Matcher::new(Config::DEFAULT);
            let pattern = Pattern::parse(&query, CaseMatching::Ignore, Normalization::Smart);
            let mut buf = Vec::new();

            match trigger.as_str() {
                "$" | "$$" => {
                    let snippets = if snippet_cache.is_empty() {
                        let s = storage
                            .get_prompts(PromptFilter {
                                tab: Some(Tab::Snippets),
                                ..Default::default()
                            })
                            .await?;
                        snippet_cache.clone_from(&s);
                        s
                    } else {
                        snippet_cache.clone()
                    };

                    let mut scored_suggestions: Vec<(u32, Prompt)> = Vec::new();
                    for s in snippets {
                        let text = s.name.as_deref().unwrap_or(&s.text);
                        if let Some(score) =
                            pattern.score(Utf32Str::new(text, &mut buf), &mut matcher)
                        {
                            scored_suggestions.push((score, s));
                        }
                    }

                    scored_suggestions.sort_by_key(|b| std::cmp::Reverse(b.0));
                    self.suggestions = scored_suggestions.into_iter().map(|(_, s)| s).collect();
                }
                "@" => {
                    self.suggestions.clear();
                    self.open = false;
                    if let Some(tx) = file_search_tx {
                        let _ = tx.try_send((current_path, query.clone()));
                    }
                    return Ok(());
                }
                "/" => {
                    let mut combined_commands: Vec<Prompt> = settings
                        .slash_commands
                        .iter()
                        .map(|cmd| {
                            Prompt::new(
                                cmd.clone(),
                                PromptType::Prompt,
                                None,
                                None,
                                Some(cmd.clone()),
                                None,
                            )
                        })
                        .collect();

                    if settings.enable_claude_commands {
                        combined_commands.extend(claude_commands.iter().cloned());
                    }

                    let mut scored_suggestions: Vec<(u32, Prompt)> = Vec::new();
                    for cmd in combined_commands {
                        let name = cmd.name.as_deref().unwrap_or(&cmd.text);
                        if let Some(score) =
                            pattern.score(Utf32Str::new(name, &mut buf), &mut matcher)
                        {
                            scored_suggestions.push((score, cmd));
                        }
                    }

                    scored_suggestions.sort_by_key(|b| std::cmp::Reverse(b.0));
                    self.suggestions = scored_suggestions.into_iter().map(|(_, s)| s).collect();
                }
                _ => self.suggestions = Vec::new(),
            }

            if self.suggestions.is_empty() {
                self.open = false;
            } else {
                self.open = true;
                if self.index >= self.suggestions.len() {
                    self.index = 0;
                }
            }
        } else {
            self.open = false;
            self.suggestions.clear();
        }

        Ok(())
    }

    fn get_current_query(textarea: &TextArea<'_>) -> Option<(String, String)> {
        let cursor = textarea.cursor();
        let row = cursor.0;
        let col = cursor.1;

        if row >= textarea.lines().len() {
            return None;
        }

        let line = &textarea.lines()[row];
        let byte_col = line.char_indices().nth(col).map_or(line.len(), |(i, _)| i);
        let before_cursor = &line[..byte_col];

        let triggers = ["$$", "$", "@", "/"];
        let mut best_trigger = None;
        let mut best_pos = 0;

        for trigger in triggers {
            if let Some(pos) = before_cursor.rfind(trigger) {
                // Check if it's a valid trigger position
                let is_valid = match trigger {
                    "/" => {
                        // Slash is only a trigger if it's at the start or preceded by space
                        pos == 0 || (pos > 0 && before_cursor.as_bytes()[pos - 1] == b' ')
                    }
                    _ => true,
                };
                if !is_valid {
                    continue;
                }

                if trigger == "$" && pos > 0 && before_cursor.as_bytes()[pos - 1] == b'$' {
                    continue;
                }

                if best_trigger.is_none() || pos > best_pos {
                    best_trigger = Some(trigger);
                    best_pos = pos;
                }
            }
        }

        if let Some(trigger) = best_trigger {
            let query = &before_cursor[best_pos + trigger.len()..];
            if query.contains(' ') {
                return None;
            }
            return Some((trigger.to_string(), query.to_string()));
        }
        None
    }

    pub fn select_suggestion(&mut self, textarea: &mut TextArea<'_>, add_space: bool) {
        if !self.suggestions.is_empty() && self.open {
            let snippet = &self.suggestions[self.index];
            let name = snippet.name.as_deref().unwrap_or(&snippet.text);

            let cursor = textarea.cursor();
            let row = cursor.0;
            let col = cursor.1;
            let line = textarea.lines()[row].clone();
            let byte_col = line.char_indices().nth(col).map_or(line.len(), |(i, _)| i);
            let before_cursor = &line[..byte_col];

            let triggers = ["$$", "$", "@", "/"];
            let mut best_trigger = None;
            let mut best_pos = 0;

            for trigger in triggers {
                if let Some(pos) = before_cursor.rfind(trigger) {
                    let is_valid = match trigger {
                        "/" => pos == 0 || (pos > 0 && before_cursor.as_bytes()[pos - 1] == b' '),
                        _ => true,
                    };

                    if !is_valid {
                        continue;
                    }

                    if trigger == "$" && pos > 0 && before_cursor.as_bytes()[pos - 1] == b'$' {
                        continue;
                    }

                    if best_trigger.is_none() || pos > best_pos {
                        best_trigger = Some(trigger);
                        best_pos = pos;
                    }
                }
            }
            if let Some(trigger) = best_trigger {
                let mut replacement = match trigger {
                    "$$" => format!("$${name}"),
                    "$" => snippet.text.clone(),
                    "@" => format!("@{name}"),
                    "/" => format!("/{name}"),
                    _ => name.to_string(),
                };

                if add_space {
                    replacement.push(' ');
                }

                let mut new_line = line[..best_pos].to_string();
                new_line.push_str(&replacement);
                new_line.push_str(&line[byte_col..]);

                let new_col = line[..best_pos].chars().count() + replacement.chars().count();

                textarea.move_cursor(ratatui_textarea::CursorMove::Jump(
                    u16::try_from(row).unwrap_or(u16::MAX),
                    0,
                ));
                textarea.delete_line_by_end();
                textarea.insert_str(&new_line);
                textarea.move_cursor(ratatui_textarea::CursorMove::Jump(
                    u16::try_from(row).unwrap_or(u16::MAX),
                    u16::try_from(new_col).unwrap_or(u16::MAX),
                ));
            }

            self.clear();
        }
    }
}

#[derive(Debug, Default)]
pub struct EditorModule<'a> {
    pub textarea: TextArea<'a>,
    pub title_textarea: TextArea<'a>,
    pub title_focused: bool,
    pub editing_id: Option<Uuid>,
    pub insert_index: Option<usize>,
    pub original_text: String,
    pub autocomplete: AutocompleteState,
    pub snippet_cache: Vec<Prompt>,
}

impl<'a> EditorModule<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Handles application messages and updates editor state.
    ///
    /// # Errors
    /// Returns an error if storage or service operations fail.
    pub async fn update(
        &mut self,
        msg: crate::types::AppMessage,
        ctx: &mut crate::types::UpdateContext<'_>,
        current_path: String,
        file_search_tx: &Option<tokio::sync::mpsc::Sender<(String, String)>>,
    ) -> contracts::Result<Option<crate::types::AppMessage>> {
        use crate::types::AppMessage;
        match msg {
            AppMessage::SaveEditor => {
                if let Some(msg) = self.handle_save(ctx) {
                    return Ok(Some(msg));
                }
            }
            AppMessage::UpdateAutocomplete => {
                let query_opt = AutocompleteState::get_current_query(&self.textarea);
                self.autocomplete
                    .update(
                        query_opt,
                        ctx.storage.clone(),
                        ctx.settings,
                        current_path,
                        file_search_tx,
                        &mut self.snippet_cache,
                        ctx.claude_commands,
                    )
                    .await?;
            }
            AppMessage::CloseAutocomplete => {
                self.autocomplete.clear();
            }
            AppMessage::MoveSuggestionDown => self.autocomplete.move_down(),
            AppMessage::MoveSuggestionUp => self.autocomplete.move_up(),
            AppMessage::SelectSuggestion(add_space) => {
                self.autocomplete.select_suggestion(&mut self.textarea, add_space);
            }
            AppMessage::Paste(content) => {
                self.handle_paste(content, ctx.active_tab);
                return Ok(Some(AppMessage::UpdateAutocomplete));
            }
            AppMessage::EditorInput(key) => {
                if let Some(msg) = self.handle_input(key, ctx.active_tab) {
                    return Ok(Some(msg));
                }
                return Ok(Some(AppMessage::UpdateAutocomplete));
            }
            _ => {}
        }
        Ok(None)
    }

    pub fn get_current_autocomplete_query(&self) -> Option<(String, String)> {
        AutocompleteState::get_current_query(&self.textarea)
    }

    fn handle_save(
        &self,
        ctx: &crate::types::UpdateContext<'_>,
    ) -> Option<crate::types::AppMessage> {
        use crate::types::AppMessage;
        use contracts::Tab;
        let text = self.textarea.lines().join("\n");

        if ctx.active_tab == Tab::Settings {
            let re = regex::Regex::new("^[a-zA-Z0-9_:-]+$").expect("Static regex");
            let mut trimmed = text.trim();
            if trimmed.starts_with('/') {
                trimmed = &trimmed[1..];
            }
            if !trimmed.is_empty() && !re.is_match(trimmed) {
                return Some(AppMessage::Notify(
                    "Slash command must match [a-zA-Z0-9_:-]+".into(),
                    ratatui_toaster::ToastType::Error,
                ));
            }
        }

        if ctx.active_tab == Tab::Snippets {
            let t = self.title_textarea.lines().join("");
            let re = regex::Regex::new("^[a-zA-Z0-9_-]+$").expect("Static regex");
            if !re.is_match(&t) {
                return Some(AppMessage::Notify(
                    "Snippet name must match [a-zA-Z0-9_-]+".into(),
                    ratatui_toaster::ToastType::Error,
                ));
            }
        }
        None
    }

    fn handle_paste(&mut self, content: String, active_tab: Tab) {
        if self.title_focused && active_tab == Tab::Snippets {
            let single_line = content.replace(['\n', '\r'], "");
            self.title_textarea.insert_str(single_line);
        } else {
            self.textarea.insert_str(content);
        }
    }

    fn handle_input(
        &mut self,
        key: crossterm::event::KeyEvent,
        active_tab: Tab,
    ) -> Option<crate::types::AppMessage> {
        use crate::types::AppMessage;
        use contracts::Tab;
        use crossterm::event::KeyCode;

        if active_tab == Tab::Snippets {
            if key.code == KeyCode::Tab {
                self.title_focused = !self.title_focused;
                return None;
            }
            if self.title_focused && key.code == KeyCode::Enter {
                self.title_focused = false;
                return None;
            }
        }

        if self.title_focused && active_tab == Tab::Snippets {
            Self::input_with_fallback(&mut self.title_textarea, key);
            if self.title_textarea.lines().len() > 1 {
                let joined = self.title_textarea.lines().join("");
                self.title_textarea = ratatui_textarea::TextArea::new(vec![joined]);
                self.title_textarea.move_cursor(ratatui_textarea::CursorMove::End);
            }
        } else {
            if active_tab == Tab::Settings && key.code == KeyCode::Enter {
                return Some(AppMessage::SaveEditor);
            }
            Self::input_with_fallback(&mut self.textarea, key);
        }
        None
    }

    fn input_with_fallback(textarea: &mut TextArea<'a>, key: crossterm::event::KeyEvent) {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        match (key.code, key.modifiers) {
            (KeyCode::Backspace, m) if m.contains(KeyModifiers::CONTROL) => {
                textarea.delete_word();
                return;
            }
            (KeyCode::Char('\u{7f}'), _) => {
                textarea.delete_word();
                return;
            }
            _ => {}
        }

        if !textarea.input(key) {
            if let KeyCode::Char(c) = key.code {
                let is_control = key.modifiers.contains(KeyModifiers::CONTROL);
                let is_alt = key.modifiers.contains(KeyModifiers::ALT);
                let is_altgr = is_control && is_alt;

                if is_altgr || (!is_control && !is_alt) {
                    textarea.input(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()));
                }
            }
        }
    }

    pub fn is_dirty(&self) -> bool {
        let current_text = self.textarea.lines().join("\n");
        current_text != self.original_text
    }

    pub fn clear(&mut self) {
        self.textarea = TextArea::default();
        self.title_textarea = TextArea::default();
        self.title_focused = false;
        self.editing_id = None;
        self.insert_index = None;
        self.original_text = String::new();
        self.autocomplete.clear();
        self.snippet_cache.clear();
    }

    pub fn enter(
        &mut self,
        text: String,
        id: Option<Uuid>,
        title: Option<String>,
        tab: Tab,
        insert_index: Option<usize>,
    ) {
        self.clear();
        self.textarea = TextArea::from(text.lines().map(String::from));
        self.textarea.set_wrap_mode(ratatui_textarea::WrapMode::WordOrGlyph);
        self.original_text = text;
        self.editing_id = id;
        self.insert_index = insert_index;

        if let Some(t) = title {
            self.title_textarea = TextArea::from(vec![t]);
        }

        if tab == Tab::Snippets {
            self.title_focused = true;
        }
    }

    pub fn exit(&mut self) {
        self.clear();
    }
}
