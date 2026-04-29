use ratatui_textarea::TextArea;
use contracts::{Prompt, Tab, PromptType};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct AutocompleteState {
    pub open: bool,
    pub suggestions: Vec<Prompt>,
    pub index: usize,
    pub list_state: ratatui::widgets::ListState,
}

#[derive(Debug)]
pub struct EditorModule<'a> {
    pub textarea: TextArea<'a>,
    pub title_textarea: TextArea<'a>,
    pub title_focused: bool,
    pub editing_id: Option<Uuid>,
    pub insert_index: Option<usize>,
    pub original_text: String,
    pub autocomplete: AutocompleteState,
}

impl Default for EditorModule<'_> {
    fn default() -> Self {
        Self {
            textarea: TextArea::default(),
            title_textarea: TextArea::default(),
            title_focused: false,
            editing_id: None,
            insert_index: None,
            original_text: String::new(),
            autocomplete: AutocompleteState::default(),
        }
    }
}

impl<'a> EditorModule<'a> {
    pub fn new() -> Self {
        Self::default()
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
        self.autocomplete = AutocompleteState::default();
    }

    pub fn enter(&mut self, text: String, id: Option<Uuid>, title: Option<String>, tab: Tab, insert_index: Option<usize>) {
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

    pub fn get_current_autocomplete_query(&self) -> Option<(String, String)> {
        let cursor = self.textarea.cursor();
        let row = cursor.0;
        let col = cursor.1;
        
        if row >= self.textarea.lines().len() {
            return None;
        }
        
        let line = &self.textarea.lines()[row];
        let byte_col = line.char_indices().nth(col).map(|(i, _)| i).unwrap_or(line.len());
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

    pub async fn update_autocomplete(
        &mut self, 
        global_snippets: Vec<Prompt>, 
        settings: &contracts::Settings,
        current_path: String,
        file_search_tx: &Option<tokio::sync::mpsc::Sender<(String, String)>>
    ) -> contracts::Result<()> {
        if let Some((trigger, query)) = self.get_current_autocomplete_query() {
            let matcher = SkimMatcherV2::default();
            
            match trigger.as_str() {
                "$" | "$$" => {
                    let query_lower = query.to_lowercase();
                    let mut scored_suggestions: Vec<(i64, Prompt)> = global_snippets
                        .into_iter()
                        .filter_map(|s| {
                            let text = s.name.as_deref().unwrap_or(&s.text);
                            matcher.fuzzy_match(&text.to_lowercase(), &query_lower).map(|score| (score, s))
                        })
                        .collect();
                    
                    scored_suggestions.sort_by_key(|b| std::cmp::Reverse(b.0));
                    self.autocomplete.suggestions = scored_suggestions.into_iter().map(|(_, s)| s).collect();
                }
                "@" => {
                    self.autocomplete.suggestions.clear();
                    self.autocomplete.open = false;
                    if let Some(tx) = file_search_tx {
                        let _ = tx.try_send((current_path, query.to_string()));
                    }
                    return Ok(());
                }
                "/" => {
                    let query_lower = query.to_lowercase();
                    let mut scored_suggestions: Vec<(i64, Prompt)> = settings.slash_commands
                        .iter()
                        .filter_map(|cmd| {
                            matcher.fuzzy_match(&cmd.to_lowercase(), &query_lower).map(|score| (score, Prompt::new(cmd.clone(), PromptType::Prompt, None, Some(cmd.clone()))))
                        })
                        .collect();
                        
                    scored_suggestions.sort_by_key(|b| std::cmp::Reverse(b.0));
                    self.autocomplete.suggestions = scored_suggestions.into_iter().map(|(_, s)| s).collect();
                }
                _ => self.autocomplete.suggestions = Vec::new(),
            }
            
            if self.autocomplete.suggestions.is_empty() {
                self.autocomplete.open = false;
            } else {
                self.autocomplete.open = true;
                if self.autocomplete.index >= self.autocomplete.suggestions.len() {
                    self.autocomplete.index = 0;
                }
            }
        } else {
            self.autocomplete.open = false;
            self.autocomplete.suggestions.clear();
        }

        Ok(())
    }

    pub fn move_suggestion_down(&mut self) {
        if !self.autocomplete.suggestions.is_empty() && self.autocomplete.index < self.autocomplete.suggestions.len() - 1 {
            self.autocomplete.index += 1;
        }
    }

    pub fn move_suggestion_up(&mut self) {
        if self.autocomplete.index > 0 {
            self.autocomplete.index -= 1;
        }
    }

    pub fn select_suggestion(&mut self) {
        if !self.autocomplete.suggestions.is_empty() && self.autocomplete.open {
            let snippet = &self.autocomplete.suggestions[self.autocomplete.index];
            let name = snippet.name.as_deref().unwrap_or(&snippet.text);
            
            let cursor = self.textarea.cursor();
            let row = cursor.0;
            let col = cursor.1;
            let line = self.textarea.lines()[row].clone();
            let byte_col = line.char_indices().nth(col).map(|(i, _)| i).unwrap_or(line.len());
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
                let replacement = match trigger {
                    "$$" => format!("$${name}"),
                    "$" => snippet.text.clone(),
                    "@" => format!("@{name}"),
                    "/" => format!("/{name}"),
                    _ => name.to_string(),
                };

                let mut new_line = line[..best_pos].to_string();
                new_line.push_str(&replacement);
                new_line.push_str(&line[byte_col..]);
                
                let new_col = line[..best_pos].chars().count() + replacement.chars().count();
                
                self.textarea.move_cursor(ratatui_textarea::CursorMove::Jump(row as u16, 0));
                self.textarea.delete_line_by_end();
                self.textarea.insert_str(&new_line);
                self.textarea.move_cursor(ratatui_textarea::CursorMove::Jump(row as u16, new_col as u16));
            }
            
            self.autocomplete.open = false;
            self.autocomplete.suggestions.clear();
            self.autocomplete.index = 0;
        }
    }
}
