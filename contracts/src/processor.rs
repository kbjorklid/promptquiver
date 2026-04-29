use crate::Prompt;
use regex::Regex;

#[derive(Debug)]
pub struct Processor;

impl Processor {
    /// Extracts a title from the text if it follows the pattern:
    /// -- Title
    ///
    /// (Empty line after title)
    pub fn extract_title(text: &str) -> (Option<String>, String) {
        let lines: Vec<&str> = text.lines().collect();
        if lines.len() >= 2 && lines[0].starts_with("--") && lines[1].trim().is_empty() {
            let title = lines[0].trim_start_matches("--").trim().to_string();
            let remaining = lines[2..].join("\n");
            (Some(title), remaining)
        } else {
            (None, text.to_string())
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
    use crate::{Prompt, PromptType};

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
    fn test_expand_snippets() {
        let snippets = vec![
            Prompt::new("expanded_content".to_string(), PromptType::Snippet, None, None, Some("mysnip".to_string())),
        ];
        let text = "Use $$mysnip here";
        assert_eq!(Processor::expand_snippets(text, &snippets), "Use expanded_content here");
    }
}
