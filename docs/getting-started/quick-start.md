---
layout: default
title: Quick Start
parent: Getting Started
nav_order: 2
---

# Quick Start

Get up and running with owt in minutes.

## Existing Regular Repository

If you already have a regular Git repository, start there. You do not need to convert it to a bare repository.

```bash
cd /path/to/regular-git-repo
owt
```

When you add a worktree from a regular repository, `owt` creates it under `~/.owt/worktree/<repo-name>/` by default. Configure `worktree_root` if you want a different root directory.

## New Project with `.bare`

Use `owt clone` when you want the project-local `.bare` sibling layout:

```bash
# Clone any Git repository into the .bare sibling layout
owt clone https://github.com/user/repo.git

# This creates:
# repo/
# ├── .bare/     <- bare repository
# └── main/      <- first worktree (default branch)

# Navigate to the worktree and launch owt
cd repo/main
owt
```

## Optional: Convert Existing Project to `.bare`

If you prefer the `.bare` sibling layout, use `owt init` for a guided conversion:

```bash
owt init
```

This shows step-by-step instructions for converting your repository. Conversion is optional; regular repositories are supported directly.

### Manual Conversion

You can also convert manually:

```bash
# In your existing repo
mv .git .bare
echo "gitdir: ./.bare" > .git
git config --bool core.bare true

# Create first worktree
git worktree add main main

# Launch owt
cd main
owt
```

## Basic Usage

Once in the TUI:

| Key | Action |
|:----|:-------|
| `j` / `k` | Move down / up |
| `Enter` | Enter worktree (cd) |
| `a` | Add new worktree |
| `d` | Delete worktree |
| `?` | Show all keybindings |

## Next Steps

- [Set up shell integration](/oh-my-worktree/getting-started/shell-integration) for seamless directory changing
- [Learn the keybindings](/oh-my-worktree/reference/keybindings) for efficient navigation
- [Configure owt](/oh-my-worktree/reference/configuration) for your workflow
