---
layout: default
title: Keybindings
parent: Reference
nav_order: 1
---

# Keybindings

Complete list of keyboard shortcuts.

## Navigation

| Key | Action |
|:----|:-------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `gg` / `Home` | Go to top |
| `G` / `End` | Go to bottom |
| `Ctrl+d` | Half page down |
| `Ctrl+u` | Half page up |
| `g` | Jump to current worktree |
| `/` | Search worktrees |
| `Space` | Select/unselect a worktree for batch actions |
| `Enter` | Enter worktree (cd) |

## Worktree Actions

| Key | Action |
|:----|:-------|
| `a` | Add new worktree |
| `d` | Delete selected worktree(s) |
| `x` | Preview the same stale-metadata and completed-PR cleanup as `owt worktree prune`; confirm before removal |
| `r` | Refresh list |
| `s` | Cycle sort mode |

## Git Operations

| Key | Action |
|:----|:-------|
| `f` | Fetch remotes |
| `p` | Pull selected worktree(s) from remote |
| `P` | Push to remote |
| `m` | Merge upstream |
| `M` | Merge branch (select) |
| `R` | Query PR status for selected worktree, all worktrees, or an arbitrary branch |
| `C` | Browse selected worktree commit tree; `+`/`-` changes the limit |
| `N` | Clone a `.bare` workspace after terminal restore |
| `I` / `S` | Show the current repository init guide / run shell setup after terminal restore |
| `V` | Show About/version |

## External Apps

| Key | Action |
|:----|:-------|
| `o` | Open in editor |
| `t` | Open in terminal |

## Other

| Key | Action |
|:----|:-------|
| `y` | Copy path to clipboard |
| `v` | Toggle verbose Git command details |
| `c` | Open config modal |
| `?` | Show help |
| `q` | Quit |
| `Ctrl+c` | Quit |
| `Esc` | Close modal / clear filter |

## Add Worktree Modal

| Key | Action |
|:----|:-------|
| `Enter` | Create worktree |
| `Tab` | Cycle base branch |
| `Ctrl+p` | Switch between branch and explicit destination path input |
| `Ctrl+t` | Cycle this create's tmux override: default → on → off |
| `Esc` | Cancel |

## Delete Confirmation

| Key | Action |
|:----|:-------|
| `y` / `Enter` | Confirm delete |
| `n` / `Esc` | Cancel |
| `b` | Toggle delete branch |
| `f` | Toggle force delete for dirty worktrees |

## Config Modal

| Key | Action |
|:----|:-------|
| `j` / `k` | Navigate options |
| `Enter` | Edit selected option |
| `s` | Save config to file |
| `Esc` / `q` | Close config |

## Search Mode

| Key | Action |
|:----|:-------|
| (any text) | Filter by path, name, branch, status, or PR |
| `Enter` | Enter selected worktree |
| `Esc` | Cancel search |
| `Backspace` | Delete character |
