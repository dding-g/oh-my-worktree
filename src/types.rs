use std::path::PathBuf;

use crate::config::BranchType;

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

/// Which base to use when creating a new worktree
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum BaseSource {
    #[default]
    Local,      // Use local branch as base
    Remote,     // Use remote branch as base (origin/<branch>)
}

/// State for the add worktree modal - holds all configuration
#[derive(Debug, Clone)]
pub struct AddWorktreeState {
    pub branch_type: Option<BranchType>,  // None means custom (manual base selection)
    pub base_branch: String,              // The base branch to create from
    pub base_source: BaseSource,          // Local or remote
    pub branch_name: String,              // Full branch name (e.g., "feature/foo")
    #[allow(dead_code)]
    pub is_fetching: bool,                // Currently fetching (for async UI)
}

impl Default for AddWorktreeState {
    fn default() -> Self {
        Self {
            branch_type: None,
            base_branch: "main".to_string(),
            base_source: BaseSource::Local,
            branch_name: String::new(),
            is_fetching: false,
        }
    }
}

impl AddWorktreeState {
    pub fn with_branch_type(branch_type: BranchType) -> Self {
        let base = branch_type.base.clone();
        let prefix = branch_type.prefix.clone();
        Self {
            branch_type: Some(branch_type),
            base_branch: base,
            base_source: BaseSource::Local,
            branch_name: prefix, // Start with prefix
            is_fetching: false,
        }
    }

    pub fn custom(base_branch: String) -> Self {
        Self {
            branch_type: None,
            base_branch,
            base_source: BaseSource::Local,
            branch_name: String::new(),
            is_fetching: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppState {
    List,
    #[allow(dead_code)]
    AddModal,  // Kept for backwards compatibility
    /// Type selection screen for add worktree
    AddTypeSelect,
    /// Branch input screen with base branch comparison
    AddBranchInput,
    ConfirmDelete { delete_branch: bool },
    ConfigModal {
        selected_index: usize,  // 0-4 (editor, terminal, copy_files, post_add_script, branch_types)
        editing: bool,          // inline editing mode
    },
    /// Branch types editing within config modal
    BranchTypesModal {
        selected_index: usize,  // Which branch type is selected
        editing_field: Option<usize>,  // Which field is being edited (0=base, 1=shortcut)
    },
    /// Initial setup modal for first-time configuration
    #[allow(dead_code)]
    SetupModal {
        selected_index: usize,  // Which branch type is selected
    },
    HelpModal,
    Fetching,
    Adding,
    Deleting,
    Pulling,
    Pushing,
    Merging,
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
