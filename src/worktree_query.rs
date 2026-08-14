//! Shared worktree search semantics for the CLI and TUI surfaces.

use crate::types::Worktree;

/// Returns whether a worktree matches a user-entered query.
///
/// The matcher intentionally uses the same display labels that the CLI record
/// and TUI table expose, so users can move between surfaces without learning
/// different search fields. An empty query matches every worktree.
pub(crate) fn matches(worktree: &Worktree, query: &str) -> bool {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return true;
    }

    [
        worktree.path.display().to_string(),
        worktree.display_name(),
        worktree.branch_display(),
        worktree.status.label().to_string(),
        worktree.github_pr_display().to_string(),
    ]
    .iter()
    .any(|value| value.to_lowercase().contains(&needle))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::types::{GithubPrStatus, Worktree, WorktreeStatus};

    use super::matches;

    fn worktree() -> Worktree {
        Worktree {
            path: PathBuf::from("/repo/feature/auth-login"),
            branch: Some("feature/auth-login".to_string()),
            is_bare: false,
            status: WorktreeStatus::Unstaged,
            last_commit_time: None,
            last_commit_timestamp: None,
            ahead_behind: None,
            github_pr_status: Some(GithubPrStatus::Closed),
        }
    }

    #[test]
    fn matches_every_cli_search_field_case_insensitively() {
        let worktree = worktree();

        for query in [
            "/REPO/FEATURE",
            "auth-login",
            "FEATURE/AUTH",
            "UNSTAGED",
            "CLOSED",
        ] {
            assert!(matches(&worktree, query), "expected {query:?} to match");
        }
    }

    #[test]
    fn empty_query_matches_every_worktree() {
        assert!(matches(&worktree(), "   "));
    }

    #[test]
    fn does_not_match_unrelated_query() {
        assert!(!matches(&worktree(), "merged"));
    }
}
