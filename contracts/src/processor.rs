use crate::Prompt;
use regex::Regex;

#[derive(Debug)]
pub struct Processor;

impl Processor {
    /// Extracts a title from the text if it follows the pattern:
    /// -- Title
    ///
    /// (Empty line after title, or end of string)
    pub fn extract_title(text: &str) -> (Option<String>, String) {
        let lines: Vec<&str> = text.lines().collect();
        if lines.is_empty() {
            return (None, text.to_string());
        }

        if lines[0].starts_with("--") {
            if lines.len() == 1 {
                let title = lines[0].trim_start_matches("--").trim().to_string();
                return (Some(title), String::new());
            }
            if lines[1].trim().is_empty() {
                let title = lines[0].trim_start_matches("--").trim().to_string();
                let remaining = lines[2..].join("\n");
                return (Some(title), remaining);
            }
        }
        (None, text.to_string())
    }

    /// Checks if a title indicates a draft.
    /// A title is a draft if it:
    /// - starts with "Draft " (case-insensitive)
    /// - starts with "[Draft]" (case-insensitive)
    /// - ends with "[Draft]" (case-insensitive)
    pub fn is_draft(title: &str) -> bool {
        let t = title.trim();
        let lower = t.to_lowercase();
        
        lower.starts_with("draft ") || 
        lower.starts_with("[draft]") || 
        lower.ends_with("[draft]") ||
        lower == "draft"
    }

    /// Returns the display title and whether it's a draft.
    /// If it's a draft, the title is cleaned (marker removed) and prefixed with [DRAFT].
    pub fn get_display_title(text: &str) -> (String, bool) {
        let (extracted, _) = Self::extract_title(text);
        let raw_title = extracted.unwrap_or_else(|| {
            let first_line = text.lines().next().unwrap_or("");
            first_line.trim_start_matches("--").trim().to_string()
        });

        if Self::is_draft(&raw_title) {
            let mut cleaned = raw_title.trim().to_string();
            let lower = cleaned.to_lowercase();

            if lower.starts_with("draft ") {
                cleaned = cleaned[6..].trim().to_string();
            } else if lower.starts_with("[draft]") {
                cleaned = cleaned[7..].trim().to_string();
            } else if lower.ends_with("[draft]") {
                cleaned = cleaned[..cleaned.len() - 7].trim().to_string();
            } else if lower == "draft" {
                cleaned = String::new();
            }

            let display = if cleaned.is_empty() {
                "[DRAFT]".to_string()
            } else {
                format!("[DRAFT] {cleaned}")
            };
            (display, true)
        } else {
            (raw_title, false)
        }
    }

    /// Strips all lines starting with '--' (no leading whitespace)
    pub fn strip_comments(text: &str) -> String {
        text.lines()
            .filter(|line| !line.starts_with("--"))
            .collect::<Vec<&str>>()
            .join("\n")
    }

    /// Expands snippets in the format $$name
    ///
    /// # Panics
    ///
    /// This function will panic if the internal snippet regex fails to compile, 
    /// which should never happen as the pattern is static and valid.
    pub fn expand_snippets(text: &str, snippets: &[Prompt]) -> String {
        let mut result = text.to_string();
        let re = Regex::new(r"\$\$([a-zA-Z0-9_-]+)").unwrap();
        
        // We do this multiple times in case of nested snippets? 
        // The spec doesn't specify recursion, but we'll do one pass.
        let mut replaced = true;
        while replaced {
            replaced = false;
            let current = result.clone();
            let mut new_result = String::new();
            let mut last_end = 0;

            for cap in re.captures_iter(&current) {
                let m = cap.get(0).unwrap();
                let name = cap.get(1).unwrap().as_str();
                
                new_result.push_str(&current[last_end..m.start()]);
                
                if let Some(snippet) = snippets.iter().find(|s| s.name.as_deref() == Some(name)) {
                    new_result.push_str(&snippet.text);
                    replaced = true;
                } else {
                    new_result.push_str(m.as_str());
                }
                last_end = m.end();
            }
            new_result.push_str(&current[last_end..]);
            result = new_result;
            
            // To prevent infinite loops in case of circular references
            if !replaced { break; }
        }
        
        result
    }

    /// Full processing: Extract title, strip comments, expand snippets
    pub fn process(text: &str, snippets: &[Prompt]) -> String {
        let (_, processed) = Self::extract_title(text);
        let processed = Self::strip_comments(&processed);
        Self::expand_snippets(&processed, snippets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_title() {
        let text = "-- My Title\n\nContent starts here";
        let (title, content) = Processor::extract_title(text);
        assert_eq!(title, Some("My Title".to_string()));
        assert_eq!(content, "Content starts here");

        let no_title = "Just content";
        let (title, content) = Processor::extract_title(no_title);
        assert_eq!(title, None);
        assert_eq!(content, "Just content");
    }

    #[test]
    fn test_strip_comments() {
        let text = "Line 1\n-- Comment\nLine 2";
        assert_eq!(Processor::strip_comments(text), "Line 1\nLine 2");
    }

    #[test]
    fn test_is_draft() {
        assert!(Processor::is_draft("Draft Fix welcome email"));
        assert!(Processor::is_draft("[Draft] Fix welcome email"));
        assert!(Processor::is_draft("Fix welcome email [Draft]"));
        assert!(Processor::is_draft("DRAFT Fix welcome email"));
        assert!(Processor::is_draft("[draft] Fix welcome email"));
        assert!(Processor::is_draft("draft"));
        
        assert!(!Processor::is_draft("Drafting a document"));
        assert!(!Processor::is_draft("My Draft version"));
        assert!(!Processor::is_draft("Fix welcome email"));
    }

    #[test]
    fn test_get_display_title() {
        assert_eq!(Processor::get_display_title("-- Draft Title\n\nContent").0, "[DRAFT] Title");
        assert_eq!(Processor::get_display_title("-- [Draft] Title\n\nContent").0, "[DRAFT] Title");
        assert_eq!(Processor::get_display_title("-- Title [Draft]\n\nContent").0, "[DRAFT] Title");
        assert_eq!(Processor::get_display_title("-- draft\n\nContent").0, "[DRAFT]");
        assert_eq!(Processor::get_display_title("No extractable title").0, "No extractable title");
    }

    #[test]
    fn test_expand_snippets() {
        let snippets = vec![
            Prompt::new("Snippet Content".to_string(), crate::PromptType::Snippet, None, None, Some("mysnip".to_string()), None),
        ];
        let text = "Use $$mysnip here";
        assert_eq!(Processor::expand_snippets(text, &snippets), "Use Snippet Content here");
        
        let text_missing = "Use $$unknown here";
        assert_eq!(Processor::expand_snippets(text_missing, &snippets), "Use $$unknown here");
    }
}
