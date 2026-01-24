use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::types::{Worktree, WorktreeStatus};

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
                let status = if is_bare {
                    WorktreeStatus::Clean
                } else {
                    get_status(&path).unwrap_or(WorktreeStatus::Clean)
                };
                let last_commit_time = if is_bare {
                    None
                } else {
                    get_last_commit_time(&path).ok()
                };
                worktrees.push(Worktree {
                    path,
                    branch: current_branch.take(),
                    is_bare,
                    status,
                    last_commit_time,
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
        let status = if is_bare {
            WorktreeStatus::Clean
        } else {
            get_status(&path).unwrap_or(WorktreeStatus::Clean)
        };
        let last_commit_time = if is_bare {
            None
        } else {
            get_last_commit_time(&path).ok()
        };
        worktrees.push(Worktree {
            path,
            branch: current_branch,
            is_bare,
            status,
            last_commit_time,
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

pub fn fetch_all(bare_repo_path: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["-C", &bare_repo_path.to_string_lossy(), "fetch", "--all"])
        .output()
        .context("Failed to fetch")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to fetch: {}", stderr.trim());
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
