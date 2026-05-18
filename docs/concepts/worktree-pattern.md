---
layout: default
title: The .bare Pattern
parent: Concepts
nav_order: 2
---

# The .bare Pattern

owt supports regular Git worktrees, but the `.bare` pattern is the default structure created by `owt clone`.

## Structure

```
project/
├── .bare/              <- bare repository
├── main/               <- worktree: main branch
│   ├── src/
│   ├── package.json
│   └── ...
├── feature-auth/       <- worktree: feature-auth branch
│   ├── src/
│   ├── package.json
│   └── ...
└── hotfix-payment/     <- worktree: hotfix-payment branch
    ├── src/
    ├── package.json
    └── ...
```

## Benefits

### 1. Clean Organization

All worktrees are siblings at the same level. This makes it easy to:
- See all active branches at a glance
- Navigate between worktrees with `cd ../other-branch`
- Keep track of what you're working on

### 2. Hidden Repository

The `.bare` folder is hidden (starts with a dot), so your project folder shows only your worktrees.

### 3. Consistent Naming

Worktree folder names match branch names, making navigation intuitive.

## How owt Detects This Pattern

owt looks for the `.bare` folder in the parent directory of your current location. When you run `owt`:

1. If you're in a worktree, owt finds `.bare` in the parent
2. If you're in the project root, owt finds `.bare` directly
3. owt can also work with traditional `repo.git` bare repositories
4. If no bare repo is found, owt falls back to the current regular Git repository

## Creating This Structure

### New Repository

```bash
owt clone https://github.com/user/repo.git
```

### Existing Repository

```bash
# In your existing repo
mv .git .bare
echo "gitdir: ./.bare" > .git
git config --bool core.bare true
git worktree add main main
```

## Alternative Structures

owt also supports traditional bare repository layouts:

```
# Traditional layout
project.git/        <- bare repository
project-main/       <- worktree
project-feature/    <- worktree
```

For regular non-bare repositories, new worktrees are created under `~/.owt/worktree/<repo-name>/` by default. Configure `worktree_root` to choose another root directory.

However, the `.bare` pattern is still recommended for a cleaner project-local structure.
