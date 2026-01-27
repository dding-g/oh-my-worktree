---
layout: default
title: Bare Repositories
parent: Concepts
nav_order: 1
---

# Bare Repositories

A bare repository is a Git repository that doesn't have a working directory.

## What is a Bare Repository?

In a normal Git repository, you have:
- `.git/` folder containing all Git data
- Working directory with your files

In a **bare** repository, you only have the Git data - no working directory:

```
# Normal repository
my-project/
├── .git/           <- Git data
├── src/            <- Working files
└── README.md

# Bare repository
my-project.git/
├── HEAD
├── config
├── objects/        <- Git data (no working files)
└── refs/
```

## Why Use Bare Repositories?

Bare repositories are the foundation for the worktree workflow because:

1. **No conflicts**: The bare repo has no working directory to conflict with worktrees
2. **Clean structure**: All worktrees are siblings, creating a clear organization
3. **Shared data**: All worktrees share the same Git objects, saving disk space

## The .bare Convention

owt uses a special convention where the bare repository is stored in a `.bare` folder:

```
project/
├── .bare/          <- bare repository (hidden)
├── main/           <- worktree for main branch
├── feature-a/      <- worktree for feature-a branch
└── hotfix-b/       <- worktree for hotfix-b branch
```

This keeps the bare repository hidden while your worktrees are visible at the top level.

## Creating a Bare Repository

### With owt (recommended)

```bash
owt clone https://github.com/user/repo.git
```

### Manually

```bash
git clone --bare https://github.com/user/repo.git .bare
```

## Converting an Existing Repository

See [Quick Start - Convert Existing Project](/oh-my-worktree/getting-started/quick-start#convert-existing-project).
