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
        .args([
            "-C",
            &path.to_string_lossy(),
            "rev-parse",
            "--is-bare-repository",
        ])
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
        .args([
            "-C",
            &path.to_string_lossy(),
            "rev-parse",
            "--git-common-dir",
        ])
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
        .args([
            "-C",
            &bare_repo_path.to_string_lossy(),
            "worktree",
            "list",
            "--porcelain",
        ])
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
            let branch = line
                .strip_prefix("branch refs/heads/")
                .unwrap_or(line.strip_prefix("branch ").unwrap_or(""));
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
    ensure_worktree_is_usable(path)?;

    let output = Command::new("git")
        .args(["-C", &path.to_string_lossy(), "status", "--porcelain"])
        .output()
        .context("Failed to get status")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to get status: {}", stderr.trim());
    }

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
        if matches!(
            (index, worktree),
            ('U', _) | (_, 'U') | ('A', 'A') | ('D', 'D')
        ) {
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

pub fn add_worktree(
    bare_repo_path: &Path,
    branch: &str,
    worktree_path: &Path,
    base_branch: Option<&str>,
) -> Result<()> {
    let mut args = vec![
        "-C".to_string(),
        bare_repo_path.to_string_lossy().to_string(),
        "worktree".to_string(),
        "add".to_string(),
    ];

    let branch_exists = ref_exists(bare_repo_path, &format!("refs/heads/{}", branch));
    let remote_branch_exists =
        ref_exists(bare_repo_path, &format!("refs/remotes/origin/{}", branch));
    let base_ref = resolve_base_ref(bare_repo_path, base_branch);

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
        if let Some(base) = base_ref {
            args.push(base);
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

    ensure_worktree_is_usable(worktree_path)?;

    Ok(())
}

fn ref_exists(bare_repo_path: &Path, reference: &str) -> bool {
    Command::new("git")
        .args([
            "-C",
            &bare_repo_path.to_string_lossy(),
            "show-ref",
            "--verify",
            "--quiet",
            reference,
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn has_origin_remote(bare_repo_path: &Path) -> bool {
    Command::new("git")
        .args([
            "-C",
            &bare_repo_path.to_string_lossy(),
            "remote",
            "get-url",
            "origin",
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn remote_branch_exists_on_origin(bare_repo_path: &Path, branch: &str) -> Result<bool> {
    if !has_origin_remote(bare_repo_path) {
        return Ok(false);
    }

    let output = Command::new("git")
        .args([
            "-C",
            &bare_repo_path.to_string_lossy(),
            "ls-remote",
            "--exit-code",
            "--heads",
            "origin",
            branch,
        ])
        .output()
        .context("Failed to inspect remote branch")?;

    if output.status.success() {
        Ok(true)
    } else if output.status.code() == Some(2) {
        Ok(false)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to inspect origin/{}: {}", branch, stderr.trim());
    }
}

pub fn fetch_remote_branch(bare_repo_path: &Path, branch: &str) -> Result<bool> {
    if !remote_branch_exists_on_origin(bare_repo_path, branch)? {
        return Ok(false);
    }

    let refspec = format!("refs/heads/{}:refs/remotes/origin/{}", branch, branch);
    let output = Command::new("git")
        .args([
            "-C",
            &bare_repo_path.to_string_lossy(),
            "fetch",
            "origin",
            &refspec,
        ])
        .output()
        .context("Failed to fetch remote branch")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to fetch origin/{}: {}", branch, stderr.trim());
    }

    Ok(true)
}

fn resolve_base_ref(bare_repo_path: &Path, base_branch: Option<&str>) -> Option<String> {
    base_branch.map(|base| {
        let remote_ref = format!("refs/remotes/origin/{}", base);
        if ref_exists(bare_repo_path, &remote_ref) {
            format!("origin/{}", base)
        } else {
            base.to_string()
        }
    })
}

fn ensure_worktree_is_usable(worktree_path: &Path) -> Result<()> {
    let bare_check = Command::new("git")
        .args([
            "-C",
            &worktree_path.to_string_lossy(),
            "rev-parse",
            "--is-bare-repository",
        ])
        .output()
        .context("Failed to verify new worktree state")?;

    if !bare_check.status.success() {
        let stderr = String::from_utf8_lossy(&bare_check.stderr);
        anyhow::bail!("Failed to verify new worktree state: {}", stderr.trim());
    }

    let is_bare = String::from_utf8_lossy(&bare_check.stdout).trim() == "true";
    if !is_bare {
        return Ok(());
    }

    let git_dir_output = Command::new("git")
        .args([
            "-C",
            &worktree_path.to_string_lossy(),
            "rev-parse",
            "--git-dir",
        ])
        .output()
        .context("Failed to resolve worktree git dir")?;

    if !git_dir_output.status.success() {
        let stderr = String::from_utf8_lossy(&git_dir_output.stderr);
        anyhow::bail!("Failed to resolve worktree git dir: {}", stderr.trim());
    }

    let git_dir_raw = String::from_utf8_lossy(&git_dir_output.stdout)
        .trim()
        .to_string();
    let git_dir_path = PathBuf::from(&git_dir_raw);
    let resolved_git_dir = if git_dir_path.is_absolute() {
        git_dir_path
    } else {
        worktree_path
            .join(git_dir_path)
            .canonicalize()
            .with_context(|| {
                format!(
                    "Failed to canonicalize worktree git dir at {}",
                    worktree_path.display()
                )
            })?
    };

    let fix_output = Command::new("git")
        .arg(format!("--git-dir={}", resolved_git_dir.display()))
        .arg(format!("--work-tree={}", worktree_path.display()))
        .args(["config", "--worktree", "core.bare", "false"])
        .output()
        .context("Failed to write worktree-specific config")?;

    if !fix_output.status.success() {
        let stderr = String::from_utf8_lossy(&fix_output.stderr);
        anyhow::bail!(
            "Failed to write worktree-specific config: {}",
            stderr.trim()
        );
    }

    let verify_output = Command::new("git")
        .args([
            "-C",
            &worktree_path.to_string_lossy(),
            "rev-parse",
            "--is-bare-repository",
        ])
        .output()
        .context("Failed to verify repaired worktree state")?;

    if !verify_output.status.success() {
        let stderr = String::from_utf8_lossy(&verify_output.stderr);
        anyhow::bail!(
            "Failed to verify repaired worktree state: {}",
            stderr.trim()
        );
    }

    if String::from_utf8_lossy(&verify_output.stdout).trim() != "false" {
        anyhow::bail!(
            "Worktree remains bare after repair attempt at {}",
            worktree_path.display()
        );
    }

    Ok(())
}

/// Build command detail string for verbose mode (mirrors add_worktree logic)
pub fn build_add_worktree_command_detail(
    bare_repo_path: &Path,
    branch: &str,
    worktree_path: &Path,
    base_branch: Option<&str>,
) -> String {
    let branch_exists = ref_exists(bare_repo_path, &format!("refs/heads/{}", branch));
    let remote_branch_exists =
        ref_exists(bare_repo_path, &format!("refs/remotes/origin/{}", branch));
    let base_ref = resolve_base_ref(bare_repo_path, base_branch);

    let bare = bare_repo_path.display();
    let wt = worktree_path.display();

    if branch_exists {
        format!("git -C {} worktree add {} {}", bare, wt, branch)
    } else if remote_branch_exists {
        format!(
            "git -C {} worktree add --track -b {} {} origin/{}",
            bare, branch, wt, branch
        )
    } else {
        match base_ref {
            Some(base) => format!("git -C {} worktree add -b {} {} {}", bare, branch, wt, base),
            None => format!("git -C {} worktree add -b {} {}", bare, branch, wt),
        }
    }
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

pub fn prune_worktrees(bare_repo_path: &Path) -> Result<String> {
    let output = Command::new("git")
        .args([
            "-C",
            &bare_repo_path.to_string_lossy(),
            "worktree",
            "prune",
            "-v",
        ])
        .output()
        .context("Failed to prune worktrees")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to prune worktrees: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().to_string())
}

pub fn delete_branch(bare_repo_path: &Path, branch: &str, force: bool) -> Result<()> {
    let flag = if force { "-D" } else { "-d" };

    let output = Command::new("git")
        .args([
            "-C",
            &bare_repo_path.to_string_lossy(),
            "branch",
            flag,
            branch,
        ])
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
        .args(["-C", &path.to_string_lossy(), "log", "-1", "--format=%ar"])
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
        .args([
            "-C",
            &worktree_path.to_string_lossy(),
            "rev-parse",
            "--abbrev-ref",
            "@{upstream}",
        ])
        .output()
        .context("Failed to get upstream")?;

    if !upstream_output.status.success() {
        anyhow::bail!("No upstream branch configured");
    }

    let upstream = String::from_utf8_lossy(&upstream_output.stdout)
        .trim()
        .to_string();

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
        .args([
            "-C",
            &worktree_path.to_string_lossy(),
            "merge",
            source_branch,
        ])
        .output()
        .context("Failed to merge")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to merge: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().to_string())
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

#[cfg(test)]
mod tests {
    use super::{add_worktree, fetch_remote_branch, list_worktrees};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let id = std::process::id();
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("owt_git_unit_{}_{}_{}", name, id, ts));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn git_cmd() -> Command {
        let mut cmd = Command::new("git");
        cmd.env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_COMMON_DIR");
        cmd
    }

    fn create_test_bare_repo(path: &PathBuf) -> String {
        let temp = path.parent().unwrap().join("temp_init");
        fs::create_dir_all(&temp).unwrap();

        let init_output = git_cmd()
            .current_dir(&temp)
            .args(["init"])
            .output()
            .unwrap();
        assert!(
            init_output.status.success(),
            "git init failed: {}",
            String::from_utf8_lossy(&init_output.stderr)
        );

        let config_email = git_cmd()
            .current_dir(&temp)
            .args(["config", "user.email", "test@test.com"])
            .output()
            .unwrap();
        assert!(
            config_email.status.success(),
            "git config email failed: {}",
            String::from_utf8_lossy(&config_email.stderr)
        );

        let config_name = git_cmd()
            .current_dir(&temp)
            .args(["config", "user.name", "Test"])
            .output()
            .unwrap();
        assert!(
            config_name.status.success(),
            "git config name failed: {}",
            String::from_utf8_lossy(&config_name.stderr)
        );

        fs::write(temp.join("README.md"), "# Test").unwrap();

        let add_output = git_cmd()
            .current_dir(&temp)
            .args(["add", "."])
            .output()
            .unwrap();
        assert!(
            add_output.status.success(),
            "git add failed: {}",
            String::from_utf8_lossy(&add_output.stderr)
        );

        let commit_output = git_cmd()
            .current_dir(&temp)
            .args(["commit", "-m", "Initial commit"])
            .output()
            .unwrap();
        assert!(
            commit_output.status.success(),
            "git commit failed: {}",
            String::from_utf8_lossy(&commit_output.stderr)
        );

        let branch_output = git_cmd()
            .current_dir(&temp)
            .args(["branch", "--show-current"])
            .output()
            .unwrap();
        assert!(
            branch_output.status.success(),
            "git branch --show-current failed: {}",
            String::from_utf8_lossy(&branch_output.stderr)
        );
        let branch = String::from_utf8_lossy(&branch_output.stdout)
            .trim()
            .to_string();
        assert!(!branch.is_empty(), "current branch should not be empty");

        let clone_output = git_cmd()
            .args([
                "clone",
                "--bare",
                &temp.to_string_lossy(),
                &path.to_string_lossy(),
            ])
            .output()
            .unwrap();
        assert!(
            clone_output.status.success(),
            "git clone --bare failed: {}",
            String::from_utf8_lossy(&clone_output.stderr)
        );

        let _ = fs::remove_dir_all(&temp);
        branch
    }

    fn git_in(path: &Path, args: &[&str]) -> Output {
        git_cmd().current_dir(path).args(args).output().unwrap()
    }

    fn assert_git_success(output: &Output, context: &str) {
        assert!(
            output.status.success(),
            "{}: {}",
            context,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn write_and_commit(path: &Path, file: &str, content: &str, message: &str) -> String {
        fs::write(path.join(file), content).unwrap();
        assert_git_success(&git_in(path, &["add", "."]), "git add failed");
        assert_git_success(
            &git_in(path, &["commit", "-m", message]),
            "git commit failed",
        );
        let output = git_in(path, &["rev-parse", "HEAD"]);
        assert_git_success(&output, "git rev-parse HEAD failed");
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn create_source_and_bare_repo(base: &Path) -> (PathBuf, PathBuf) {
        let source_path = base.join("source");
        let bare_path = base.join("test.bare");

        fs::create_dir_all(&source_path).unwrap();
        assert_git_success(
            &git_in(&source_path, &["init", "-b", "main"]),
            "git init failed",
        );
        assert_git_success(
            &git_in(&source_path, &["config", "user.email", "test@test.com"]),
            "git config user.email failed",
        );
        assert_git_success(
            &git_in(&source_path, &["config", "user.name", "Test"]),
            "git config user.name failed",
        );

        write_and_commit(&source_path, "README.md", "# Test\n", "initial");
        assert_git_success(
            &git_in(&source_path, &["checkout", "-b", "staging"]),
            "git checkout staging failed",
        );
        write_and_commit(&source_path, "staging.txt", "v1\n", "staging v1");
        assert_git_success(
            &git_in(&source_path, &["checkout", "main"]),
            "git checkout main failed",
        );

        let clone_output = git_cmd()
            .args([
                "clone",
                "--bare",
                &source_path.to_string_lossy(),
                &bare_path.to_string_lossy(),
            ])
            .output()
            .unwrap();
        assert_git_success(&clone_output, "git clone --bare failed");

        (source_path, bare_path)
    }

    fn git_output(args: &[&str]) -> Output {
        git_cmd().args(args).output().unwrap()
    }

    fn enable_worktree_config(bare_path: &Path) {
        let enable_ext = git_output(&[
            "-C",
            &bare_path.to_string_lossy(),
            "config",
            "extensions.worktreeConfig",
            "true",
        ]);
        assert!(
            enable_ext.status.success(),
            "failed to enable extensions.worktreeConfig: {}",
            String::from_utf8_lossy(&enable_ext.stderr)
        );
    }

    fn create_local_branch(bare_path: &Path, branch: &str, base_branch: &str) {
        let output = git_output(&[
            "-C",
            &bare_path.to_string_lossy(),
            "branch",
            branch,
            base_branch,
        ]);
        assert!(
            output.status.success(),
            "failed to create branch {}: {}",
            branch,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn resolve_git_path(worktree_path: &Path, git_path: &str) -> PathBuf {
        let output = git_output(&[
            "-C",
            &worktree_path.to_string_lossy(),
            "rev-parse",
            "--git-path",
            git_path,
        ]);
        assert!(
            output.status.success(),
            "failed to resolve git path {}: {}",
            git_path,
            String::from_utf8_lossy(&output.stderr)
        );

        PathBuf::from(String::from_utf8_lossy(&output.stdout).trim())
    }

    fn resolve_git_dir(worktree_path: &Path) -> PathBuf {
        let output = git_output(&[
            "-C",
            &worktree_path.to_string_lossy(),
            "rev-parse",
            "--git-dir",
        ]);
        assert!(
            output.status.success(),
            "failed to resolve git dir: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let git_dir = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
        if git_dir.is_absolute() {
            git_dir
        } else {
            worktree_path.join(git_dir).canonicalize().unwrap()
        }
    }

    fn assert_worktree_usable(worktree_path: &Path) {
        let bare_output = git_output(&[
            "-C",
            &worktree_path.to_string_lossy(),
            "rev-parse",
            "--is-bare-repository",
        ]);
        assert!(
            bare_output.status.success(),
            "failed to query bare state: {}",
            String::from_utf8_lossy(&bare_output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&bare_output.stdout).trim(),
            "false",
            "worktree should not be bare"
        );

        let status_output = git_output(&[
            "-C",
            &worktree_path.to_string_lossy(),
            "status",
            "--porcelain",
        ]);
        assert!(
            status_output.status.success(),
            "worktree should be usable, but git status failed: {}",
            String::from_utf8_lossy(&status_output.stderr)
        );
    }

    fn assert_worktree_config_marks_non_bare(worktree_path: &Path) {
        let config_worktree_path = resolve_git_path(worktree_path, "config.worktree");
        assert!(
            config_worktree_path.exists(),
            "config.worktree should exist at {}",
            config_worktree_path.display()
        );

        let content = fs::read_to_string(&config_worktree_path).unwrap();
        assert!(
            content.contains("bare = false"),
            "config.worktree should mark worktree non-bare, got: {}",
            content
        );
    }

    fn canonicalize_existing(path: &Path) -> PathBuf {
        path.canonicalize().unwrap()
    }

    #[test]
    fn add_worktree_creates_usable_worktree_with_worktree_config_extension_enabled() {
        let base = temp_dir("add_worktree_worktree_config");
        let bare_path = base.join("test.bare");
        let branch = create_test_bare_repo(&bare_path);
        let worktree_path = base.join("main");

        enable_worktree_config(&bare_path);

        let add_result = add_worktree(&bare_path, &branch, &worktree_path, None);
        assert!(
            add_result.is_ok(),
            "add_worktree should succeed: {:?}",
            add_result
        );

        assert_worktree_usable(&worktree_path);
        assert_worktree_config_marks_non_bare(&worktree_path);

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn add_worktree_repairs_nested_feature_branch_when_worktree_config_enabled() {
        let base = temp_dir("nested_feature_worktree_config");
        let bare_path = base.join("test.bare");
        let default_branch = create_test_bare_repo(&bare_path);
        let branch = "feature/syn-6485-qcp";
        let worktree_path = base.join("feature").join("syn-6485-qcp");

        enable_worktree_config(&bare_path);

        let add_result = add_worktree(&bare_path, branch, &worktree_path, Some(&default_branch));
        assert!(
            add_result.is_ok(),
            "add_worktree should succeed: {:?}",
            add_result
        );

        assert_worktree_usable(&worktree_path);
        assert_worktree_config_marks_non_bare(&worktree_path);

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn add_worktree_repairs_nested_hotfix_branch_from_existing_ref() {
        let base = temp_dir("nested_hotfix_existing_ref");
        let bare_path = base.join("test.bare");
        let default_branch = create_test_bare_repo(&bare_path);
        let branch = "hotfix/syn-6485-qcp";
        let worktree_path = base.join("hotfix").join("syn-6485-qcp");

        enable_worktree_config(&bare_path);
        create_local_branch(&bare_path, branch, &default_branch);

        let add_result = add_worktree(&bare_path, branch, &worktree_path, Some(&default_branch));
        assert!(
            add_result.is_ok(),
            "add_worktree should succeed: {:?}",
            add_result
        );

        assert_worktree_usable(&worktree_path);
        assert_worktree_config_marks_non_bare(&worktree_path);

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn add_worktree_keeps_feature_and_hotfix_same_leaf_usable() {
        let base = temp_dir("same_leaf_nested_worktrees");
        let bare_path = base.join("test.bare");
        let default_branch = create_test_bare_repo(&bare_path);
        let feature_branch = "feature/foo";
        let hotfix_branch = "hotfix/foo";
        let feature_path = base.join("feature").join("foo");
        let hotfix_path = base.join("hotfix").join("foo");

        enable_worktree_config(&bare_path);

        let feature_result = add_worktree(
            &bare_path,
            feature_branch,
            &feature_path,
            Some(&default_branch),
        );
        assert!(
            feature_result.is_ok(),
            "feature add_worktree should succeed: {:?}",
            feature_result
        );

        let hotfix_result = add_worktree(
            &bare_path,
            hotfix_branch,
            &hotfix_path,
            Some(&default_branch),
        );
        assert!(
            hotfix_result.is_ok(),
            "hotfix add_worktree should succeed: {:?}",
            hotfix_result
        );

        assert_worktree_usable(&feature_path);
        assert_worktree_usable(&hotfix_path);

        let feature_git_dir = resolve_git_dir(&feature_path);
        let hotfix_git_dir = resolve_git_dir(&hotfix_path);
        assert_ne!(
            feature_git_dir, hotfix_git_dir,
            "feature and hotfix worktrees should use distinct git dirs"
        );

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn ensure_worktree_repairs_legacy_nested_worktree_after_worktree_config_enabled() {
        let base = temp_dir("legacy_nested_worktree_repair");
        let bare_path = base.join("test.bare");
        let default_branch = create_test_bare_repo(&bare_path);
        let branch = "feature/legacy";
        let worktree_path = base.join("feature").join("legacy");

        let add_result = add_worktree(&bare_path, branch, &worktree_path, Some(&default_branch));
        assert!(
            add_result.is_ok(),
            "add_worktree should succeed before enabling extension: {:?}",
            add_result
        );

        enable_worktree_config(&bare_path);

        let broken_status = git_output(&[
            "-C",
            &worktree_path.to_string_lossy(),
            "status",
            "--porcelain",
        ]);
        assert!(
            !broken_status.status.success(),
            "legacy nested worktree should fail before self-heal once extension is enabled"
        );

        let worktrees =
            list_worktrees(&bare_path).expect("list_worktrees should succeed after self-heal");
        let expected_path = canonicalize_existing(&worktree_path);
        assert!(
            worktrees
                .iter()
                .any(|wt| canonicalize_existing(&wt.path) == expected_path),
            "list_worktrees should include the repaired legacy worktree"
        );

        assert_worktree_usable(&worktree_path);
        assert_worktree_config_marks_non_bare(&worktree_path);

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn add_worktree_uses_latest_origin_base_branch_when_creating_new_branch() {
        let base = temp_dir("selected_origin_base");
        let (source_path, bare_path) = create_source_and_bare_repo(&base);
        let staging_path = base.join("staging");
        let feature_path = base.join("feature").join("test-base");

        let add_staging = add_worktree(&bare_path, "staging", &staging_path, Some("main"));
        assert!(
            add_staging.is_ok(),
            "staging worktree should be created: {:?}",
            add_staging
        );

        assert_git_success(
            &git_in(&source_path, &["checkout", "staging"]),
            "git checkout staging failed",
        );
        let latest_staging_head =
            write_and_commit(&source_path, "staging.txt", "v2\n", "staging v2");
        assert_git_success(
            &git_in(&source_path, &["checkout", "main"]),
            "git checkout main failed",
        );

        let fetch_result = fetch_remote_branch(&bare_path, "staging");
        assert!(
            fetch_result.is_ok(),
            "fetch_remote_branch should succeed: {:?}",
            fetch_result
        );

        let add_feature = add_worktree(
            &bare_path,
            "feature/test-base",
            &feature_path,
            Some("staging"),
        );
        assert!(
            add_feature.is_ok(),
            "feature worktree should be created from staging: {:?}",
            add_feature
        );

        let feature_head =
            git_output(&["-C", &feature_path.to_string_lossy(), "rev-parse", "HEAD"]);
        assert!(feature_head.status.success(), "git rev-parse failed");
        assert_eq!(
            String::from_utf8_lossy(&feature_head.stdout).trim(),
            latest_staging_head
        );

        let _ = fs::remove_dir_all(&base);
    }
}
