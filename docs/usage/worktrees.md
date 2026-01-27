---
layout: default
title: Managing Worktrees
parent: Usage
nav_order: 2
---

# Managing Worktrees

Create, switch, and delete worktrees.

## Adding a Worktree

Press `a` to open the add worktree dialog.

### Step 1: Select Branch Type

Choose a branch type shortcut or use custom:

| Key | Type | Prefix |
|:----|:-----|:-------|
| `f` | Feature | `feature/` |
| `b` | Bugfix | `bugfix/` |
| `h` | Hotfix | `hotfix/` |
| `c` | Custom | (no prefix) |

Branch types are [configurable](/oh-my-worktree/reference/configuration).

### Step 2: Enter Branch Name

Type your branch name. If you selected a branch type, the prefix is already added.

**Keyboard shortcuts in this screen:**

| Key | Action |
|:----|:-------|
| `Enter` | Create worktree |
| `Esc` | Cancel |
| `Shift+F` | Fetch base branch from remote |
| `Shift+L` | Use local branch as base |
| `Shift+U` | Use remote branch as base |

### What Happens

When you create a worktree:

1. A new folder is created next to your existing worktrees
2. If configured, files are copied from an existing worktree (e.g., `.env`)
3. If configured, a post-add script runs (e.g., `npm install`)

## Switching Worktrees

1. Navigate to the desired worktree with `j`/`k`
2. Press `Enter`

Your shell's current directory changes to that worktree.

## Deleting a Worktree

1. Select the worktree to delete
2. Press `d`
3. Confirm with `y` or `Enter`
4. Optionally press `b` to also delete the branch

{: .warning }
You cannot delete a worktree with uncommitted changes. Commit or stash your changes first.

## Opening in External Apps

| Key | Action |
|:----|:-------|
| `o` | Open in editor (`$EDITOR`) |
| `t` | Open in terminal |

These use your configured editor and terminal. See [Configuration](/oh-my-worktree/reference/configuration).

## Copying Path

Press `y` to copy the worktree path to your clipboard.

## Refreshing

Press `r` to refresh the worktree list. This updates:
- Status indicators
- Ahead/behind counts
- Last commit times
