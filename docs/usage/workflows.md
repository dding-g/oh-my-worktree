---
layout: default
title: Workflows
parent: Usage
nav_order: 4
---

# Workflows

Real-world use cases and how to handle them with owt.

## Use Case 1: Developing Multiple Features Simultaneously

**Situation:** You're working on `feature-A` when an urgent request comes in to start `feature-B`.

**Solution:**

1. Press `a` to open the add worktree modal
2. Enter the new branch name (e.g., `feature-B`)
3. Press `Enter` to create and switch to the new worktree
4. Work on `feature-B` in the new directory
5. Use owt to switch back to `feature-A` when needed

```
project/
├── .bare/
├── main/
├── feature-A/     ← original work
└── feature-B/     ← new urgent work
```

No stashing, no context switching headaches.

## Use Case 2: Emergency Hotfix

**Situation:** A critical bug is discovered in production (`main`), but you're in the middle of feature development.

**Solution:**

1. Press `a` to create a new worktree from `main` (e.g., `hotfix/critical-bug`)
2. Press `Enter` to switch to the hotfix worktree
3. Fix the bug
4. Press `P` to push to remote
5. Create a PR and merge
6. Press `m` to merge upstream changes back
7. Return to your feature branch when done

```
project/
├── .bare/
├── main/
├── feature-auth/        ← your feature work (untouched)
└── hotfix/critical-bug/ ← emergency fix
```

Your feature work remains untouched while you handle the emergency.

## Use Case 3: Reviewing a PR Locally

**Situation:** A colleague asks you to test their PR locally before merging.

**Solution:**

1. Press `f` to fetch all remotes (gets the PR branch)
2. Press `a` to create a worktree for the PR branch
3. Test the changes in the new worktree
4. When done, press `d` to delete the review worktree

```
project/
├── .bare/
├── main/
├── feature-yours/           ← your work
└── feature-colleague-pr/    ← PR under review
```

## Use Case 4: Comparing Multiple Versions

**Situation:** You need to compare code between `v1.0` and `v2.0` side by side.

**Solution:**

1. Press `a` to create a worktree for `v1.0` tag
2. Press `a` again to create a worktree for `v2.0` tag
3. Press `o` on each to open both in your editor
4. Compare side by side

```
project/
├── .bare/
├── main/
├── v1.0/    ← old version
└── v2.0/    ← new version
```

Both versions are available simultaneously for comparison.

## Use Case 5: Long-running Task in Background

**Situation:** You need to run a long build or test suite but want to continue working.

**Solution:**

1. Create a worktree for the long-running task
2. Press `t` to open it in a new terminal
3. Run the long task in that terminal
4. Switch back to your main worktree and continue working
5. Check back on the task when needed

## Tips

- **Keep worktrees focused**: One worktree per task/branch
- **Clean up regularly**: Delete worktrees when PRs are merged
- **Use descriptive names**: `feature/auth`, `hotfix/payment`, `review/pr-123`
- **Fetch often**: Press `f` regularly to stay up to date with remote
