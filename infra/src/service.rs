use async_trait::async_trait;
use chrono;
use contracts::Tab::{Archive, Notes, Prompts, Settings, Snippets};
use contracts::{AppService, Clipboard, Processor, Prompt, PromptFilter, Result, Storage, Tab};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use std::sync::Arc;
use uuid;

pub struct RealAppService {
    storage: Arc<dyn Storage>,
    clipboard: Arc<dyn Clipboard>,
}

impl std::fmt::Debug for RealAppService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RealAppService").finish_non_exhaustive()
    }
}

impl RealAppService {
    pub fn new(storage: Arc<dyn Storage>, clipboard: Arc<dyn Clipboard>) -> Self {
        Self { storage, clipboard }
    }

    /// Clears the `last_copied` field for all prompts.
    ///
    /// # Errors
    /// Returns an error if the storage cannot be accessed.
    async fn clear_all_last_copied(&self) -> Result<()> {
        let all = self.storage.get_prompts(PromptFilter::default()).await?;
        for mut p in all {
            if p.last_copied {
                p.last_copied = false;
                self.storage.save_prompt(p).await?;
            }
        }
        Ok(())
    }
}

fn walk_recursive(
    base_dir: &std::path::Path,
    current_dir: &std::path::Path,
    pattern: &Pattern,
    matcher: &mut Matcher,
    buf: &mut Vec<char>,
    results: &mut Vec<(u32, Prompt)>,
) {
    if results.len() >= 1000 {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(current_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let relative_path =
                path.strip_prefix(base_dir).unwrap_or(&path).to_string_lossy().to_string();
            let path_normalized = relative_path.replace('\\', "/");

            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name == "target"
                        || name == ".git"
                        || name == "node_modules"
                        || name.starts_with('.')
                    {
                        continue;
                    }
                }

                let dir_path_normalized =
                    if path_normalized.ends_with('/') || path_normalized.is_empty() {
                        path_normalized.clone()
                    } else {
                        format!("{path_normalized}/")
                    };

                if !dir_path_normalized.is_empty() {
                    if let Some(score) =
                        pattern.score(Utf32Str::new(&dir_path_normalized, buf), matcher)
                    {
                        results.push((
                            score + 50,
                            Prompt::new(
                                path.to_string_lossy().to_string(),
                                contracts::PromptType::Note,
                                None,
                                None,
                                Some(dir_path_normalized),
                                None,
                            ),
                        ));
                    }
                }

                walk_recursive(base_dir, &path, pattern, matcher, buf, results);
            } else if let Some(score) = pattern.score(Utf32Str::new(&path_normalized, buf), matcher)
            {
                results.push((
                    score,
                    Prompt::new(
                        path.to_string_lossy().to_string(),
                        contracts::PromptType::Note,
                        None,
                        None,
                        Some(path_normalized),
                        None,
                    ),
                ));
            }
        }
    }
}

#[async_trait]
impl AppService for RealAppService {
    async fn stage_item(&self, _folder: &str, tab: Tab, mut item: Prompt) -> Result<()> {
        if tab == Settings {
            return Ok(());
        }

        let snippets = self
            .storage
            .get_prompts(PromptFilter { tab: Some(Snippets), ..Default::default() })
            .await?;

        // Alias for Notes and Snippets: they cannot be staged anymore
        if tab == Notes || tab == Snippets {
            let processed_text = Processor::process(&item.text, &snippets);
            self.clipboard.copy(processed_text).await?;
            return Ok(());
        }

        // Check if it's a draft (only for Prompts/Canned)
        if (tab == Prompts || tab == Tab::Canned)
            && Processor::is_draft(&Processor::get_display_title(&item.text).0)
        {
            return Err(contracts::Error::Storage(
                "Cannot stage a draft prompt. Remove 'Draft' from the title first.".to_string(),
            ));
        }

        if item.staged {
            // Un-stage
            item.staged = false;
            self.storage.save_prompt(item).await?;
        } else {
            // Stage
            // Unstage and archive other staged items in the same scope?
            // Old behavior: "Remove archived items from their original lists. Add to archive (to the top)"
            let all_in_scope = self
                .storage
                .get_prompts(PromptFilter { folder: item.folder.clone(), ..Default::default() })
                .await?;
            for mut p in all_in_scope {
                if p.r#type == contracts::PromptType::Prompt && p.staged && p.id != item.id {
                    p.staged = false;
                    p.is_archived = true;
                    self.storage.save_prompt(p).await?;
                }
            }

            // Set target to staged
            item.staged = true;
            self.storage.save_prompt(item.clone()).await?;

            // Clear last_copied for all when staging
            self.clear_all_last_copied().await?;

            // Process text before copying
            let processed_text = Processor::process(&item.text, &snippets);
            self.clipboard.copy(processed_text).await?;
        }

        Ok(())
    }

    async fn archive_item(&self, _folder: &str, tab: Tab, mut item: Prompt) -> Result<()> {
        if tab == Settings {
            return Ok(());
        }

        if tab == Archive {
            // Permanent delete
            self.storage.delete_prompt(item.id).await?;
        } else {
            // Move to archive
            item.is_archived = true;
            item.staged = false;
            self.storage.save_prompt(item).await?;
        }
        Ok(())
    }

    async fn restore_item(&self, _folder: &str, mut item: Prompt) -> Result<()> {
        item.is_archived = false;
        item.staged = false;
        self.storage.save_prompt(item).await?;
        Ok(())
    }

    async fn duplicate_item(
        &self,
        _folder: &str,
        tab: Tab,
        item: Prompt,
    ) -> Result<Option<Prompt>> {
        if tab == Settings {
            return Ok(None);
        }

        let mut p = item.clone();
        p.id = uuid::Uuid::new_v4();
        p.staged = false;
        p.is_archived = false;
        p.created_at = chrono::Utc::now();
        p.updated_at = p.created_at;

        self.storage.save_prompt(p.clone()).await?;
        Ok(Some(p))
    }

    async fn copy_item(&self, _folder: &str, tab: Tab, item: Prompt) -> Result<()> {
        if tab == Settings {
            return Ok(());
        }

        // 1. Clear all
        self.clear_all_last_copied().await?;

        // 2. Mark current as last_copied
        let mut p = item.clone();
        p.last_copied = true;
        self.storage.save_prompt(p).await?;

        // 3. Process and copy
        let snippets = self
            .storage
            .get_prompts(PromptFilter { tab: Some(Snippets), ..Default::default() })
            .await?;
        let processed_text = Processor::process(&item.text, &snippets);
        self.clipboard.copy(processed_text).await?;

        Ok(())
    }

    async fn save_item(&self, args: contracts::SaveItemArgs) -> Result<uuid::Uuid> {
        if let Some(id) = args.id {
            // We need to find the prompt to update it
            let all = self.storage.get_prompts(contracts::PromptFilter::default()).await?;
            if let Some(mut p) = all.into_iter().find(|p| p.id == id) {
                p.text = args.text;
                p.name = args.title;
                p.updated_at = chrono::Utc::now();

                // Unstage if it's a draft
                if (args.tab == Prompts || args.tab == Tab::Canned)
                    && Processor::is_draft(&Processor::get_display_title(&p.text).0)
                {
                    p.staged = false;
                }

                self.storage.save_prompt(p).await?;
                return Ok(id);
            }
            Err(contracts::Error::NotFound)
        } else {
            let r#type = match args.tab {
                Tab::Notes => contracts::PromptType::Note,
                Tab::Snippets => contracts::PromptType::Snippet,
                _ => contracts::PromptType::Prompt,
            };

            let is_global_tab = args.tab == Tab::Canned || args.tab == Tab::Snippets;
            let folder = if is_global_tab { None } else { Some(args.project_path.clone()) };
            let branch = if is_global_tab { None } else { args.branch.clone() };
            let project_id = if is_global_tab { None } else { args.project_id };

            let mut prompt =
                contracts::Prompt::new(args.text, r#type, folder, branch, args.title, project_id);

            // A new item is never staged by default, but let's be safe
            if (args.tab == Prompts || args.tab == Tab::Canned)
                && Processor::is_draft(&Processor::get_display_title(&prompt.text).0)
            {
                prompt.staged = false;
            }

            let saved_id = prompt.id;

            if let Some(idx) = args.insert_index {
                // Get all prompts for the current view to calculate order_index
                let filter = contracts::PromptFilter {
                    folder: if args.tab == Tab::Canned {
                        None
                    } else {
                        Some(args.project_path.clone())
                    },
                    project_id: args.project_id,
                    branch: if args.tab == Tab::Prompts { args.branch } else { None },
                    tab: Some(args.tab),
                    project_filter: args.project_id.is_some(),
                    staged: None,
                };
                let existing = self.storage.get_prompts(filter).await?;

                // If we are inserting, we might need to re-index everything to be safe
                // or just pick an order_index.
                // Since we sort by order_index ASC, created_at DESC:
                // Newer items with same order_index are at the top.

                if idx == 0 {
                    // Insert at top
                    if let Some(first) = existing.first() {
                        prompt.order_index = first.order_index - 1;
                    } else {
                        prompt.order_index = 0;
                    }
                } else if idx >= existing.len() {
                    // Insert at bottom
                    if let Some(last) = existing.last() {
                        prompt.order_index = last.order_index + 1;
                    } else {
                        prompt.order_index = 0;
                    }
                } else {
                    // Insert between existing[idx-1] and existing[idx]
                    let mut new_list = Vec::new();
                    for (i, mut p) in existing.into_iter().enumerate() {
                        if i == idx {
                            prompt.order_index = i32::try_from(i).unwrap_or(i32::MAX) * 10;
                            new_list.push(prompt.clone());
                        }
                        p.order_index =
                            i32::try_from(i + usize::from(i >= idx)).unwrap_or(i32::MAX) * 10;
                        new_list.push(p);
                    }
                    self.storage.save_prompts(new_list).await?;
                    return Ok(saved_id);
                }
            }

            self.storage.save_prompt(prompt).await?;
            Ok(saved_id)
        }
    }

    async fn search_files(&self, base_dir: &str, query: &str) -> Result<Vec<Prompt>> {
        let base_path = std::path::PathBuf::from(base_dir);
        let mut matcher = Matcher::new(Config::DEFAULT);
        let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
        let mut buf = Vec::new();

        let mut scored_results = Vec::new();
        walk_recursive(
            &base_path,
            &base_path,
            &pattern,
            &mut matcher,
            &mut buf,
            &mut scored_results,
        );
        scored_results.sort_by_key(|b| std::cmp::Reverse(b.0));

        Ok(scored_results.into_iter().take(20).map(|(_, p)| p).collect())
    }

    async fn get_claude_commands(&self, project_path: &str) -> Result<Vec<Prompt>> {
        Ok(crate::claude::discover_commands(project_path))
    }

    async fn export_data(&self, include_archived: bool) -> Result<String> {
        let mut data = self.storage.get_all_data().await?;
        if !include_archived {
            data.prompts.retain(|p| !p.is_archived);
        }
        toml::to_string_pretty(&data).map_err(|e| contracts::Error::Storage(e.to_string()))
    }

    async fn import_data(&self, toml_data: &str) -> Result<()> {
        let data: contracts::DatabaseExport =
            toml::from_str(toml_data).map_err(|e| contracts::Error::Storage(e.to_string()))?;
        self.storage.restore_all_data(data).await
    }
}
