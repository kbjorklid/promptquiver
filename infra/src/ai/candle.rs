use anyhow::Result;
use async_trait::async_trait;
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::generation::{LogitsProcessor, Sampling};
use hf_hub::{api::sync::ApiBuilder, Repo, RepoType};
use std::path::{Path, PathBuf};
use tokenizers::Tokenizer;

use crate::ai::AiEngine;

// Greedy decoding for deterministic, high-quality titles
const TEMPERATURE: f64 = 0.0;
const DTYPE_CPU: DType = DType::F32;
const DTYPE_GPU: DType = DType::BF16;

enum GemmaModel {
    Gemma3(candle_transformers::models::gemma3::Model),
    Gemma4(candle_transformers::models::gemma4::text::TextModel),
}

impl GemmaModel {
    fn forward(&mut self, input: &Tensor, seqlen_offset: usize) -> candle_core::Result<Tensor> {
        match self {
            Self::Gemma3(m) => m.forward(input, seqlen_offset),
            Self::Gemma4(m) => m.forward(input, seqlen_offset),
        }
    }

    fn clear_kv_cache(&mut self) {
        match self {
            Self::Gemma3(m) => m.clear_kv_cache(),
            Self::Gemma4(m) => m.clear_kv_cache(),
        }
    }
}

pub struct CandleEngine {
    inner: std::sync::Mutex<CandleEngineInner>,
}

struct CandleEngineInner {
    model: GemmaModel,
    tokenizer: Tokenizer,
    device: Device,
    eos_token_id: u32,
}

impl CandleEngine {
    /// Loads the model from hf-hub cache. Must be called after download completes.
    /// Runs synchronously — call from within `tokio::task::block_in_place`.
    pub fn load(data_dir: &Path, model_id: &str, hf_token: Option<&str>) -> Result<Self> {
        let device = Self::select_device();
        let dtype = if device.is_cuda() { DTYPE_GPU } else { DTYPE_CPU };
        let cache_dir = data_dir.join("hub");

        let mut builder = ApiBuilder::new().with_cache_dir(cache_dir);
        if let Some(t) = hf_token {
            builder = builder.with_token(Some(t.to_string()));
        }
        let api = builder.build()?;
        let repo = api.repo(Repo::new(model_id.to_string(), RepoType::Model));

        let tokenizer_path = repo.get("tokenizer.json")?;
        let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(anyhow::Error::msg)?;

        let eos_token_id = tokenizer
            .token_to_id("</s>")
            .or_else(|| tokenizer.token_to_id("<eos>"))
            .unwrap_or(1);

        let model_files = load_safetensor_paths(&repo)?;
        // Safety: memory-mapping downloaded model files is safe; we trust the files
        // we downloaded via hf-hub.
        let vb = unsafe { VarBuilder::from_mmaped_safetensors(&model_files, dtype, &device)? };

        let config_path = repo.get("config.json")?;
        let model = if model_id.contains("gemma-4") {
            load_gemma4(config_path, vb)?
        } else {
            load_gemma3(config_path, vb)?
        };

        Ok(Self {
            inner: std::sync::Mutex::new(CandleEngineInner {
                model,
                tokenizer,
                device,
                eos_token_id,
            }),
        })
    }

    fn select_device() -> Device {
        #[cfg(feature = "ai-cuda")]
        {
            if let Ok(d) = Device::new_cuda(0) {
                return d;
            }
        }
        #[cfg(feature = "ai-metal")]
        {
            if let Ok(d) = Device::new_metal(0) {
                return d;
            }
        }
        Device::Cpu
    }
}

fn load_safetensor_paths(repo: &hf_hub::api::sync::ApiRepo) -> Result<Vec<PathBuf>> {
    if let Ok(index_path) = repo.get("model.safetensors.index.json") {
        let raw = std::fs::read(&index_path)?;
        let index: serde_json::Value = serde_json::from_slice(&raw)?;
        let map = index["weight_map"]
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("invalid model.safetensors.index.json"))?;
        let mut shard_names: std::collections::BTreeSet<String> =
            map.values().filter_map(|v| v.as_str()).map(str::to_string).collect();
        let paths = shard_names
            .into_iter()
            .map(|name| repo.get(&name).map_err(anyhow::Error::from))
            .collect::<Result<Vec<_>>>()?;
        return Ok(paths);
    }
    Ok(vec![repo.get("model.safetensors")?])
}

fn load_gemma4(config_path: PathBuf, vb: VarBuilder<'_>) -> Result<GemmaModel> {
    use candle_transformers::models::gemma4::config::Gemma4TextConfig;
    use candle_transformers::models::gemma4::text::TextModel;

    let raw: serde_json::Value = serde_json::from_slice(&std::fs::read(&config_path)?)?;
    let mut config: Gemma4TextConfig = if let Some(text_cfg) = raw.get("text_config") {
        serde_json::from_value(text_cfg.clone())?
    } else {
        serde_json::from_value(raw)?
    };
    config.use_flash_attn = false;
    let model = TextModel::new(&config, vb)?;
    Ok(GemmaModel::Gemma4(model))
}

fn load_gemma3(config_path: PathBuf, vb: VarBuilder<'_>) -> Result<GemmaModel> {
    use candle_transformers::models::gemma3::Config;
    use candle_transformers::models::gemma3::Model;

    let config: Config = serde_json::from_slice(&std::fs::read(&config_path)?)?;
    let model = Model::new(false, &config, vb)?;
    Ok(GemmaModel::Gemma3(model))
}

#[async_trait]
impl AiEngine for CandleEngine {
    async fn generate(&self, prompt: &str, max_tokens: usize) -> anyhow::Result<String> {
        let formatted = format_chat_prompt(prompt);

        tokio::task::block_in_place(|| {
            let mut inner = self.inner.lock().expect("candle mutex poisoned");
            inner.model.clear_kv_cache();

            let tokens = inner
                .tokenizer
                .encode(formatted.as_str(), true)
                .map_err(anyhow::Error::msg)?
                .get_ids()
                .to_vec();

            let mut logits_processor =
                LogitsProcessor::from_sampling(42, Sampling::ArgMax);

            let mut all_tokens = tokens.clone();
            let mut output_ids: Vec<u32> = Vec::new();
            let prompt_len = tokens.len();

            for index in 0..max_tokens {
                let context_size = if index > 0 { 1 } else { prompt_len };
                let start_pos = all_tokens.len().saturating_sub(context_size);
                let ctxt = &all_tokens[start_pos..];

                let input = Tensor::new(ctxt, &inner.device)?.unsqueeze(0)?;
                let logits = inner.model.forward(&input, start_pos)?;
                let logits = logits.squeeze(0)?.squeeze(0)?.to_dtype(DType::F32)?;

                let next_token = logits_processor.sample(&logits)?;
                all_tokens.push(next_token);

                if next_token == inner.eos_token_id {
                    break;
                }
                output_ids.push(next_token);
            }

            let text = inner
                .tokenizer
                .decode(&output_ids, true)
                .map_err(anyhow::Error::msg)?;
            Ok(text)
        })
    }
}

fn format_chat_prompt(user_message: &str) -> String {
    format!(
        "<start_of_turn>user\n{user_message}<end_of_turn>\n<start_of_turn>model\n"
    )
}
