---
layout: default
title: Why Worktrees?
parent: Concepts
nav_order: 3
---

# Why Worktrees?

Git worktrees solve common frustrations with branch switching.

## The Problem

Without worktrees, when you need to switch branches you must:

1. **Stash or commit** your work-in-progress
2. **Wait** for the checkout to complete
3. **Restore** your environment (dependencies, builds, etc.)
4. **Repeat** when switching back

This gets especially painful when:
- You're mid-debugging and need to check another branch
- You're reviewing a PR while working on a feature
- You need to hotfix production while developing
- Your project has slow builds or installs

## The Solution: Worktrees

With worktrees, each branch has its own directory:

```
project/
├── .bare/
├── main/             <- Your stable branch
├── feature-auth/     <- Your current feature
└── pr-review-123/    <- PR you're reviewing
```

Now you can:
- **Switch instantly** by changing directories
- **Keep environments** for each branch
- **Run multiple branches** simultaneously
- **Compare code** side by side

## Common Workflows

### Hotfix While Developing

```bash
# Working on feature-auth...
# Production issue reported!

owt                   # Launch TUI
a                     # Add worktree
# Select hotfix type, enter name
Enter                 # Switch to new worktree

# Fix, commit, push, then...

owt
# Select feature-auth
Enter                 # Right back where you were!
```

### PR Review

```bash
owt
a                     # Add worktree
# Enter branch name from PR

# Review, test, comment...
# Then delete when done

owt
d                     # Delete the review worktree
```

### Parallel Testing

Run different branches simultaneously:

```bash
# Terminal 1: main worktree
cd project/main
npm run dev -- --port 3000

# Terminal 2: feature worktree
cd project/feature-auth
npm run dev -- --port 3001

# Compare behavior side by side!
```

## Disk Space

Worktrees are efficient because they share:
- Git objects (commits, blobs, trees)
- Remote tracking data
- Configuration

Each worktree only stores:
- Working directory files
- Worktree-specific config

For a typical project, each worktree adds only the size of your source files, not the entire Git history.

## When Not to Use Worktrees

Worktrees may not be ideal when:
- Your project is very large and disk space is limited
- You only ever work on one thing at a time
- Your workflow requires a single checkout location

But for most developers, worktrees significantly improve productivity.
