use super::*;

fn prune_test_worktree(branch: Option<&str>, status: types::WorktreeStatus) -> types::Worktree {
    types::Worktree {
        path: PathBuf::from(format!("/worktrees/{}", branch.unwrap_or("detached"))),
        branch: branch.map(str::to_string),
        is_bare: false,
        status,
        last_commit_time: None,
        ahead_behind: None,
        github_pr_status: None,
    }
}

fn prune_test_decision<'a>(
    current_path: Option<&'a Path>,
    head_branch: Option<&'a str>,
    pr_status: Option<types::GithubPrStatus>,
) -> PruneWorktreeDecision<'a> {
    PruneWorktreeDecision {
        current_path,
        head_branch,
        pr_status,
    }
}

#[test]
fn prune_action_removes_clean_worktrees_when_pr_is_merged_or_closed() {
    let merged = prune_test_worktree(Some("feature/merged"), types::WorktreeStatus::Clean);
    let closed = prune_test_worktree(Some("feature/closed"), types::WorktreeStatus::Clean);

    assert_eq!(
        prune_worktree_action(
            &merged,
            prune_test_decision(None, Some("main"), Some(types::GithubPrStatus::Merged))
        ),
        PruneWorktreeAction::WouldRemove
    );
    assert_eq!(
        prune_worktree_action(
            &closed,
            prune_test_decision(None, Some("main"), Some(types::GithubPrStatus::Closed))
        ),
        PruneWorktreeAction::WouldRemove
    );
}

#[test]
fn prune_action_keeps_worktrees_without_completed_pr_status() {
    let missing = prune_test_worktree(Some("feature/missing"), types::WorktreeStatus::Clean);
    let open = prune_test_worktree(Some("feature/open"), types::WorktreeStatus::Clean);
    let draft = prune_test_worktree(Some("feature/draft"), types::WorktreeStatus::Clean);

    assert_eq!(
        prune_worktree_action(&missing, prune_test_decision(None, Some("main"), None)),
        PruneWorktreeAction::Kept("pr-missing".to_string())
    );
    assert_eq!(
        prune_worktree_action(
            &open,
            prune_test_decision(None, Some("main"), Some(types::GithubPrStatus::Open))
        ),
        PruneWorktreeAction::Kept("pr-open".to_string())
    );
    assert_eq!(
        prune_worktree_action(
            &draft,
            prune_test_decision(None, Some("main"), Some(types::GithubPrStatus::Draft))
        ),
        PruneWorktreeAction::Kept("pr-draft".to_string())
    );
}

#[test]
fn prune_action_keeps_dirty_current_head_and_detached_worktrees() {
    let dirty = prune_test_worktree(Some("feature/dirty"), types::WorktreeStatus::Unstaged);
    let current = prune_test_worktree(Some("feature/current"), types::WorktreeStatus::Clean);
    let head = prune_test_worktree(Some("main"), types::WorktreeStatus::Clean);
    let detached = prune_test_worktree(None, types::WorktreeStatus::Clean);

    assert_eq!(
        prune_worktree_action(
            &dirty,
            prune_test_decision(None, Some("main"), Some(types::GithubPrStatus::Closed))
        ),
        PruneWorktreeAction::Kept("status-unstaged".to_string())
    );
    assert_eq!(
        prune_worktree_action(
            &current,
            prune_test_decision(
                Some(current.path.as_path()),
                Some("main"),
                Some(types::GithubPrStatus::Closed)
            )
        ),
        PruneWorktreeAction::Kept("current".to_string())
    );
    assert_eq!(
        prune_worktree_action(
            &head,
            prune_test_decision(None, Some("main"), Some(types::GithubPrStatus::Merged))
        ),
        PruneWorktreeAction::Kept("head".to_string())
    );
    assert_eq!(
        prune_worktree_action(
            &detached,
            prune_test_decision(None, Some("main"), Some(types::GithubPrStatus::Merged))
        ),
        PruneWorktreeAction::Kept("detached".to_string())
    );
}
