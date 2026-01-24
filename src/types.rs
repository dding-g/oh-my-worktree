use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreeStatus {
    Clean,
    Staged,
    Unstaged,
    Conflict,
    Mixed,
}

impl WorktreeStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            WorktreeStatus::Clean => "✓",
            WorktreeStatus::Staged => "+",
            WorktreeStatus::Unstaged => "~",
            WorktreeStatus::Conflict => "!",
            WorktreeStatus::Mixed => "*",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            WorktreeStatus::Clean => "clean",
            WorktreeStatus::Staged => "staged",
            WorktreeStatus::Unstaged => "unstaged",
            WorktreeStatus::Conflict => "conflict",
            WorktreeStatus::Mixed => "mixed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Worktree {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub is_bare: bool,
    pub status: WorktreeStatus,
    pub last_commit_time: Option<String>,
}

impl Worktree {
    pub fn display_name(&self) -> String {
        if self.is_bare {
            "(bare)".to_string()
        } else {
            self.path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| self.path.to_string_lossy().to_string())
        }
    }

    pub fn branch_display(&self) -> String {
        self.branch.clone().unwrap_or_else(|| "-".to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppState {
    List,
    AddModal,
    ConfirmDelete { delete_branch: bool },
    ConfigModal {
        selected_index: usize,  // 0-3 (editor, terminal, copy_files, post_add_script)
        editing: bool,          // inline editing mode
    },
    HelpModal,
    Fetching,
    Adding,
    Deleting,
}

/// Exit reason when quitting the app
#[derive(Debug, Clone)]
pub enum ExitAction {
    Quit,
    ChangeDirectory(PathBuf),
}

#[derive(Debug, Clone)]
pub struct AppMessage {
    pub text: String,
    pub is_error: bool,
}

impl AppMessage {
    pub fn info(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_error: false,
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_error: true,
        }
    }
}
