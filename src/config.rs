use anyhow::Result;
use std::fs;
use std::path::PathBuf;

/// Branch type configuration for automatic base branch selection
#[derive(Debug, Clone)]
pub struct BranchType {
    pub name: String,      // "feature", "hotfix" etc.
    pub prefix: String,    // "feature/", "hotfix/" etc.
    pub base: String,      // "develop", "main" etc.
    pub shortcut: char,    // 'f', 'h' etc.
}

impl BranchType {
    pub fn new(name: &str, prefix: &str, base: &str, shortcut: char) -> Self {
        Self {
            name: name.to_string(),
            prefix: prefix.to_string(),
            base: base.to_string(),
            shortcut,
        }
    }
}

/// Default branch types for git-flow style workflow
pub fn default_branch_types() -> Vec<BranchType> {
    vec![
        BranchType::new("feature", "feature/", "develop", 'f'),
        BranchType::new("hotfix", "hotfix/", "main", 'h'),
        BranchType::new("release", "release/", "develop", 'r'),
        BranchType::new("bugfix", "bugfix/", "develop", 'b'),
    ]
}

#[derive(Debug, Default)]
pub struct Config {
    pub editor: Option<String>,
    pub terminal: Option<String>,
    pub copy_files: Vec<String>,        // Files to copy when adding worktree
    pub post_add_script: Option<String>, // Script to run after adding worktree
    pub branch_types: Vec<BranchType>,  // Branch type configurations
}

impl Config {
    /// Load config with project-level override support
    /// Priority: project (.owt/config.toml) > global (~/.config/owt/config.toml)
    #[allow(dead_code)]
    pub fn load() -> Result<Self> {
        Self::load_with_project(None)
    }

    /// Load config with optional project path for project-level config
    pub fn load_with_project(bare_repo_path: Option<&std::path::Path>) -> Result<Self> {
        // Start with global config
        let global_path = Self::global_config_path();
        let mut config = if global_path.exists() {
            let content = fs::read_to_string(&global_path)?;
            Self::parse(&content)?
        } else {
            Self::default()
        };

        // Override with project-level config if exists
        if let Some(bare_path) = bare_repo_path {
            let project_path = Self::project_config_path(bare_path);
            if project_path.exists() {
                let content = fs::read_to_string(&project_path)?;
                let project_config = Self::parse(&content)?;
                config.merge_from(project_config);
            }
        }

        // If no branch types configured, use defaults
        if config.branch_types.is_empty() {
            config.branch_types = default_branch_types();
        }

        Ok(config)
    }

    /// Merge project config into self (project overrides global)
    fn merge_from(&mut self, other: Config) {
        if other.editor.is_some() {
            self.editor = other.editor;
        }
        if other.terminal.is_some() {
            self.terminal = other.terminal;
        }
        if !other.copy_files.is_empty() {
            self.copy_files = other.copy_files;
        }
        if other.post_add_script.is_some() {
            self.post_add_script = other.post_add_script;
        }
        if !other.branch_types.is_empty() {
            self.branch_types = other.branch_types;
        }
    }

    /// Global config path: ~/.config/owt/config.toml
    pub fn global_config_path() -> PathBuf {
        let config_dir = dirs_config_dir().join("owt");
        config_dir.join("config.toml")
    }

    /// Project config path: .owt/config.toml (relative to bare repo parent)
    pub fn project_config_path(bare_repo_path: &std::path::Path) -> PathBuf {
        Self::owt_dir(bare_repo_path).join("config.toml")
    }

    /// Legacy: for backwards compatibility
    #[allow(dead_code)]
    pub fn config_path() -> PathBuf {
        Self::global_config_path()
    }

    /// Save config to global config file
    pub fn save(&self) -> Result<()> {
        self.save_to(&Self::global_config_path())
    }

    /// Save config to project-level config file
    pub fn save_to_project(&self, bare_repo_path: &std::path::Path) -> Result<()> {
        self.save_to(&Self::project_config_path(bare_repo_path))
    }

    /// Save config to specified path
    fn save_to(&self, config_path: &PathBuf) -> Result<()> {
        let config_dir = config_path.parent().unwrap();

        if !config_dir.exists() {
            fs::create_dir_all(config_dir)?;
        }

        let mut content = String::new();

        if let Some(ref editor) = self.editor {
            content.push_str(&format!("editor = \"{}\"\n", editor));
        }
        if let Some(ref terminal) = self.terminal {
            content.push_str(&format!("terminal = \"{}\"\n", terminal));
        }
        if !self.copy_files.is_empty() {
            let files = self.copy_files
                .iter()
                .map(|f| format!("\"{}\"", f))
                .collect::<Vec<_>>()
                .join(", ");
            content.push_str(&format!("copy_files = [{}]\n", files));
        }
        if let Some(ref script) = self.post_add_script {
            content.push_str(&format!("post_add_script = \"{}\"\n", script));
        }

        // Write branch types
        if !self.branch_types.is_empty() {
            content.push('\n');
            for bt in &self.branch_types {
                content.push_str("[[branch_types]]\n");
                content.push_str(&format!("name = \"{}\"\n", bt.name));
                content.push_str(&format!("prefix = \"{}\"\n", bt.prefix));
                content.push_str(&format!("base = \"{}\"\n", bt.base));
                content.push_str(&format!("shortcut = \"{}\"\n", bt.shortcut));
                content.push('\n');
            }
        }

        fs::write(config_path, content)?;
        Ok(())
    }

    fn parse(content: &str) -> Result<Self> {
        let mut config = Config::default();
        let mut in_branch_type = false;
        let mut current_bt: Option<BranchTypeBuilder> = None;

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Handle section headers
            if line.starts_with('[') {
                // Finalize previous branch_type if any
                if let Some(bt) = current_bt.take() {
                    if let Some(branch_type) = bt.build() {
                        config.branch_types.push(branch_type);
                    }
                }

                if line == "[[branch_types]]" {
                    in_branch_type = true;
                    current_bt = Some(BranchTypeBuilder::default());
                } else {
                    in_branch_type = false;
                }
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');

                if in_branch_type {
                    if let Some(ref mut bt) = current_bt {
                        match key {
                            "name" => bt.name = Some(value.to_string()),
                            "prefix" => bt.prefix = Some(value.to_string()),
                            "base" => bt.base = Some(value.to_string()),
                            "shortcut" => bt.shortcut = value.chars().next(),
                            _ => {}
                        }
                    }
                } else {
                    match key {
                        "editor" => config.editor = Some(value.to_string()),
                        "terminal" => config.terminal = Some(value.to_string()),
                        "post_add_script" => config.post_add_script = Some(value.to_string()),
                        "copy_files" => {
                            // Parse comma-separated list or array-like syntax
                            let files: Vec<String> = value
                                .trim_matches('[').trim_matches(']')
                                .split(',')
                                .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                            config.copy_files = files;
                        }
                        _ => {}
                    }
                }
            }
        }

        // Finalize last branch_type if any
        if let Some(bt) = current_bt {
            if let Some(branch_type) = bt.build() {
                config.branch_types.push(branch_type);
            }
        }

        Ok(config)
    }
}

/// Builder for parsing branch types from TOML
#[derive(Default)]
struct BranchTypeBuilder {
    name: Option<String>,
    prefix: Option<String>,
    base: Option<String>,
    shortcut: Option<char>,
}

impl BranchTypeBuilder {
    fn build(self) -> Option<BranchType> {
        Some(BranchType {
            name: self.name?,
            prefix: self.prefix?,
            base: self.base?,
            shortcut: self.shortcut?,
        })
    }
}

impl Config {
    pub fn get_editor(&self) -> String {
        self.editor
            .clone()
            .or_else(|| std::env::var("EDITOR").ok())
            .unwrap_or_else(|| "vim".to_string())
    }

    pub fn get_terminal(&self) -> Option<String> {
        self.terminal
            .clone()
            .or_else(|| std::env::var("TERMINAL").ok())
    }

    /// Get the .owt directory path (in bare repo parent)
    pub fn owt_dir(bare_repo_path: &std::path::Path) -> PathBuf {
        bare_repo_path
            .parent()
            .map(|p| p.join(".owt"))
            .unwrap_or_else(|| PathBuf::from(".owt"))
    }

    /// Get the post-add script path
    pub fn post_add_script_path(bare_repo_path: &std::path::Path) -> PathBuf {
        Self::owt_dir(bare_repo_path).join("post-add.sh")
    }

    /// Find branch type by shortcut key
    pub fn find_branch_type_by_shortcut(&self, shortcut: char) -> Option<&BranchType> {
        self.branch_types.iter().find(|bt| bt.shortcut == shortcut)
    }
}

fn dirs_config_dir() -> PathBuf {
    // Try XDG_CONFIG_HOME first, then fall back to ~/.config
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg);
    }

    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config");
    }

    PathBuf::from(".config")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let config = Config::parse("").unwrap();
        assert!(config.editor.is_none());
        assert!(config.terminal.is_none());
    }

    #[test]
    fn test_parse_values() {
        let content = r#"
[core]
editor = "code"
terminal = "Ghostty"
"#;
        let config = Config::parse(content).unwrap();
        assert_eq!(config.editor, Some("code".to_string()));
        assert_eq!(config.terminal, Some("Ghostty".to_string()));
    }

    #[test]
    fn test_parse_with_comments() {
        let content = r#"
# This is a comment
editor = vim
# terminal = iTerm
"#;
        let config = Config::parse(content).unwrap();
        assert_eq!(config.editor, Some("vim".to_string()));
        assert!(config.terminal.is_none());
    }

    #[test]
    fn test_parse_copy_files() {
        let content = r#"
copy_files = [".env", ".envrc", "config.json"]
"#;
        let config = Config::parse(content).unwrap();
        assert_eq!(config.copy_files, vec![".env", ".envrc", "config.json"]);
    }

    #[test]
    fn test_parse_branch_types() {
        let content = r#"
editor = "vim"

[[branch_types]]
name = "feature"
prefix = "feature/"
base = "develop"
shortcut = "f"

[[branch_types]]
name = "hotfix"
prefix = "hotfix/"
base = "main"
shortcut = "h"
"#;
        let config = Config::parse(content).unwrap();
        assert_eq!(config.editor, Some("vim".to_string()));
        assert_eq!(config.branch_types.len(), 2);
        assert_eq!(config.branch_types[0].name, "feature");
        assert_eq!(config.branch_types[0].prefix, "feature/");
        assert_eq!(config.branch_types[0].base, "develop");
        assert_eq!(config.branch_types[0].shortcut, 'f');
        assert_eq!(config.branch_types[1].name, "hotfix");
        assert_eq!(config.branch_types[1].base, "main");
        assert_eq!(config.branch_types[1].shortcut, 'h');
    }

    #[test]
    fn test_find_branch_type_by_shortcut() {
        let config = Config {
            branch_types: default_branch_types(),
            ..Default::default()
        };

        let feature = config.find_branch_type_by_shortcut('f');
        assert!(feature.is_some());
        assert_eq!(feature.unwrap().name, "feature");

        let hotfix = config.find_branch_type_by_shortcut('h');
        assert!(hotfix.is_some());
        assert_eq!(hotfix.unwrap().base, "main");

        let unknown = config.find_branch_type_by_shortcut('x');
        assert!(unknown.is_none());
    }
}
