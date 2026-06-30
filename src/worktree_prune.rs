use anyhow::{Context, Result};
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::{git, types};

#[derive(Debug, PartialEq, Eq)]
enum PruneWorktreeAction {
    Removed,
    WouldRemove,
    Kept(String),
}

pub(crate) struct PruneWorktreeLog {
    branch: Option<String>,
    path: PathBuf,
    action: PruneWorktreeAction,
}

struct PruneWorktreeDecision<'a> {
    current_path: Option<&'a Path>,
    head_branch: Option<&'a str>,
    pr_status: Option<types::GithubPrStatus>,
}

pub(crate) fn prune_completed_pr_worktrees(
    repo_path: &Path,
    launch_path: &Path,
    dry_run: bool,
) -> Result<Vec<PruneWorktreeLog>> {
    let worktrees = git::list_worktrees(repo_path)?;
    let current_path = current_worktree_path(&worktrees, launch_path);
    let head_branch = git::get_default_branch(repo_path).ok();
    let pr_status_by_path = prune_pr_statuses(repo_path, &worktrees);
    let mut logs = Vec::new();

    for worktree in worktrees {
        let action = prune_worktree_action(
            &worktree,
            PruneWorktreeDecision {
                current_path: current_path.as_deref(),
                head_branch: head_branch.as_deref(),
                pr_status: pr_status_by_path
                    .get(&worktree.path)
                    .and_then(|status| *status),
            },
        );
        let action = match action {
            PruneWorktreeAction::WouldRemove if dry_run => {
                if confirm_dry_run_prune(&worktree)? {
                    PruneWorktreeAction::WouldRemove
                } else {
                    PruneWorktreeAction::Kept("declined".to_string())
                }
            }
            action => action,
        };

        logs.push(PruneWorktreeLog {
            branch: worktree.branch.clone(),
            path: worktree.path,
            action,
        });
    }

    if !dry_run {
        remove_prune_candidates(repo_path, &mut logs)?;
    }

    Ok(logs)
}

fn prune_pr_statuses(
    repo_path: &Path,
    worktrees: &[types::Worktree],
) -> HashMap<PathBuf, Option<types::GithubPrStatus>> {
    let targets: Vec<(PathBuf, String)> = worktrees
        .iter()
        .filter_map(|worktree| {
            if worktree.is_bare {
                return None;
            }
            Some((worktree.path.clone(), worktree.branch.clone()?))
        })
        .collect();

    git::github_pr_statuses_for_worktrees(repo_path, &targets)
        .into_iter()
        .collect()
}

fn prune_worktree_action(
    worktree: &types::Worktree,
    decision: PruneWorktreeDecision<'_>,
) -> PruneWorktreeAction {
    if worktree.is_bare {
        return PruneWorktreeAction::Kept("bare".to_string());
    }
    if decision
        .current_path
        .map(|path| paths_refer_to_same_location(path, &worktree.path))
        .unwrap_or(false)
    {
        return PruneWorktreeAction::Kept("current".to_string());
    }
    if worktree.status != types::WorktreeStatus::Clean {
        return PruneWorktreeAction::Kept(format!("status-{}", worktree.status.label()));
    }
    let Some(branch) = worktree.branch.as_deref() else {
        return PruneWorktreeAction::Kept("detached".to_string());
    };
    if decision
        .head_branch
        .map(|head| head == branch)
        .unwrap_or(false)
    {
        return PruneWorktreeAction::Kept("head".to_string());
    }

    match decision.pr_status {
        Some(types::GithubPrStatus::Merged | types::GithubPrStatus::Closed) => {
            PruneWorktreeAction::WouldRemove
        }
        Some(status) => PruneWorktreeAction::Kept(format!("pr-{}", status.label())),
        None => PruneWorktreeAction::Kept("pr-missing".to_string()),
    }
}

fn remove_prune_candidates(repo_path: &Path, logs: &mut [PruneWorktreeLog]) -> Result<()> {
    let candidates: Vec<(usize, PathBuf)> = logs
        .iter()
        .enumerate()
        .filter_map(|(index, log)| {
            if matches!(log.action, PruneWorktreeAction::WouldRemove) {
                Some((index, log.path.clone()))
            } else {
                None
            }
        })
        .collect();

    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = candidates
            .into_iter()
            .map(|(index, path)| {
                scope.spawn(move || {
                    let result = git::remove_completed_pr_worktree(repo_path, &path);
                    (index, path, result)
                })
            })
            .collect();
        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            let result = handle
                .join()
                .map_err(|_| anyhow::anyhow!("worktree prune worker panicked"))?;
            results.push(result);
        }
        Ok::<Vec<(usize, PathBuf, Result<()>)>, anyhow::Error>(results)
    })?;

    for (index, path, result) in results {
        result.with_context(|| format!("Failed to prune worktree {}", path.display()))?;
        logs[index].action = PruneWorktreeAction::Removed;
    }

    Ok(())
}

fn confirm_dry_run_prune(worktree: &types::Worktree) -> Result<bool> {
    let branch = worktree.branch.as_deref().unwrap_or("-");
    eprint!(
        "dry-run delete candidate\t{}\t{} [y/N] ",
        plain_field(branch),
        plain_field(&worktree.path.display().to_string())
    );
    io::stderr()
        .flush()
        .context("Failed to flush dry-run prune prompt")?;

    let mut answer = String::new();
    io::stdin()
        .read_line(&mut answer)
        .context("Failed to read dry-run prune answer")?;
    Ok(matches!(answer.trim(), "y" | "Y" | "yes" | "YES"))
}

pub(crate) fn print_prune_output(metadata_output: &str, logs: &[PruneWorktreeLog]) {
    let removed_or_selected = logs
        .iter()
        .filter(|log| {
            matches!(
                log.action,
                PruneWorktreeAction::Removed | PruneWorktreeAction::WouldRemove
            )
        })
        .count();

    if metadata_output.is_empty() && removed_or_selected == 0 {
        println!("pruned\t0");
    }

    for log in logs {
        let branch = plain_field(log.branch.as_deref().unwrap_or("-"));
        let path = plain_field(&log.path.display().to_string());
        match &log.action {
            PruneWorktreeAction::Removed => {
                println!("pruned\tworktree\t{}\t{}", branch, path);
            }
            PruneWorktreeAction::WouldRemove => {
                println!("pruned\tlog\twould-remove\t{}\t{}\tselected", branch, path);
            }
            PruneWorktreeAction::Kept(reason) => {
                println!(
                    "pruned\tlog\tkept\t{}\t{}\t{}",
                    branch,
                    path,
                    plain_field(reason)
                );
            }
        }
    }

    for line in metadata_output.lines() {
        println!("pruned\tmetadata\t{}", plain_field(line));
    }
}

fn current_worktree_path(worktrees: &[types::Worktree], launch_path: &Path) -> Option<PathBuf> {
    let canonical_launch = launch_path.canonicalize().ok()?;
    worktrees
        .iter()
        .find(|worktree| {
            !worktree.is_bare
                && worktree
                    .path
                    .canonicalize()
                    .ok()
                    .map(|path| canonical_launch.starts_with(path))
                    .unwrap_or(false)
        })
        .map(|worktree| worktree.path.clone())
}

fn paths_refer_to_same_location(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }

    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn plain_field(value: &str) -> String {
    value.replace(['\t', '\n', '\r'], " ")
}

#[cfg(test)]
mod tests;
