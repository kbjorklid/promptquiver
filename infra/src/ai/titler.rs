use crate::ai::AiEngine;

const TITLE_TEMPLATE: &str = "\
You are a concise assistant. Generate a short title (3–7 words) for the following prompt.
Output only the title — no quotes, no explanation.

Prompt:
{text}

Title:";

pub async fn generate_title(prompt_text: &str, engine: &dyn AiEngine) -> Option<String> {
    let formatted = TITLE_TEMPLATE.replace("{text}", prompt_text);
    let raw = engine.generate(&formatted, 30).await.ok()?;
    let title = raw.lines().next()?.trim().to_string();
    if title.is_empty() || title.len() > 60 {
        return None;
    }
    if title.chars().all(|c| c.is_ascii_punctuation() || c.is_whitespace()) {
        return None;
    }
    Some(title)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::MockAiEngine;

    #[tokio::test]
    async fn valid_title_is_returned() {
        let engine = MockAiEngine("Fix the build".to_string());
        let result = generate_title("Some prompt text", &engine).await;
        assert_eq!(result, Some("Fix the build".to_string()));
    }

    #[tokio::test]
    async fn title_over_60_chars_is_discarded() {
        let long = "a".repeat(61);
        let engine = MockAiEngine(long);
        assert_eq!(generate_title("x", &engine).await, None);
    }

    #[tokio::test]
    async fn empty_title_is_discarded() {
        let engine = MockAiEngine(String::new());
        assert_eq!(generate_title("x", &engine).await, None);
    }

    #[tokio::test]
    async fn only_first_line_is_used() {
        let engine = MockAiEngine("Good Title\nextra stuff here".to_string());
        assert_eq!(generate_title("x", &engine).await, Some("Good Title".to_string()));
    }

    #[tokio::test]
    async fn engine_error_returns_none() {
        struct FailEngine;
        #[async_trait::async_trait]
        impl AiEngine for FailEngine {
            async fn generate(&self, _: &str, _: usize) -> anyhow::Result<String> {
                anyhow::bail!("oops")
            }
        }
        assert_eq!(generate_title("x", &FailEngine).await, None);
    }

    #[tokio::test]
    async fn punctuation_only_title_is_discarded() {
        let engine = MockAiEngine("...".to_string());
        assert_eq!(generate_title("x", &engine).await, None);
    }
}
