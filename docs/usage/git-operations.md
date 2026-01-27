---
layout: default
title: Git Operations
parent: Usage
nav_order: 3
---

# Git Operations

Perform common Git operations directly from owt.

## Fetching

Press `f` to fetch the selected worktree's remote tracking branch.

This updates:
- Remote branch references
- Ahead/behind indicators

## Pull

Press `p` to pull changes for the selected worktree.

This performs `git pull` in the worktree directory, fetching and merging remote changes.

{: .note }
The worktree must be clean (no uncommitted changes) to pull.

## Push

Press `P` (Shift+p) to push the selected worktree to remote.

This performs `git push` for the current branch.

## Merge Upstream

Press `m` to merge the upstream branch into the selected worktree.

This merges the configured upstream branch (typically `origin/main` or `origin/master`) into your current branch.

## Merge Branch

Press `M` (Shift+m) to select a branch to merge.

1. A modal appears with a list of branches
2. Navigate with `j`/`k`
3. Press `Enter` to merge the selected branch
4. Press `Esc` to cancel

## Operation Status

During long operations:
- The UI shows a "Fetching...", "Pulling...", or similar message
- Key input is blocked until the operation completes
- Success or failure is shown in the status bar

## Tips

### Before Pull/Merge

1. Check the status icon - worktree should be clean (`✓`)
2. Check ahead/behind - see how many commits you'll receive

### After Push

1. Refresh with `r` to update the ahead/behind count
2. The `↑` indicator should reset or decrease

### Merge Conflicts

If a merge creates conflicts:
1. The status changes to `!` (conflict)
2. Exit owt and resolve conflicts manually
3. Commit the merge, then return to owt
