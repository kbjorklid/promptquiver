use async_trait::async_trait;
use contracts::{AppService, Clipboard, Prompt, Result, Storage, Tab, Tab::*, Processor, PromptFilter};
use std::sync::Arc;
use uuid;
use chrono;

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

#[async_trait]
impl AppService for RealAppService {
    async fn stage_item(&self, _folder: &str, tab: Tab, mut item: Prompt) -> Result<()> {
        if tab == Settings {
            return Ok(());
        }

        let snippets = self.storage.get_prompts(PromptFilter { tab: Some(Snippets), ..Default::default() }).await?;

        // Alias for Notes and Snippets: they cannot be staged anymore
        if tab == Notes || tab == Snippets {
            let processed_text = Processor::process(&item.text, &snippets);
            self.clipboard.copy(processed_text).await?;
            return Ok(());
        }

        // Check if it's a draft (only for Prompts/Canned)
        if (tab == Prompts || tab == Tab::Canned) && Processor::is_draft(&Processor::get_display_title(&item.text).0) {
            return Err(contracts::Error::Storage("Cannot stage a draft prompt. Remove 'Draft' from the title first.".to_string()));
        }

        if item.staged {
            // Un-stage
            item.staged = false;
            self.storage.save_prompt(item).await?;
        } else {
            // Stage
            // Unstage and archive other staged items in the same scope?
            // Old behavior: "Remove archived items from their original lists. Add to archive (to the top)"
            let all_in_scope = self.storage.get_prompts(PromptFilter { folder: item.folder.clone(), ..Default::default() }).await?;
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

    async fn duplicate_item(&self, _folder: &str, tab: Tab, item: Prompt) -> Result<Option<Prompt>> {
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
        let snippets = self.storage.get_prompts(PromptFilter { tab: Some(Snippets), ..Default::default() }).await?;
        let processed_text = Processor::process(&item.text, &snippets);
        self.clipboard.copy(processed_text).await?;

        Ok(())
    }

    async fn save_item(&self, folder: &str, tab: Tab, text: String, title: Option<String>, id: Option<uuid::Uuid>, insert_index: Option<usize>, branch: Option<String>, project_id: Option<uuid::Uuid>) -> Result<()> {
        if let Some(id) = id {
            // We need to find the prompt to update it
            let all = self.storage.get_prompts(contracts::PromptFilter::default()).await?;
            if let Some(mut p) = all.into_iter().find(|p| p.id == id) {
                p.text = text;
                p.name = title;
                p.updated_at = chrono::Utc::now();
                
                // Unstage if it's a draft
                if (tab == Prompts || tab == Tab::Canned) && Processor::is_draft(&Processor::get_display_title(&p.text).0) {
                    p.staged = false;
                }

                self.storage.save_prompt(p).await?;
            }
        } else {
            let r#type = match tab {
                Tab::Notes => contracts::PromptType::Note,
                Tab::Snippets => contracts::PromptType::Snippet,
                _ => contracts::PromptType::Prompt,
            };
            
            let mut prompt = contracts::Prompt::new(text, r#type, Some(folder.to_string()), branch.clone(), title, project_id);
            if tab == Tab::Canned {
                prompt.folder = None;
            }

            // A new item is never staged by default, but let's be safe
            if (tab == Prompts || tab == Tab::Canned) && Processor::is_draft(&Processor::get_display_title(&prompt.text).0) {
                prompt.staged = false;
            }

            if let Some(idx) = insert_index {
                // Get all prompts for the current view to calculate order_index
                let filter = contracts::PromptFilter {
                    folder: if tab == Tab::Canned { None } else { Some(folder.to_string()) },
                    project_id,
                    branch: if tab == Tab::Prompts { branch } else { None },
                    tab: Some(tab),
                    project_filter: project_id.is_some(),
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
                    // Since it's at the bottom, we want it to be LAST.
                    // If multiple have same order_index, newest is first.
                    // So for bottom insertion, we MUST have a higher order_index.
                } else {
                    // Insert between existing[idx-1] and existing[idx]
                    // We'll re-index everything to ensure there's a gap or just use the mid point if we use larger gaps.
                    // Let's just re-index everything starting from 0, with gaps of 10.
                    let mut new_list = Vec::new();
                    for (i, mut p) in existing.into_iter().enumerate() {
                        if i == idx {
                            prompt.order_index = (i as i32) * 10;
                            new_list.push(prompt.clone());
                        }
                        p.order_index = ((i + if i >= idx { 1 } else { 0 }) as i32) * 10;
                        new_list.push(p);
                    }
                    self.storage.save_prompts(new_list).await?;
                    return Ok(());
                }
            }
            
            self.storage.save_prompt(prompt).await?;
        }
        Ok(())
    }

    async fn search_files(&self, base_dir: &str, query: &str) -> Result<Vec<Prompt>> {
        use fuzzy_matcher::FuzzyMatcher;
        use fuzzy_matcher::skim::SkimMatcherV2;

        let base_path = std::path::PathBuf::from(base_dir);
        let matcher = SkimMatcherV2::default();
        let query_normalized = query.replace('\\', "/").to_lowercase();
        
        let mut scored_results = Vec::new();

        fn walk_recursive(base_dir: &std::path::Path, current_dir: &std::path::Path, query_normalized: &str, matcher: &SkimMatcherV2, results: &mut Vec<(i64, Prompt)>) {
            if results.len() >= 1000 { return; }
            if let Ok(entries) = std::fs::read_dir(current_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let relative_path = path.strip_prefix(base_dir).unwrap_or(&path).to_string_lossy().to_string();
                    let path_normalized = relative_path.replace('\\', "/");
                    let path_lower = path_normalized.to_lowercase();

                    if path.is_dir() {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            if name == "target" || name == ".git" || name == "node_modules" || name.starts_with('.') { continue; }
                        }
                        
                        // Suggest the directory itself
                        let dir_path_normalized = if path_normalized.ends_with('/') || path_normalized.is_empty() { 
                            path_normalized.clone() 
                        } else { 
                            format!("{}/", path_normalized) 
                        };
                        let dir_path_lower = dir_path_normalized.to_lowercase();
                        
                        if !dir_path_normalized.is_empty() {
                            if let Some(score) = matcher.fuzzy_match(&dir_path_lower, query_normalized) {
                                let mut final_score = score + 50; // Directory bonus
                                if dir_path_lower.contains(query_normalized) { final_score += 100; }
                                results.push((final_score, Prompt::new(path.to_string_lossy().to_string(), contracts::PromptType::Note, None, None, Some(dir_path_normalized), None)));
                            }
                        }

                        walk_recursive(base_dir, &path, query_normalized, matcher, results);
                    } else {
                        if let Some(score) = matcher.fuzzy_match(&path_lower, query_normalized) {
                            let mut final_score = score;
                            if path_lower.contains(query_normalized) { final_score += 100; }
                            results.push((final_score, Prompt::new(path.to_string_lossy().to_string(), contracts::PromptType::Note, None, None, Some(path_normalized), None)));
                        }
                    }
                }
            }
        }

        walk_recursive(&base_path, &base_path, &query_normalized, &matcher, &mut scored_results);
        scored_results.sort_by_key(|b| std::cmp::Reverse(b.0));
        
        Ok(scored_results.into_iter().take(20).map(|(_, p)| p).collect())
    }
}
