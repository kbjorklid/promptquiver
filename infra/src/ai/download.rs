use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub files_done: u32,
    pub files_total: u32,
}

impl DownloadProgress {
    pub fn fraction(&self) -> f32 {
        if self.files_total == 0 {
            return 1.0;
        }
        self.files_done as f32 / self.files_total as f32
    }
}

#[derive(Debug)]
pub struct ModelDownloader {
    data_dir: PathBuf,
}

impl ModelDownloader {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    pub fn hub_cache_dir(&self) -> PathBuf {
        self.data_dir.join("hub")
    }

    /// Returns the expected hf-hub snapshot base path for a model.
    /// hf-hub uses: {cache}/{models--owner--repo}/snapshots/
    fn snapshot_base(&self, model_id: &str) -> PathBuf {
        let cache_name = format!("models--{}", model_id.replace('/', "--"));
        self.hub_cache_dir().join(cache_name).join("snapshots")
    }

    pub fn is_downloaded(&self, model_id: &str) -> bool {
        let snapshots = self.snapshot_base(model_id);
        if !snapshots.exists() {
            return false;
        }
        std::fs::read_dir(&snapshots)
            .ok()
            .and_then(|entries| {
                entries.filter_map(|e| e.ok()).find(|e| e.path().join("tokenizer.json").exists())
            })
            .is_some()
    }

    #[cfg(feature = "ai")]
    pub async fn download(
        &self,
        model_id: &str,
        hf_token: Option<&str>,
        progress_tx: tokio::sync::mpsc::Sender<DownloadProgress>,
    ) -> anyhow::Result<()> {
        use hf_hub::{api::tokio::ApiBuilder, Repo, RepoType};

        let cache_dir = self.hub_cache_dir();
        let mut builder = ApiBuilder::new().with_cache_dir(cache_dir);
        if let Some(t) = hf_token {
            builder = builder.with_token(Some(t.to_string()));
        }
        let api = builder.build()?;
        let repo = api.repo(Repo::new(model_id.to_string(), RepoType::Model));

        // Resolve shard list from index file, or single safetensors file
        let shard_names = match repo.get("model.safetensors.index.json").await {
            Ok(index_path) => {
                let raw = std::fs::read(&index_path)?;
                let index: serde_json::Value = serde_json::from_slice(&raw)?;
                let map = index["weight_map"]
                    .as_object()
                    .ok_or_else(|| anyhow::anyhow!("invalid model.safetensors.index.json"))?;
                let names: std::collections::BTreeSet<String> =
                    map.values().filter_map(|v| v.as_str()).map(str::to_string).collect();
                names.into_iter().collect::<Vec<_>>()
            }
            Err(_) => vec!["model.safetensors".to_string()],
        };

        let static_files = ["tokenizer.json", "config.json", "tokenizer_config.json"];
        let all_files: Vec<String> =
            static_files.iter().map(|s| s.to_string()).chain(shard_names).collect();

        let total = u32::try_from(all_files.len()).unwrap_or(u32::MAX);
        for (i, name) in all_files.iter().enumerate() {
            repo.get(name).await?;
            let _ = progress_tx
                .send(DownloadProgress {
                    files_done: u32::try_from(i + 1).unwrap_or(u32::MAX),
                    files_total: total,
                })
                .await;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn not_downloaded_for_empty_dir() {
        let dir = tempdir().unwrap();
        let dl = ModelDownloader::new(dir.path().to_path_buf());
        assert!(!dl.is_downloaded("google/gemma-4-E2B-it"));
    }

    #[test]
    fn is_downloaded_when_snapshot_has_tokenizer() {
        let dir = tempdir().unwrap();
        let dl = ModelDownloader::new(dir.path().to_path_buf());
        let snap = dl.snapshot_base("google/gemma-4-E2B-it").join("abc123");
        std::fs::create_dir_all(&snap).unwrap();
        std::fs::write(snap.join("tokenizer.json"), "{}").unwrap();
        assert!(dl.is_downloaded("google/gemma-4-E2B-it"));
    }

    #[test]
    fn progress_fraction() {
        let p = DownloadProgress { files_done: 3, files_total: 10 };
        assert!((p.fraction() - 0.3).abs() < f32::EPSILON);
    }
}
