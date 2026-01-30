use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::types::{AheadBehind, Worktree, WorktreeStatus};

/// Check for .bare folder pattern (common worktree layout)
/// Returns the path to .bare if found
pub fn find_bare_in_parent(path: &Path) -> Option<PathBuf> {
    let bare_path = path.join(".bare");
    if bare_path.exists() && bare_path.is_dir() {
        if is_bare_repo(&bare_path).unwrap_or(false) {
            return Some(bare_path);
        }
    }
    None
}

pub fn is_bare_repo(path: &Path) -> Result<bool> {
    let output = Command::new("git")
        .args(["-C", &path.to_string_lossy(), "rev-parse", "--is-bare-repository"])
        .output()
        .context("Failed to execute git command")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim() == "true")
}

pub fn is_git_repo(path: &Path) -> bool {
    Command::new("git")
        .args(["-C", &path.to_string_lossy(), "rev-parse", "--git-dir"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get the common git directory (bare repo root for worktrees)
pub fn get_git_common_dir(path: &Path) -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["-C", &path.to_string_lossy(), "rev-parse", "--git-common-dir"])
        .output()
        .context("Failed to get git common directory")?;

    if !output.status.success() {
        anyhow::bail!("Not a git repository");
    }

    let git_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let git_path = PathBuf::from(&git_dir);

    if git_path.is_absolute() {
        Ok(git_path)
    } else {
        Ok(path.join(git_path).canonicalize()?)
    }
}

pub fn list_worktrees(bare_repo_path: &Path) -> Result<Vec<Worktree>> {
    let output = Command::new("git")
        .args(["-C", &bare_repo_path.to_string_lossy(), "worktree", "list", "--porcelain"])
        .output()
        .context("Failed to list worktrees")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to list worktrees: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_worktree_list(&stdout, bare_repo_path)
}

fn parse_worktree_list(output: &str, _bare_repo_path: &Path) -> Result<Vec<Worktree>> {
    let mut worktrees = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;
    let mut is_bare = false;

    for line in output.lines() {
        if line.starts_with("worktree ") {
            if let Some(path) = current_path.take() {
                let (status, last_commit_time, ahead_behind) = if is_bare {
                    (WorktreeStatus::Clean, None, None)
                } else {
                    (
                        get_status(&path).unwrap_or(WorktreeStatus::Clean),
                        get_last_commit_time(&path).ok(),
                        get_ahead_behind(&path),
                    )
                };
                worktrees.push(Worktree {
                    path,
                    branch: current_branch.take(),
                    is_bare,
                    status,
                    last_commit_time,
                    ahead_behind,
                });
            }
            current_path = Some(PathBuf::from(line.strip_prefix("worktree ").unwrap()));
            is_bare = false;
        } else if line.starts_with("branch ") {
            let branch = line.strip_prefix("branch refs/heads/").unwrap_or(
                line.strip_prefix("branch ").unwrap_or("")
            );
            current_branch = Some(branch.to_string());
        } else if line == "bare" {
            is_bare = true;
        } else if line.starts_with("HEAD ") {
            // Detached HEAD, no branch
        }
    }

    // Handle the last worktree
    if let Some(path) = current_path {
        let (status, last_commit_time, ahead_behind) = if is_bare {
            (WorktreeStatus::Clean, None, None)
        } else {
            (
                get_status(&path).unwrap_or(WorktreeStatus::Clean),
                get_last_commit_time(&path).ok(),
                get_ahead_behind(&path),
            )
        };
        worktrees.push(Worktree {
            path,
            branch: current_branch,
            is_bare,
            status,
            last_commit_time,
            ahead_behind,
        });
    }

    Ok(worktrees)
}

pub fn get_status(path: &Path) -> Result<WorktreeStatus> {
    let output = Command::new("git")
        .args(["-C", &path.to_string_lossy(), "status", "--porcelain"])
        .output()
        .context("Failed to get status")?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    if stdout.trim().is_empty() {
        return Ok(WorktreeStatus::Clean);
    }

    let mut has_staged = false;
    let mut has_unstaged = false;
    let mut has_conflict = false;

    for line in stdout.lines() {
        if line.len() < 2 {
            continue;
        }
        let index = line.chars().next().unwrap_or(' ');
        let worktree = line.chars().nth(1).unwrap_or(' ');

        // Check for conflicts (UU, AA, DD, etc.)
        if matches!((index, worktree), ('U', _) | (_, 'U') | ('A', 'A') | ('D', 'D')) {
            has_conflict = true;
        }

        // Staged changes (index has non-space, non-? character)
        if index != ' ' && index != '?' {
            has_staged = true;
        }

        // Unstaged changes (worktree has non-space character)
        if worktree != ' ' && worktree != '?' {
            has_unstaged = true;
        }
    }

    if has_conflict {
        Ok(WorktreeStatus::Conflict)
    } else if has_staged && has_unstaged {
        Ok(WorktreeStatus::Mixed)
    } else if has_staged {
        Ok(WorktreeStatus::Staged)
    } else if has_unstaged {
        Ok(WorktreeStatus::Unstaged)
    } else {
        Ok(WorktreeStatus::Clean)
    }
}

pub fn add_worktree(bare_repo_path: &Path, branch: &str, worktree_path: &Path, base_branch: Option<&str>) -> Result<()> {
    let mut args = vec![
        "-C".to_string(),
        bare_repo_path.to_string_lossy().to_string(),
        "worktree".to_string(),
        "add".to_string(),
    ];

    // Check if branch exists
    let branch_exists = Command::new("git")
        .args(["-C", &bare_repo_path.to_string_lossy(), "show-ref", "--verify", "--quiet", &format!("refs/heads/{}", branch)])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    let remote_branch_exists = Command::new("git")
        .args(["-C", &bare_repo_path.to_string_lossy(), "show-ref", "--verify", "--quiet", &format!("refs/remotes/origin/{}", branch)])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if branch_exists {
        // Branch exists locally, just add worktree
        args.push(worktree_path.to_string_lossy().to_string());
        args.push(branch.to_string());
    } else if remote_branch_exists {
        // Remote branch exists, track it
        args.push("--track".to_string());
        args.push("-b".to_string());
        args.push(branch.to_string());
        args.push(worktree_path.to_string_lossy().to_string());
        args.push(format!("origin/{}", branch));
    } else {
        // Create new branch
        args.push("-b".to_string());
        args.push(branch.to_string());
        args.push(worktree_path.to_string_lossy().to_string());
        if let Some(base) = base_branch {
            args.push(base.to_string());
        }
    }

    let output = Command::new("git")
        .args(&args)
        .output()
        .context("Failed to add worktree")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to add worktree: {}", stderr.trim());
    }

    Ok(())
}

pub fn remove_worktree(bare_repo_path: &Path, worktree_path: &Path, force: bool) -> Result<()> {
    let bare_repo_str = bare_repo_path.to_string_lossy();
    let worktree_str = worktree_path.to_string_lossy();

    let mut args = vec!["-C", &*bare_repo_str, "worktree", "remove"];

    if force {
        args.push("--force");
    }

    args.push(&*worktree_str);

    let output = Command::new("git")
        .args(&args)
        .output()
        .context("Failed to remove worktree")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to remove worktree: {}", stderr.trim());
    }

    Ok(())
}

pub fn delete_branch(bare_repo_path: &Path, branch: &str, force: bool) -> Result<()> {
    let flag = if force { "-D" } else { "-d" };

    let output = Command::new("git")
        .args(["-C", &bare_repo_path.to_string_lossy(), "branch", flag, branch])
        .output()
        .context("Failed to delete branch")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to delete branch: {}", stderr.trim());
    }

    Ok(())
}

/// Fetch only the remote tracking branch for a specific worktree
pub fn fetch_worktree(worktree_path: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["-C", &worktree_path.to_string_lossy(), "fetch", "origin"])
        .output()
        .context("Failed to fetch")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to fetch: {}", stderr.trim());
    }

    Ok(())
}

pub fn get_last_commit_time(path: &Path) -> Result<String> {
    let output = Command::new("git")
        .args([
            "-C",
            &path.to_string_lossy(),
            "log",
            "-1",
            "--format=%ar",
        ])
        .output()
        .context("Failed to get last commit time")?;

    if !output.status.success() {
        anyhow::bail!("Failed to get last commit time");
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn get_ahead_behind(path: &Path) -> Option<AheadBehind> {
    // Get the upstream tracking branch
    let output = Command::new("git")
        .args([
            "-C",
            &path.to_string_lossy(),
            "rev-list",
            "--left-right",
            "--count",
            "@{upstream}...HEAD",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout.trim().split('\t').collect();

    if parts.len() == 2 {
        let behind = parts[0].parse().unwrap_or(0);
        let ahead = parts[1].parse().unwrap_or(0);
        Some(AheadBehind { ahead, behind })
    } else {
        None
    }
}

pub fn clone_bare(url: &str, path: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["clone", "--bare", url, &path.to_string_lossy()])
        .output()
        .context("Failed to clone repository")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to clone: {}", stderr.trim());
    }

    Ok(())
}

pub fn get_default_branch(bare_repo_path: &Path) -> Result<String> {
    // Try to get the default branch from HEAD
    let output = Command::new("git")
        .args([
            "-C",
            &bare_repo_path.to_string_lossy(),
            "symbolic-ref",
            "HEAD",
        ])
        .output()
        .context("Failed to get default branch")?;

    if output.status.success() {
        let head = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // refs/heads/main -> main
        if let Some(branch) = head.strip_prefix("refs/heads/") {
            return Ok(branch.to_string());
        }
    }

    // Fallback: try common branch names
    for branch in &["main", "master"] {
        let check = Command::new("git")
            .args([
                "-C",
                &bare_repo_path.to_string_lossy(),
                "show-ref",
                "--verify",
                "--quiet",
                &format!("refs/heads/{}", branch),
            ])
            .status();

        if check.map(|s| s.success()).unwrap_or(false) {
            return Ok(branch.to_string());
        }
    }

    // Default to main
    Ok("main".to_string())
}

/// Commit info for display
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub hash: String,      // Short hash (e.g., abc1234)
    pub message: String,   // First line of commit message
    pub time_ago: String,  // Relative time (e.g., "2 days ago")
}

/// Comparison between local and remote branch
#[derive(Debug, Clone, Default)]
pub struct BranchComparison {
    pub local: Option<CommitInfo>,
    pub remote: Option<CommitInfo>,
    pub behind_count: u32,
    pub ahead_count: u32,
}

/// Get commit info for a branch (local or remote)
pub fn get_branch_commit_info(bare_repo_path: &Path, branch: &str) -> Result<CommitInfo> {
    let output = Command::new("git")
        .args([
            "-C",
            &bare_repo_path.to_string_lossy(),
            "log",
            "-1",
            "--format=%h|%s|%ar",
            branch,
        ])
        .output()
        .context("Failed to get commit info")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to get commit info: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout.trim().splitn(3, '|').collect();

    if parts.len() < 3 {
        anyhow::bail!("Invalid commit info format");
    }

    Ok(CommitInfo {
        hash: parts[0].to_string(),
        message: parts[1].to_string(),
        time_ago: parts[2].to_string(),
    })
}

/// Compare local branch with its remote counterpart
pub fn compare_local_remote(bare_repo_path: &Path, branch: &str) -> Result<BranchComparison> {
    let mut comparison = BranchComparison::default();

    // Get local branch info
    let local_ref = format!("refs/heads/{}", branch);
    if branch_exists(bare_repo_path, &local_ref) {
        comparison.local = get_branch_commit_info(bare_repo_path, branch).ok();
    }

    // Get remote branch info
    let remote_ref = format!("origin/{}", branch);
    let remote_full_ref = format!("refs/remotes/origin/{}", branch);
    if branch_exists(bare_repo_path, &remote_full_ref) {
        comparison.remote = get_branch_commit_info(bare_repo_path, &remote_ref).ok();
    }

    // Get ahead/behind counts if both exist
    if comparison.local.is_some() && comparison.remote.is_some() {
        let output = Command::new("git")
            .args([
                "-C",
                &bare_repo_path.to_string_lossy(),
                "rev-list",
                "--left-right",
                "--count",
                &format!("{}...{}", branch, remote_ref),
            ])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let parts: Vec<&str> = stdout.trim().split('\t').collect();
                if parts.len() == 2 {
                    comparison.ahead_count = parts[0].parse().unwrap_or(0);
                    comparison.behind_count = parts[1].parse().unwrap_or(0);
                }
            }
        }
    } else if comparison.remote.is_some() && comparison.local.is_none() {
        // If only remote exists, count commits from common ancestor
        // For now, just indicate that remote exists
    }

    Ok(comparison)
}

/// Check if a branch reference exists
fn branch_exists(bare_repo_path: &Path, ref_name: &str) -> bool {
    Command::new("git")
        .args([
            "-C",
            &bare_repo_path.to_string_lossy(),
            "show-ref",
            "--verify",
            "--quiet",
            ref_name,
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Fetch a specific branch from origin
pub fn fetch_branch(bare_repo_path: &Path, branch: &str) -> Result<()> {
    let output = Command::new("git")
        .args([
            "-C",
            &bare_repo_path.to_string_lossy(),
            "fetch",
            "origin",
            &format!("{}:{}", branch, format!("refs/remotes/origin/{}", branch)),
        ])
        .output()
        .context("Failed to fetch branch")?;

    // Git fetch may return non-zero even on partial success, so check stderr
    if !output.status.success() {
        // Try simpler fetch
        let output2 = Command::new("git")
            .args([
                "-C",
                &bare_repo_path.to_string_lossy(),
                "fetch",
                "origin",
                branch,
            ])
            .output()
            .context("Failed to fetch branch")?;

        if !output2.status.success() {
            let stderr = String::from_utf8_lossy(&output2.stderr);
            anyhow::bail!("Failed to fetch branch: {}", stderr.trim());
        }
    }

    Ok(())
}

/// List all branches (local and remote)
#[allow(dead_code)]
pub fn list_branches(bare_repo_path: &Path) -> Result<Vec<String>> {
    let mut branches = Vec::new();

    // Get local branches
    let output = Command::new("git")
        .args([
            "-C",
            &bare_repo_path.to_string_lossy(),
            "for-each-ref",
            "--format=%(refname:short)",
            "refs/heads/",
        ])
        .output()
        .context("Failed to list local branches")?;

    if output.status.success() {
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let branch = line.trim();
            if !branch.is_empty() {
                branches.push(branch.to_string());
            }
        }
    }

    // Get remote branches (without origin/ prefix)
    let output = Command::new("git")
        .args([
            "-C",
            &bare_repo_path.to_string_lossy(),
            "for-each-ref",
            "--format=%(refname:short)",
            "refs/remotes/origin/",
        ])
        .output()
        .context("Failed to list remote branches")?;

    if output.status.success() {
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let branch = line.trim();
            if !branch.is_empty() && branch != "origin/HEAD" {
                // Remove origin/ prefix
                let short_name = branch.strip_prefix("origin/").unwrap_or(branch);
                if !branches.contains(&short_name.to_string()) {
                    branches.push(short_name.to_string());
                }
            }
        }
    }

    // Sort and deduplicate
    branches.sort();
    branches.dedup();

    Ok(branches)
}

/// Check if a local branch exists
#[allow(dead_code)]
pub fn local_branch_exists(bare_repo_path: &Path, branch: &str) -> bool {
    branch_exists(bare_repo_path, &format!("refs/heads/{}", branch))
}

/// Check if a remote branch exists
#[allow(dead_code)]
pub fn remote_branch_exists(bare_repo_path: &Path, branch: &str) -> bool {
    branch_exists(bare_repo_path, &format!("refs/remotes/origin/{}", branch))
}

/// Pull changes from remote for a worktree
pub fn pull_worktree(worktree_path: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["-C", &worktree_path.to_string_lossy(), "pull"])
        .output()
        .context("Failed to pull")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to pull: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().to_string())
}

/// Push changes to remote for a worktree
pub fn push_worktree(worktree_path: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["-C", &worktree_path.to_string_lossy(), "push"])
        .output()
        .context("Failed to push")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to push: {}", stderr.trim());
    }

    // Git push often outputs to stderr even on success
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let message = if stdout.trim().is_empty() {
        stderr.trim().to_string()
    } else {
        stdout.trim().to_string()
    };
    Ok(message)
}

/// Merge upstream branch into a worktree
/// Finds the configured upstream and merges it
pub fn merge_upstream(worktree_path: &Path) -> Result<String> {
    // First, get the upstream branch
    let upstream_output = Command::new("git")
        .args(["-C", &worktree_path.to_string_lossy(), "rev-parse", "--abbrev-ref", "@{upstream}"])
        .output()
        .context("Failed to get upstream")?;

    if !upstream_output.status.success() {
        anyhow::bail!("No upstream branch configured");
    }

    let upstream = String::from_utf8_lossy(&upstream_output.stdout).trim().to_string();

    // Merge the upstream
    let output = Command::new("git")
        .args(["-C", &worktree_path.to_string_lossy(), "merge", &upstream])
        .output()
        .context("Failed to merge upstream")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to merge: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(format!("Merged {} - {}", upstream, stdout.trim()))
}

/// Merge a specific branch into a worktree
pub fn merge_branch(worktree_path: &Path, source_branch: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["-C", &worktree_path.to_string_lossy(), "merge", source_branch])
        .output()
        .context("Failed to merge")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to merge: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().to_string())
}

/// Force update a local branch ref to match its remote counterpart.
/// Skips if the branch is currently checked out in any worktree.
pub fn force_update_local_branch(bare_repo_path: &Path, branch: &str) -> Result<()> {
    // Check if the remote ref exists
    let remote_ref = format!("refs/remotes/origin/{}", branch);
    if !branch_exists(bare_repo_path, &remote_ref) {
        anyhow::bail!("Remote branch origin/{} not found", branch);
    }

    // Check if the branch is checked out in any worktree
    let output = Command::new("git")
        .args([
            "-C",
            &bare_repo_path.to_string_lossy(),
            "worktree",
            "list",
            "--porcelain",
        ])
        .output()
        .context("Failed to list worktrees")?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let checked_out_branch = format!("branch refs/heads/{}", branch);
        for line in stdout.lines() {
            if line == checked_out_branch {
                // Branch is checked out in a worktree, skip update
                return Ok(());
            }
        }
    }

    // Update local ref to match remote
    let local_ref = format!("refs/heads/{}", branch);
    let output = Command::new("git")
        .args([
            "-C",
            &bare_repo_path.to_string_lossy(),
            "update-ref",
            &local_ref,
            &remote_ref,
        ])
        .output()
        .context("Failed to update local branch ref")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to update local branch: {}", stderr.trim());
    }

    Ok(())
}

/// List local branches for merge selection
pub fn list_local_branches(bare_repo_path: &Path) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args([
            "-C",
            &bare_repo_path.to_string_lossy(),
            "for-each-ref",
            "--format=%(refname:short)",
            "refs/heads/",
        ])
        .output()
        .context("Failed to list branches")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to list branches: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let branches: Vec<String> = stdout
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok(branches)
}
