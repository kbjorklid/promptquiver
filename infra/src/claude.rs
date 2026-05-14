use contracts::{Prompt, PromptType};
use std::path::{Path, PathBuf};
use yaml_serde::Value;

pub fn discover_commands(project_path: &str) -> Vec<Prompt> {
    let mut commands = Vec::new();

    // Global path: ~/.claude/commands/ and ~/.claude/plugins/cache/*/commands/
    if let Some(user_dirs) = directories::UserDirs::new() {
        let global_path = user_dirs.home_dir().join(".claude").join("commands");
        commands.extend(scan_directory(&global_path));

        let cache_path = user_dirs.home_dir().join(".claude").join("plugins").join("cache");
        if cache_path.exists() {
            if let Ok(entries) = std::fs::read_dir(cache_path) {
                for entry in entries.flatten() {
                    if entry.file_type().is_ok_and(|ft| ft.is_dir()) {
                        let plugin_commands_path = entry.path().join("commands");
                        commands.extend(scan_directory(&plugin_commands_path));
                    }
                }
            }
        }
    }

    // Project path
    if let Some(root) = find_project_root(Path::new(project_path)) {
        let project_commands = root.join(".claude").join("commands");
        commands.extend(scan_directory(&project_commands));
    }

    // Deduplicate by name (project overrides global)
    let mut map = std::collections::HashMap::new();
    for cmd in commands {
        if let Some(name) = &cmd.name {
            map.insert(name.clone(), cmd);
        }
    }

    map.into_values().collect()
}

fn find_project_root(start_path: &Path) -> Option<PathBuf> {
    let mut current = start_path;
    loop {
        if current.join(".git").exists()
            || current.join("CLAUDE.md").exists()
            || current.join("package.json").exists()
        {
            return Some(current.to_path_buf());
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }
    None
}

fn scan_directory(dir: &Path) -> Vec<Prompt> {
    let mut commands = Vec::new();
    if !dir.exists() {
        return commands;
    }

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    let content = std::fs::read_to_string(&path).unwrap_or_default();
                    let description = extract_description(&content);
                    commands.push(Prompt::new(
                        description,
                        PromptType::Prompt,
                        None,
                        None,
                        Some(name.to_string()),
                        None,
                    ));
                }
            }
        }
    }
    commands
}

fn extract_description(content: &str) -> String {
    // Extract frontmatter between --- and ---
    if content.starts_with("---\n") || content.starts_with("---\r\n") {
        let parts: Vec<&str> = content.split("---").collect();
        if parts.len() >= 3 {
            let frontmatter = parts[1];
            if let Ok(val) = yaml_serde::from_str::<Value>(frontmatter) {
                if let Some(desc) = val.get("description").and_then(|v| v.as_str()) {
                    return desc.to_string();
                }
            }
        }
    }
    String::new()
}
