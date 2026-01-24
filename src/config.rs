use anyhow::Result;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct Config {
    pub editor: Option<String>,
    pub terminal: Option<String>,
    pub copy_files: Vec<String>,        // Files to copy when adding worktree
    pub post_add_script: Option<String>, // Script to run after adding worktree
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&config_path)?;
        Self::parse(&content)
    }

    pub fn config_path() -> PathBuf {
        let config_dir = dirs_config_dir().join("owt");
        config_dir.join("config.toml")
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path();
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

        fs::write(&config_path, content)?;
        Ok(())
    }

    fn parse(content: &str) -> Result<Self> {
        let mut config = Config::default();

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');

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

        Ok(config)
    }

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
}
