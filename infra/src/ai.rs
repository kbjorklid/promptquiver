/// Extracts a clean title from the first line of model output.
/// Returns None if empty, >60 chars, or whitespace only.
pub fn extract_title(raw: &str) -> Option<String> {
    let title = raw.lines().next()?.trim().trim_matches('"').trim_matches('\'').to_string();
    if title.is_empty() || title.len() > 60 {
        return None;
    }
    Some(title)
}

/// Prompt template for title generation.
pub fn title_prompt(prompt_text: &str) -> String {
    format!(
        "You are a concise assistant. Generate a short title (3-7 words) for the following prompt.\n\
         Output only the title — no quotes, no explanation.\n\n\
         Prompt:\n{prompt_text}\n\nTitle:"
    )
}

/// Builds and runs the mistral.rs model.
///
/// Compiled only when the `ai` feature is enabled.
#[cfg(feature = "ai")]
pub mod engine {
    use super::{extract_title, title_prompt};
    use anyhow::{Context, Result};
    use mistralrs::{IsqType, RequestBuilder, TextMessageRole, TextMessages, TextModelBuilder};
    use uuid::Uuid;

    pub struct AiTitleEngine {
        model: mistralrs::Model,
    }

    impl AiTitleEngine {
        /// Loads the model from a HuggingFace Hub model ID or local directory path.
        pub async fn load(model_id: &str) -> Result<Self> {
            let model = TextModelBuilder::new(model_id)
                .with_isq(IsqType::Q4K)
                .build()
                .await
                .with_context(|| format!("Failed to load model: {model_id}"))?;
            Ok(Self { model })
        }

        /// Generates a title for the given prompt text.
        /// Returns `None` if the output is empty, malformed, or longer than 60 chars.
        pub async fn generate_title(&self, id: Uuid, text: &str) -> Result<Option<(Uuid, String)>> {
            let user_message = title_prompt(text);
            let messages = TextMessages::new().add_message(TextMessageRole::User, &user_message);

            let request = RequestBuilder::from(messages).set_sampler_max_len(30);

            let response = self.model.send_chat_request(request).await?;
            let raw =
                response.choices.first().and_then(|c| c.message.content.as_deref()).unwrap_or("");

            Ok(extract_title(raw).map(|title| (id, title)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_plain_title() {
        assert_eq!(extract_title("Short title here"), Some("Short title here".to_string()));
    }

    #[test]
    fn extract_strips_double_quotes() {
        assert_eq!(extract_title("\"Quoted title\""), Some("Quoted title".to_string()));
    }

    #[test]
    fn extract_strips_single_quotes() {
        assert_eq!(extract_title("'Single quoted'"), Some("Single quoted".to_string()));
    }

    #[test]
    fn extract_empty_returns_none() {
        assert_eq!(extract_title(""), None);
    }

    #[test]
    fn extract_too_long_returns_none() {
        assert_eq!(extract_title(&"x".repeat(61)), None);
    }

    #[test]
    fn extract_exactly_60_chars_is_ok() {
        let s = "x".repeat(60);
        assert_eq!(extract_title(&s), Some(s));
    }

    #[test]
    fn extract_uses_first_line_only() {
        assert_eq!(extract_title("first line\nsecond line"), Some("first line".to_string()));
    }

    #[test]
    fn title_prompt_contains_text() {
        let p = title_prompt("my prompt body");
        assert!(p.contains("my prompt body"));
        assert!(p.contains("Title:"));
    }

    #[test]
    fn title_prompt_instructs_no_explanation() {
        let p = title_prompt("x");
        assert!(p.contains("no quotes, no explanation"));
    }

    #[test]
    fn extract_whitespace_only_returns_none() {
        assert_eq!(extract_title("   "), None);
    }

    #[test]
    fn extract_trims_surrounding_whitespace() {
        assert_eq!(extract_title("  Good title  "), Some("Good title".to_string()));
    }

    #[test]
    fn extract_empty_after_quote_strip_returns_none() {
        assert_eq!(extract_title("\"\""), None);
    }

    #[test]
    fn extract_newline_only_returns_none() {
        assert_eq!(extract_title("\n"), None);
    }
}
