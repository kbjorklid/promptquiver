# mistral.rs — Implementation Notes

Notes gathered during research (May 2026) for a potential future implementation of the AI auto-titling feature using mistral.rs instead of candle.

## Library Overview

- **Crate:** `mistralrs` on crates.io
- **Version at time of research:** 0.8.1 (stable release, no git dep needed)
- **Repo:** https://github.com/EricLBuehler/mistral.rs
- **Gemma 4 support:** Yes, full support confirmed

## Dependency Declaration

```toml
[dependencies]
mistralrs = { version = "0.8.1", optional = true }

[features]
ai = ["mistralrs"]
ai-cuda = ["ai", "mistralrs/cuda"]
ai-metal = ["ai", "mistralrs/metal"]
```

No separate `hf-hub`, `tokenizers`, or `candle-*` crates needed — mistral.rs bundles everything including HuggingFace Hub download.

## API Pattern

mistral.rs uses a builder pattern throughout. The rough shape for a text-only use case:

```rust
use mistralrs::{TextModelBuilder, RequestBuilder, TextMessageRole, TextMessages};

// Load model (downloads from HF Hub automatically if not cached)
let model = TextModelBuilder::new("google/gemma-4-E2B-it")
    .with_auto_isq(IsqBits::Four)  // 4-bit quantization, reduces memory
    .build()
    .await?;

// Build a chat request
let messages = TextMessages::new()
    .add_message(TextMessageRole::User, "your prompt here");

let request = RequestBuilder::new(messages);
let response = model.send_chat_request(request).await?;

// Extract text
let text = response.choices[0].message.content.as_ref().unwrap();
```

## Key Differences from Low-Level Inference

- **Chat template is automatic.** mistral.rs reads `tokenizer_config.json` from the model and applies the correct template. No manual `<start_of_turn>user\n...<end_of_turn>` formatting needed.
- **Downloading is built in.** `TextModelBuilder::new("google/gemma-4-E2B-it")` handles HF Hub download and caching. No separate download module required.
- **Hardware is auto-detected.** CPU fallback happens automatically. CUDA and Metal enabled via feature flags only.
- **Quantization via ISQ.** `with_auto_isq(IsqBits::Four)` applies in-situ 4-bit quantization after loading, which reduces memory significantly without needing a pre-quantized GGUF.

## Inference Flow

```mermaid
flowchart LR
    A[TextModelBuilder::new] --> B[.with_auto_isq]
    B --> C[.build().await]
    C --> D[TextMessages::new]
    D --> E[.add_message]
    E --> F[model.send_chat_request]
    F --> G[response.choices[0].message.content]
```

## Model IDs

- `google/gemma-4-E2B-it` — 2B parameter instruction-tuned (smaller, faster)
- `google/gemma-4-E4B-it` — 4B parameter instruction-tuned (higher quality)

The 2B variant is probably the right default for an auto-titling use case given the small input (prompt text → short title).

## Download and Cache Location

mistral.rs uses the HuggingFace Hub cache format, same as hf-hub. The default cache lives in the OS-standard HF cache dir (`~/.cache/huggingface/hub` on Linux/Mac, `%USERPROFILE%\.cache\huggingface\hub` on Windows). This means models downloaded by mistral.rs and by hf-hub are compatible and stored in the same location.

If the app needs a custom cache location (e.g., next to the app's own data dir), check whether `TextModelBuilder` exposes a cache dir option — it may not, unlike hf-hub's `ApiBuilder::with_cache_dir`.

## Async Considerations

All mistral.rs inference is async. The `.build()` and `.send_chat_request()` calls are `async fn`, so they can be called directly from a Tokio task without `block_in_place`. This is cleaner than candle's synchronous model loading which requires `tokio::task::block_in_place`.

## What mistral.rs Does Not Handle

- **Progress reporting during model download.** There's no built-in progress callback for the download phase. If you need to show a progress bar in the TUI while the model downloads, you'd need a workaround (e.g., poll the cache directory size, or pre-download with a separate hf-hub call and then pass the local path to mistral.rs).
- **Custom stopping criteria beyond EOS.** For simple titling this is fine; the model stops on its own.
