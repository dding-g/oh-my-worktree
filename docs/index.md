---
layout: home
title: Home
nav_order: 1
---

# owt (oh-my-worktree)

A fast terminal UI for developers who use Git branches as working contexts, not bookmarks.
{: .fs-6 .fw-300 }

[Get Started](/oh-my-worktree/getting-started/installation){: .btn .btn-primary .fs-5 .mb-4 .mb-md-0 .mr-2 }
[View on GitHub](https://github.com/dding-g/oh-my-worktree){: .btn .fs-5 .mb-4 .mb-md-0 }

![Version](https://img.shields.io/badge/version-v{{ site.owt_version }}-blue)

---

## Why owt exists

Modern development rarely happens on one branch at a time. You might be reviewing a PR, testing a hotfix, keeping a long-running feature open, and checking main before a release. Plain `git switch` makes that workflow expensive because every context switch drags along uncommitted files, dependencies, editor state, and mental state.

Git worktrees solve the underlying problem. **owt** makes them easy enough to use every day: open the TUI, pick a worktree, create another one, delete stale contexts, fetch, pull, push, merge, open an editor, or move your shell without remembering the exact Git incantation.

## Start from the repo you already have

You do not need to convert your repository. Run `owt` directly inside an existing regular Git repository; new worktrees default to `~/.owt/worktree/<repo-name>/` unless you configure `worktree_root`.

```
repo/                       # existing non-bare repository
└── .git/

~/.owt/worktree/repo/
├── feature-auth/           # new worktree created by owt
└── hotfix-payment/         # another worktree
```

## Or use the `.bare` sibling layout

If you prefer keeping every worktree inside one project folder, use `owt clone`. The `.bare` layout is supported and convenient, but optional.

```
project/
├── .bare/                # bare repository (hidden)
├── main/                 # main branch worktree
├── feature-auth/         # feature branch worktree
└── hotfix-payment/       # hotfix branch worktree
```

`owt init` prints a conversion guide for people who want this layout later; it is not required before using `owt`.

## Daily worktree workflow

| Without owt | With owt |
|:------------|:---------|
| `git worktree list` | Visual TUI with dirty state, ahead/behind, last commit, and PR status |
| `git worktree add ../feature -b feature` | Press `a`, choose a local or remote branch |
| `cd ../feature && git status` | Press `Enter` to move your shell into the selected context |
| `git fetch && git pull && git push` | Press `f`, `p`, or `P` from the worktree list |
| `git merge origin/main` | Press `m` for upstream merge or `M` for selected branch merge |

## Quick Start

```bash
# Install via npm
npm install -g oh-my-worktree

# Run directly inside an existing regular Git repository
cd /path/to/regular-git-repo
owt

# Or clone into the .bare sibling layout
owt clone https://github.com/user/repo.git

# Navigate into the first `.bare` worktree and launch
cd repo/main
owt
```

## What you get

- **Keyboard-first TUI** - `j`/`k` to move, `/` to search, `a`/`d` to add or delete worktrees
- **Regular repository support** - start in the repo you already use; conversion is not required
- **Optional `.bare` layout** - project-local sibling worktrees when you want everything side by side
- **Fast Git operations** - fetch, pull, push, upstream merge, and selected-branch merge from the list
- **Status and PR visibility** - dirty state, ahead/behind, last commit, and GitHub PR state at a glance
- **Shell and app integration** - `Enter` can move your shell; `o`, `t`, and `y` open editor, terminal, or copy path
