---
layout: default
title: Quick Start
parent: Getting Started
nav_order: 2
---

# Quick Start

Get up and running with owt in minutes.

## New Project

The easiest way to start is with `owt clone`:

```bash
# Clone any Git repository as a bare repo with worktree structure
owt clone https://github.com/user/repo.git

# This creates:
# repo/
# ├── .bare/     <- bare repository
# └── main/      <- first worktree (default branch)

# Navigate to the worktree and launch owt
cd repo/main
owt
```

## Convert Existing Project

If you already have a regular Git repository, use `owt init` for a guided conversion:

```bash
owt init
```

This will show step-by-step instructions for converting your repository.

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
