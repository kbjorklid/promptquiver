use contracts::{Prompt, PromptType};
use std::path::{Path, PathBuf};
use yaml_serde::Value;

pub fn discover_commands(project_path: &str) -> Vec<Prompt> {
    let claude_dir =
        directories::UserDirs::new().map(|u| u.home_dir().to_path_buf().join(".claude"));

    let plugin_dirs: Vec<PathBuf> = claude_dir
        .as_ref()
        .map(|d| enumerate_plugin_dirs(&d.join("plugins").join("cache")))
        .unwrap_or_default();

    let project_root = find_project_root(Path::new(project_path));

    collect_commands(claude_dir.as_deref(), &plugin_dirs, project_root.as_deref())
}

fn enumerate_plugin_dirs(cache: &Path) -> Vec<PathBuf> {
    if !cache.exists() {
        return Vec::new();
    }
    let mut result = Vec::new();
    collect_plugin_content_dirs(cache, 0, &mut result);
    result
}

fn collect_plugin_content_dirs(dir: &Path, depth: usize, result: &mut Vec<PathBuf>) {
    if depth > 5 {
        return;
    }
    if dir.join("commands").is_dir() || dir.join("skills").is_dir() {
        result.push(dir.to_path_buf());
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|ft| ft.is_dir()) {
                collect_plugin_content_dirs(&entry.path(), depth + 1, result);
            }
        }
    }
}

fn collect_commands(
    claude_dir: Option<&Path>,
    plugin_dirs: &[PathBuf],
    project_root: Option<&Path>,
) -> Vec<Prompt> {
    let mut commands = Vec::new();

    if let Some(dir) = claude_dir {
        commands.extend(scan_directory(&dir.join("commands"), None));
        commands.extend(scan_directory(&dir.join("skills"), None));
    }

    for plugin_dir in plugin_dirs {
        let plugin_name = plugin_dir.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str());
        commands.extend(scan_directory(&plugin_dir.join("commands"), plugin_name));
        commands.extend(scan_directory(&plugin_dir.join("skills"), plugin_name));
    }

    if let Some(root) = project_root {
        commands.extend(scan_directory(&root.join(".claude").join("commands"), None));
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
        if current.join(".git").exists() {
            return Some(current.to_path_buf());
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }
    None
}

fn scan_directory(dir: &Path, prefix: Option<&str>) -> Vec<Prompt> {
    let mut commands = Vec::new();
    if !dir.exists() {
        return commands;
    }

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let name = match prefix {
                        Some(p) => format!("{p}:{stem}"),
                        None => stem.to_string(),
                    };
                    let content = std::fs::read_to_string(&path).unwrap_or_default();
                    let description = extract_description(&content);
                    commands.push(Prompt::new(
                        description,
                        PromptType::Prompt,
                        None,
                        None,
                        Some(name),
                        None,
                    ));
                }
            } else if path.is_dir() {
                // Nested skill layout: skills/<skill-name>/SKILL.md
                let skill_md = path.join("SKILL.md");
                if skill_md.exists() {
                    if let Some(dir_name) = path.file_name().and_then(|s| s.to_str()) {
                        let name = match prefix {
                            Some(p) => format!("{p}:{dir_name}"),
                            None => dir_name.to_string(),
                        };
                        let content = std::fs::read_to_string(&skill_md).unwrap_or_default();
                        let description = extract_description(&content);
                        commands.push(Prompt::new(
                            description,
                            PromptType::Prompt,
                            None,
                            None,
                            Some(name),
                            None,
                        ));
                    }
                }
            }
        }
    }
    commands
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_plugin_nested_skill_dir_with_skill_md() {
        // Real plugin layout: skills/<skill-name>/SKILL.md
        let tmp = tempdir().unwrap();
        let version_dir = tmp.path().join("my-plugin").join("v1");
        let skill_dir = version_dir.join("skills").join("writing-clearly-and-concisely");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "---\ndescription: Write clearly\n---\n").unwrap();

        let results = collect_commands(None, &[version_dir], None);

        assert!(
            results
                .iter()
                .any(|p| p.name.as_deref() == Some("my-plugin:writing-clearly-and-concisely")),
            "nested SKILL.md should be discovered with plugin prefix, got: {:?}",
            results.iter().map(|p| p.name.as_deref()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_plugin_skills_dir_is_included_in_suggestions() {
        let tmp = tempdir().unwrap();
        let version_dir = tmp.path().join("my-plugin").join("v1");
        let skills_dir = version_dir.join("skills");
        fs::create_dir_all(&skills_dir).unwrap();
        fs::write(skills_dir.join("commit.md"), "---\ndescription: Create a git commit\n---\n")
            .unwrap();

        let results = collect_commands(None, &[version_dir], None);

        assert!(
            results.iter().any(|p| p.name.as_deref() == Some("my-plugin:commit")),
            "skill 'commit' should be discovered with plugin prefix, got: {:?}",
            results.iter().map(|p| p.name.as_deref()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_global_skills_dir_is_included_in_suggestions() {
        let tmp = tempdir().unwrap();
        let claude_dir = tmp.path().join(".claude");
        let skills_dir = claude_dir.join("skills");
        fs::create_dir_all(&skills_dir).unwrap();
        fs::write(skills_dir.join("review.md"), "---\ndescription: Review code\n---\n").unwrap();

        let results = collect_commands(Some(&claude_dir), &[], None);

        assert!(
            results.iter().any(|p| p.name.as_deref() == Some("review")),
            "skill 'review' should be discovered from global skills dir"
        );
    }

    #[test]
    fn test_plugin_commands_dir_still_works() {
        let tmp = tempdir().unwrap();
        let version_dir = tmp.path().join("my-plugin").join("v1");
        let commands_dir = version_dir.join("commands");
        fs::create_dir_all(&commands_dir).unwrap();
        fs::write(commands_dir.join("deploy.md"), "---\ndescription: Deploy app\n---\n").unwrap();

        let results = collect_commands(None, &[version_dir], None);

        assert!(
            results.iter().any(|p| p.name.as_deref() == Some("my-plugin:deploy")),
            "command 'deploy' should be discovered with plugin prefix, got: {:?}",
            results.iter().map(|p| p.name.as_deref()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_nested_plugin_cache_structure() {
        // Real cache layout: cache/<group>/<plugin>/<version>/skills/
        let tmp = tempdir().unwrap();
        let cache = tmp.path();

        let version_dir = cache.join("my-group").join("my-plugin").join("abc123");
        let skills_dir = version_dir.join("skills");
        fs::create_dir_all(&skills_dir).unwrap();
        fs::write(skills_dir.join("my-skill.md"), "---\ndescription: My skill\n---\n").unwrap();

        let plugin_dirs = enumerate_plugin_dirs(cache);
        let results = collect_commands(None, &plugin_dirs, None);

        assert!(
            results.iter().any(|p| p.name.as_deref() == Some("my-plugin:my-skill")),
            "skill should be prefixed with plugin name: expected 'my-plugin:my-skill', got: {:?}",
            results.iter().map(|p| p.name.as_deref()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_plugin_command_has_plugin_name_prefix() {
        // Real cache layout: cache/<group>/<plugin-name>/<version>/commands/
        let tmp = tempdir().unwrap();
        let cache = tmp.path();

        let version_dir = cache.join("my-group").join("ddd-domain-design").join("abc123");
        let commands_dir = version_dir.join("commands");
        fs::create_dir_all(&commands_dir).unwrap();
        fs::write(commands_dir.join("ddd-design.md"), "---\ndescription: Design DDD\n---\n")
            .unwrap();

        let plugin_dirs = enumerate_plugin_dirs(cache);
        let results = collect_commands(None, &plugin_dirs, None);

        assert!(
            results.iter().any(|p| p.name.as_deref() == Some("ddd-domain-design:ddd-design")),
            "plugin command should be prefixed with plugin name, got: {:?}",
            results.iter().map(|p| p.name.as_deref()).collect::<Vec<_>>()
        );
    }
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
