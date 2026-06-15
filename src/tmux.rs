use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

pub fn open_worktree_pane(worktree_path: &Path, worktree_name: &str) -> Result<()> {
    let pane_id = tmux_output(&[
        "split-window".to_string(),
        "-c".to_string(),
        worktree_path.display().to_string(),
        "-P".to_string(),
        "-F".to_string(),
        "#{pane_id}".to_string(),
    ])?;
    let pane_id = pane_id.trim();
    if pane_id.is_empty() {
        anyhow::bail!("tmux did not return a pane id");
    }

    let name = tmux_name(worktree_name);
    tmux_status(&[
        "select-pane".to_string(),
        "-t".to_string(),
        pane_id.to_string(),
        "-T".to_string(),
        name.clone(),
    ])?;

    let window_id = tmux_output(&[
        "display-message".to_string(),
        "-p".to_string(),
        "-t".to_string(),
        pane_id.to_string(),
        "#{window_id}".to_string(),
    ])?;
    let window_id = window_id.trim();
    if !window_id.is_empty() {
        tmux_status(&[
            "rename-window".to_string(),
            "-t".to_string(),
            window_id.to_string(),
            name,
        ])?;
    }

    Ok(())
}

pub fn focus_pane_named(worktree_name: &str) -> Result<bool> {
    let target_name = tmux_name(worktree_name);
    let panes = tmux_output(&[
        "list-panes".to_string(),
        "-a".to_string(),
        "-F".to_string(),
        "#{pane_id}\t#{pane_title}\t#{session_name}:#{window_index}.#{pane_index}".to_string(),
    ])?;

    for line in panes.lines() {
        let mut parts = line.splitn(3, '\t');
        let Some(pane_id) = parts.next() else {
            continue;
        };
        let Some(pane_title) = parts.next() else {
            continue;
        };
        let Some(target) = parts.next() else {
            continue;
        };

        if pane_title == target_name {
            let _ = tmux_status(&[
                "switch-client".to_string(),
                "-t".to_string(),
                target.to_string(),
            ]);
            tmux_status(&[
                "select-pane".to_string(),
                "-t".to_string(),
                pane_id.to_string(),
            ])?;
            return Ok(true);
        }
    }

    Ok(false)
}

fn tmux_output(args: &[String]) -> Result<String> {
    let output = Command::new("tmux")
        .args(args)
        .output()
        .with_context(|| format!("failed to run tmux {}", args.join(" ")))?;

    if !output.status.success() {
        anyhow::bail!(
            "tmux {} failed: {}",
            args.join(" "),
            command_detail(&output)
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn tmux_status(args: &[String]) -> Result<()> {
    let output = Command::new("tmux")
        .args(args)
        .output()
        .with_context(|| format!("failed to run tmux {}", args.join(" ")))?;

    if !output.status.success() {
        anyhow::bail!(
            "tmux {} failed: {}",
            args.join(" "),
            command_detail(&output)
        );
    }

    Ok(())
}

fn command_detail(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stderr.is_empty() {
        stdout
    } else {
        stderr
    }
}

fn tmux_name(name: &str) -> String {
    let name = name.replace(['\t', '\n', '\r'], " ").trim().to_string();
    if name.is_empty() {
        "worktree".to_string()
    } else {
        name
    }
}
