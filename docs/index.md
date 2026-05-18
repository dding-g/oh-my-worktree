---
layout: home
title: Home
nav_order: 1
---

# owt (oh-my-worktree)

A TUI tool for managing Git worktrees from either regular repositories or bare `.bare` layouts.
{: .fs-6 .fw-300 }

[Get Started](/oh-my-worktree/getting-started/installation){: .btn .btn-primary .fs-5 .mb-4 .mb-md-0 .mr-2 }
[View on GitHub](https://github.com/dding-g/oh-my-worktree){: .btn .fs-5 .mb-4 .mb-md-0 }

![Version](https://img.shields.io/badge/version-v{{ site.owt_version }}-blue)

---

## What is owt?

**owt** is a terminal user interface (TUI) that makes working with Git worktrees effortless. It provides an intuitive way to:

- View all your worktrees at a glance
- Create new worktrees from any branch
- Switch between worktrees quickly
- Manage worktree lifecycle (add, delete, fetch)
- Open worktrees in your editor or terminal

## What are Git Worktrees?

Git worktrees allow you to check out multiple branches simultaneously from a single repository. This means you can work on multiple features, review PRs, or fix hotfixes in parallel without stashing or switching branches.

Regular repository layout:

```
repo/                       # existing non-bare repository
└── .git/

~/.owt/worktree/repo/
├── feature-auth/           # new worktree created by owt
└── hotfix-payment/         # another worktree
```

Bare `.bare` layout:

```
project/
├── .bare/                # bare repository (hidden)
├── main/                 # main branch worktree
├── feature-auth/         # feature branch worktree
└── hotfix-payment/       # hotfix branch worktree
```

## Why use owt?

| Without owt | With owt |
|:------------|:---------|
| `git worktree list` | Visual TUI with status icons |
| `git worktree add ../feature -b feature` | Press `a`, type branch name |
| `cd ../feature` | Press `Enter` to switch |
| `git fetch && git status` | See status at a glance |

## Quick Start

```bash
# Install via npm
npm install -g oh-my-worktree

# Run directly inside an existing regular Git repository
cd /path/to/regular-git-repo
owt

# Or clone into the .bare sibling layout
owt clone https://github.com/user/repo.git

# Navigate into the first .bare worktree and launch
cd repo/main
owt
```

## Features

- **Vim-style navigation** - `j`/`k` to move, `/` to search
- **Status indicators** - See staged, unstaged, conflict at a glance
- **Base branch selection** - Press `Tab` when adding a worktree to cycle local base branches
- **Shell integration** - `Enter` changes directory to selected worktree
- **Configurable** - Custom editor, terminal, and post-add scripts
