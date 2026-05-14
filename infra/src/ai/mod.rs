#[cfg(feature = "ai")]
pub mod candle;
pub mod download;
pub mod titler;

use async_trait::async_trait;
use contracts::ModelTier;

#[async_trait]
pub trait AiEngine: Send + Sync {
    async fn generate(&self, prompt: &str, max_tokens: usize) -> anyhow::Result<String>;
}

pub fn model_id(tier: ModelTier) -> &'static str {
    match tier {
        ModelTier::Fast => "google/gemma-4-E2B-it",
        ModelTier::Balanced => "google/gemma-4-E4B-it",
        ModelTier::Quality => "google/gemma-3-12b-it",
    }
}

#[derive(Debug)]
pub struct NoopEngine;

#[async_trait]
impl AiEngine for NoopEngine {
    async fn generate(&self, _prompt: &str, _max_tokens: usize) -> anyhow::Result<String> {
        anyhow::bail!("AI not available")
    }
}

/// Canned-response engine for tests and integration tests.
#[derive(Debug)]
pub struct MockAiEngine(pub String);

#[async_trait]
impl AiEngine for MockAiEngine {
    async fn generate(&self, _prompt: &str, _max_tokens: usize) -> anyhow::Result<String> {
        Ok(self.0.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_engine_returns_canned_response() {
        let engine = MockAiEngine("Hello World".to_string());
        let out = engine.generate("anything", 20).await.unwrap();
        assert_eq!(out, "Hello World");
    }

    #[tokio::test]
    async fn noop_engine_returns_error() {
        let engine = NoopEngine;
        assert!(engine.generate("anything", 20).await.is_err());
    }

    #[test]
    fn model_id_maps_all_tiers() {
        assert_eq!(model_id(ModelTier::Fast), "google/gemma-4-E2B-it");
        assert_eq!(model_id(ModelTier::Balanced), "google/gemma-4-E4B-it");
        assert_eq!(model_id(ModelTier::Quality), "google/gemma-3-12b-it");
    }
}
