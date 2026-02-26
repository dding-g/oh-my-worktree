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

#[derive(Debug, Clone, Default)]
pub struct AheadBehind {
    pub ahead: u32,
    pub behind: u32,
}

impl AheadBehind {
    pub fn display(&self) -> Option<String> {
        if self.ahead == 0 && self.behind == 0 {
            None
        } else if self.ahead > 0 && self.behind > 0 {
            Some(format!("↑{}↓{}", self.ahead, self.behind))
        } else if self.ahead > 0 {
            Some(format!("↑{}", self.ahead))
        } else {
            Some(format!("↓{}", self.behind))
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
    pub ahead_behind: Option<AheadBehind>,
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
    ConfirmDelete { delete_branch: bool, force: bool },
    ConfigModal {
        selected_index: usize,  // 0-3 (editor, terminal, copy_files, post_add_script)
        editing: bool,
    },
    HelpModal,
    /// Branch selection for merge
    MergeBranchSelect {
        branches: Vec<String>,
        selected: usize,
    },
}

/// Exit reason when quitting the app
#[derive(Debug, Clone)]
pub enum ExitAction {
    Quit,
    ChangeDirectory(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortMode {
    #[default]
    Name,
    Recent,
    Status,
}

impl SortMode {
    pub fn next(self) -> Self {
        match self {
            SortMode::Name => SortMode::Recent,
            SortMode::Recent => SortMode::Status,
            SortMode::Status => SortMode::Name,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            SortMode::Name => "name",
            SortMode::Recent => "recent",
            SortMode::Status => "status",
        }
    }
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

/// Status of background post-add script execution.
#[derive(Debug, Clone)]
pub enum ScriptStatus {
    Idle,
    Running { worktree_name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpKind {
    Fetch,
    Pull,
    Push,
    Add,
    Delete,
    Merge,
}

pub struct OpResult {
    pub kind: OpKind,
    pub success: bool,
    pub message: String,
    pub cmd_detail: String,
    pub worktree_path: PathBuf,
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub struct ActiveOp {
    pub kind: OpKind,
    pub worktree_path: PathBuf,
    pub display_name: String,
}
