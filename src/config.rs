use anyhow::Result;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct Config {
    pub editor: Option<String>,
    pub terminal: Option<String>,
    pub worktree_root: Option<String>,
    pub copy_files: Vec<String>, // Files to copy when adding worktree
    pub post_add_script: Option<String>, // Script to run after adding worktree
    pub run_post_add_script_in_tmux: bool,
    run_post_add_script_in_tmux_configured: bool,
}

impl Config {
    /// Load config with project-level override support
    /// Priority: project (.owt/config.toml) > global (~/.config/owt/config.toml)
    #[allow(dead_code)]
    pub fn load() -> Result<Self> {
        Self::load_with_project(None)
    }

    /// Load config with optional project root for project-level config
    pub fn load_with_project(project_root_path: Option<&std::path::Path>) -> Result<Self> {
        // Start with global config
        let global_path = Self::global_config_path();
        let mut config = if global_path.exists() {
            let content = fs::read_to_string(&global_path)?;
            Self::parse(&content)?
        } else {
            Self::default()
        };

        // Override with project-level config if exists
        if let Some(project_root) = project_root_path {
            let project_path = Self::project_config_path(project_root);
            if project_path.exists() {
                let content = fs::read_to_string(&project_path)?;
                let project_config = Self::parse(&content)?;
                config.merge_from_project(project_config);
            }
        }

        Ok(config)
    }

    /// Merge project config into self (project overrides global safe values).
    /// Script auto-run must stay globally trusted and cannot be enabled by a repo.
    fn merge_from_project(&mut self, other: Config) {
        if other.editor.is_some() {
            self.editor = other.editor;
        }
        if other.terminal.is_some() {
            self.terminal = other.terminal;
        }
        if other.worktree_root.is_some() {
            self.worktree_root = other.worktree_root;
        }
        if !other.copy_files.is_empty() {
            self.copy_files = other.copy_files;
        }
        if other.post_add_script.is_some() {
            self.post_add_script = other.post_add_script;
        }
    }

    /// Global config path: ~/.config/owt/config.toml
    pub fn global_config_path() -> PathBuf {
        let config_dir = dirs_config_dir().join("owt");
        config_dir.join("config.toml")
    }

    /// Project config path: .owt/config.toml under the project root
    pub fn project_config_path(project_root_path: &std::path::Path) -> PathBuf {
        Self::owt_dir(project_root_path).join("config.toml")
    }

    /// Legacy: for backwards compatibility
    #[allow(dead_code)]
    pub fn config_path() -> PathBuf {
        Self::global_config_path()
    }

    /// Save config to global config file
    #[allow(dead_code)]
    pub fn save(&self) -> Result<()> {
        self.save_to(&Self::global_config_path())
    }

    /// Save config to project-level config file
    pub fn save_to_project(&self, project_root_path: &std::path::Path) -> Result<()> {
        self.save_to_project_path(&Self::project_config_path(project_root_path))
    }

    /// Save config to specified path
    #[allow(dead_code)]
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
        if let Some(ref worktree_root) = self.worktree_root {
            content.push_str(&format!("worktree_root = \"{}\"\n", worktree_root));
        }
        if !self.copy_files.is_empty() {
            let files = self
                .copy_files
                .iter()
                .map(|f| format!("\"{}\"", f))
                .collect::<Vec<_>>()
                .join(", ");
            content.push_str(&format!("copy_files = [{}]\n", files));
        }
        if let Some(ref script) = self.post_add_script {
            content.push_str(&format!("post_add_script = \"{}\"\n", script));
        }
        content.push_str(&format!(
            "run_post_add_script_in_tmux = {}\n",
            self.run_post_add_script_in_tmux
        ));

        fs::write(config_path, content)?;
        Ok(())
    }

    fn save_to_project_path(&self, config_path: &PathBuf) -> Result<()> {
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
        if let Some(ref worktree_root) = self.worktree_root {
            content.push_str(&format!("worktree_root = \"{}\"\n", worktree_root));
        }
        if !self.copy_files.is_empty() {
            let files = self
                .copy_files
                .iter()
                .map(|f| format!("\"{}\"", f))
                .collect::<Vec<_>>()
                .join(", ");
            content.push_str(&format!("copy_files = [{}]\n", files));
        }
        if let Some(ref script) = self.post_add_script {
            content.push_str(&format!("post_add_script = \"{}\"\n", script));
        }

        fs::write(config_path, content)?;
        Ok(())
    }

    fn parse(content: &str) -> Result<Self> {
        let mut config = Config::default();

        for line in content.lines() {
            let line = line.trim();

            // Skip comments, empty lines, and section headers
            if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');

                match key {
                    "editor" => config.editor = Some(value.to_string()),
                    "terminal" => config.terminal = Some(value.to_string()),
                    "worktree_root" => config.worktree_root = Some(value.to_string()),
                    "post_add_script" => config.post_add_script = Some(value.to_string()),
                    "run_post_add_script_in_tmux" => {
                        config.run_post_add_script_in_tmux = parse_bool(value);
                        config.run_post_add_script_in_tmux_configured = true;
                    }
                    "copy_files" => {
                        let files: Vec<String> = value
                            .trim_matches('[')
                            .trim_matches(']')
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
}

fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "1" | "yes" | "on"
    )
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

    pub fn default_worktree_root() -> PathBuf {
        home_dir()
            .map(|home| home.join(".owt").join("worktree"))
            .unwrap_or_else(|| PathBuf::from(".owt").join("worktree"))
    }

    pub fn resolved_worktree_root(&self) -> PathBuf {
        self.worktree_root
            .as_deref()
            .map(expand_home_path)
            .unwrap_or_else(Self::default_worktree_root)
    }

    /// Get the .owt directory path under the project root
    pub fn owt_dir(project_root_path: &std::path::Path) -> PathBuf {
        project_root_path.join(".owt")
    }

    /// Get the post-add script path
    pub fn post_add_script_path(project_root_path: &std::path::Path) -> PathBuf {
        Self::owt_dir(project_root_path).join("post-add.sh")
    }

    pub fn resolved_post_add_script_path(&self, project_root_path: &std::path::Path) -> PathBuf {
        self.post_add_script
            .as_deref()
            .map(expand_home_path)
            .map(|path| {
                if path.is_absolute() {
                    path
                } else {
                    project_root_path.join(path)
                }
            })
            .unwrap_or_else(|| Self::post_add_script_path(project_root_path))
    }
}

fn expand_home_path(path: &str) -> PathBuf {
    if path == "~" {
        return home_dir().unwrap_or_else(|| PathBuf::from(path));
    }

    if let Some(rest) = path.strip_prefix("~/") {
        return home_dir()
            .map(|home| home.join(rest))
            .unwrap_or_else(|| PathBuf::from(path));
    }

    PathBuf::from(path)
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
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
    use std::sync::{Mutex, OnceLock};

    fn test_env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn acquire_test_env_lock() -> std::sync::MutexGuard<'static, ()> {
        test_env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    struct EnvVarGuard {
        key: &'static str,
        original: Option<std::ffi::OsString>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
            let original = std::env::var_os(key);
            std::env::set_var(key, value);
            Self { key, original }
        }

        fn unset(key: &'static str) -> Self {
            let original = std::env::var_os(key);
            std::env::remove_var(key);
            Self { key, original }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(ref value) = self.original {
                std::env::set_var(self.key, value);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    #[test]
    fn test_parse_empty() {
        let config = Config::parse("").unwrap();
        assert!(config.editor.is_none());
        assert!(config.terminal.is_none());
        assert!(!config.run_post_add_script_in_tmux);
    }

    #[test]
    fn test_parse_values() {
        let content = r#"
[core]
editor = "code"
terminal = "Ghostty"
worktree_root = "~/.owt/worktree"
run_post_add_script_in_tmux = true
"#;
        let config = Config::parse(content).unwrap();
        assert_eq!(config.editor, Some("code".to_string()));
        assert_eq!(config.terminal, Some("Ghostty".to_string()));
        assert_eq!(config.worktree_root, Some("~/.owt/worktree".to_string()));
        assert!(config.run_post_add_script_in_tmux);
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
    fn test_save_writes_tmux_post_add_flag() {
        let dir = std::env::temp_dir().join(format!(
            "owt_config_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        let config = Config {
            editor: Some("code".to_string()),
            run_post_add_script_in_tmux: true,
            ..Default::default()
        };

        config.save_to(&path).unwrap();
        let saved = fs::read_to_string(&path).unwrap();

        assert!(saved.contains("editor = \"code\""));
        assert!(saved.contains("run_post_add_script_in_tmux = true"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_project_config_does_not_enable_tmux_post_add() {
        let _env_lock = acquire_test_env_lock();
        let dir = std::env::temp_dir().join(format!(
            "owt_project_config_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let home_dir = dir.join("home");
        let xdg_config_home = dir.join("xdg-config");
        let project_dir = dir.join("project");
        fs::create_dir_all(&home_dir).unwrap();
        fs::create_dir_all(&xdg_config_home).unwrap();
        fs::create_dir_all(project_dir.join(".owt")).unwrap();
        fs::write(
            project_dir.join(".owt").join("config.toml"),
            "run_post_add_script_in_tmux = true\n",
        )
        .unwrap();

        let _home_guard = EnvVarGuard::set("HOME", &home_dir);
        let _xdg_guard = EnvVarGuard::unset("XDG_CONFIG_HOME");

        let config = Config::load_with_project(Some(&project_dir)).unwrap();

        assert!(!config.run_post_add_script_in_tmux);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_global_config_post_add_script_keeps_tmux_enabled() {
        let _env_lock = acquire_test_env_lock();
        let dir = std::env::temp_dir().join(format!(
            "owt_global_config_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let home_dir = dir.join("home");
        let xdg_config_home = dir.join("xdg-config");
        fs::create_dir_all(&home_dir).unwrap();
        fs::create_dir_all(xdg_config_home.join("owt")).unwrap();
        let project_dir = dir.join("project");
        fs::create_dir_all(project_dir.join(".owt")).unwrap();
        fs::write(
            xdg_config_home.join("owt").join("config.toml"),
            "run_post_add_script_in_tmux = true\n",
        )
        .unwrap();

        let _home_guard = EnvVarGuard::set("HOME", &home_dir);
        let _xdg_guard = EnvVarGuard::set("XDG_CONFIG_HOME", &xdg_config_home);

        let config = Config::load_with_project(Some(&project_dir)).unwrap();

        assert!(config.run_post_add_script_in_tmux);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_project_config_applies_safe_overrides() {
        let _env_lock = acquire_test_env_lock();
        let dir = std::env::temp_dir().join(format!(
            "owt_project_safe_override_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let home_dir = dir.join("home");
        let xdg_config_home = dir.join("xdg-config");
        fs::create_dir_all(&home_dir).unwrap();
        fs::create_dir_all(&xdg_config_home).unwrap();
        let project_dir = dir.join("project");
        fs::create_dir_all(project_dir.join(".owt")).unwrap();
        fs::write(
            project_dir.join(".owt").join("config.toml"),
            r#"
editor = "code"
terminal = "Ghostty"
worktree_root = "/tmp/owt-worktrees"
copy_files = [".env", ".envrc"]
post_add_script = "setup.sh"
run_post_add_script_in_tmux = true
"#,
        )
        .unwrap();

        let _home_guard = EnvVarGuard::set("HOME", &home_dir);
        let _xdg_guard = EnvVarGuard::unset("XDG_CONFIG_HOME");

        let config = Config::load_with_project(Some(&project_dir)).unwrap();

        assert_eq!(config.editor, Some("code".to_string()));
        assert_eq!(config.terminal, Some("Ghostty".to_string()));
        assert_eq!(config.worktree_root, Some("/tmp/owt-worktrees".to_string()));
        assert_eq!(config.copy_files, vec![".env", ".envrc"]);
        assert_eq!(config.post_add_script, Some("setup.sh".to_string()));
        assert!(!config.run_post_add_script_in_tmux);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_project_post_add_script_overrides_global() {
        let _env_lock = acquire_test_env_lock();
        let dir = std::env::temp_dir().join(format!(
            "owt_project_post_add_override_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let home_dir = dir.join("home");
        let xdg_config_home = dir.join("xdg-config");
        let project_dir = dir.join("project");
        fs::create_dir_all(&home_dir).unwrap();
        fs::create_dir_all(&xdg_config_home.join("owt")).unwrap();
        fs::create_dir_all(project_dir.join(".owt")).unwrap();
        fs::write(
            xdg_config_home.join("owt").join("config.toml"),
            r#"
post_add_script = "/global/post-add.sh"
run_post_add_script_in_tmux = true
"#,
        )
        .unwrap();
        fs::write(
            project_dir.join(".owt").join("config.toml"),
            r#"
post_add_script = "scripts/project-post-add.sh"
"#,
        )
        .unwrap();

        let _home_guard = EnvVarGuard::set("HOME", &home_dir);
        let _xdg_guard = EnvVarGuard::set("XDG_CONFIG_HOME", &xdg_config_home);

        let config = Config::load_with_project(Some(&project_dir)).unwrap();

        assert_eq!(
            config.post_add_script,
            Some("scripts/project-post-add.sh".to_string())
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_save_to_project_omits_tmux_post_add_flag() {
        let dir = std::env::temp_dir().join(format!(
            "owt_project_save_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let config = Config {
            editor: Some("code".to_string()),
            run_post_add_script_in_tmux: true,
            ..Default::default()
        };

        config.save_to_project(&dir).unwrap();
        let saved = fs::read_to_string(dir.join(".owt").join("config.toml")).unwrap();

        assert!(saved.contains("editor = \"code\""));
        assert!(!saved.contains("run_post_add_script_in_tmux"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_parse_ignores_unknown_sections() {
        // Old config files with [[branch_types]] should not break parsing
        let content = r#"
editor = "vim"

[[branch_types]]
name = "feature"
prefix = "feature/"
base = "develop"
shortcut = "f"
"#;
        let config = Config::parse(content).unwrap();
        assert_eq!(config.editor, Some("vim".to_string()));
    }

    #[test]
    fn test_resolved_worktree_root_uses_configured_absolute_path() {
        let config = Config {
            worktree_root: Some("/tmp/custom-worktrees".to_string()),
            ..Default::default()
        };

        assert_eq!(
            config.resolved_worktree_root(),
            PathBuf::from("/tmp/custom-worktrees")
        );
    }

    #[test]
    fn test_resolved_worktree_root_expands_home_path() {
        let _env_lock = acquire_test_env_lock();
        let dir = std::env::temp_dir().join(format!(
            "owt_worktree_home_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let home_dir = dir.join("home");
        fs::create_dir_all(&home_dir).unwrap();
        let _home_guard = EnvVarGuard::set("HOME", &home_dir);
        let _xdg_guard = EnvVarGuard::unset("XDG_CONFIG_HOME");
        let config = Config {
            worktree_root: Some("~/custom-worktrees".to_string()),
            ..Default::default()
        };

        assert_eq!(
            config.resolved_worktree_root(),
            home_dir.join("custom-worktrees")
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_post_add_script_relative_path_resolves_against_project_root() {
        let config = Config {
            post_add_script: Some("scripts/post-add.sh".to_string()),
            ..Default::default()
        };
        let project_root = PathBuf::from("/tmp/project-root");

        assert_eq!(
            config.resolved_post_add_script_path(&project_root),
            project_root.join("scripts/post-add.sh")
        );
    }

    #[test]
    fn test_post_add_script_absolute_path_uses_as_is() {
        let config = Config {
            post_add_script: Some("/opt/setup/post-add.sh".to_string()),
            ..Default::default()
        };
        let project_root = PathBuf::from("/tmp/project-root");

        assert_eq!(
            config.resolved_post_add_script_path(&project_root),
            PathBuf::from("/opt/setup/post-add.sh")
        );
    }

    #[test]
    fn test_post_add_script_defaults_to_project_helper_when_unset() {
        let config = Config::default();
        let project_root = PathBuf::from("/tmp/project-root");

        assert_eq!(
            config.resolved_post_add_script_path(&project_root),
            Config::post_add_script_path(&project_root)
        );
    }

    #[test]
    fn test_editor_and_terminal_precedence() {
        let _env_lock = acquire_test_env_lock();

        {
            let _editor_guard = EnvVarGuard::set("EDITOR", "nano");
            let _terminal_guard = EnvVarGuard::set("TERMINAL", "Ghostty");

            let default_config = Config::default();
            assert_eq!(default_config.get_editor(), "nano");
            assert_eq!(default_config.get_terminal(), Some("Ghostty".to_string()));

            let configured = Config {
                editor: Some("code".to_string()),
                terminal: Some("WezTerm".to_string()),
                ..Default::default()
            };
            assert_eq!(configured.get_editor(), "code");
            assert_eq!(configured.get_terminal(), Some("WezTerm".to_string()));
        }

        {
            let _editor_guard = EnvVarGuard::unset("EDITOR");
            let _terminal_guard = EnvVarGuard::unset("TERMINAL");

            assert_eq!(Config::default().get_editor(), "vim");
            assert_eq!(Config::default().get_terminal(), None);
        }
    }
}
