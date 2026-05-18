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

Type your branch name. Use `Tab` to cycle the base branch for the new worktree. The first default is `main`; after you choose a different base branch, that branch remains the default for later worktrees in the same session.

**Keyboard shortcuts in this screen:**

| Key | Action |
|:----|:-------|
| `Enter` | Create worktree |
| `Tab` | Cycle base branch |
| `Esc` | Cancel |

### What Happens

When you create a worktree:

1. A new folder is created next to your existing worktrees for the `.bare` layout, or under `~/.owt/worktree/<repo-name>/` for regular non-bare repos
2. If configured, files are copied from an existing worktree (e.g., `.env`)
3. If configured, a post-add script runs (e.g., `npm install`)

Set `worktree_root` in `~/.config/owt/config.toml` to change the root used for regular repositories or to override the default location for new worktrees.

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
