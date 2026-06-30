---
name: owt-worktree
description: Use whenever an AI agent needs to list, create, enter, delete, prune, search, or inspect Git worktrees. Requires using owt plain CLI worktree commands instead of raw git worktree mutation commands.
---

# owt Worktree Handling

Use `owt` as the worktree manager. Its plain CLI preserves repository-layout policy, configured worktree roots, copy-file behavior, tmux worktree mode, and parseable output.

## Core Rule

Use `owt worktree ...` for worktree handling. Do not use raw `git worktree add`, `git worktree remove`, or `git worktree prune` unless `owt` cannot run and the user explicitly approves a fallback.

## First Checks

```bash
owt --version
owt worktree --help
owt worktree list
```

`owt worktree list` and `owt search` print:

```text
kind<TAB>path<TAB>branch<TAB>status<TAB>last_commit<TAB>ahead<TAB>behind<TAB>pr
```

`owt worktree create` prints:

```text
created<TAB>branch<TAB>path
```

## Common Operations

List worktrees:

```bash
owt worktree list
owt worktree list --pr
```

Create a worktree:

```bash
owt worktree create feature/example --base main
```

Capture the created path for follow-up commands:

```bash
target_path=$(owt worktree create feature/example --base main | awk -F '\t' '$1=="created"{print $3; exit}')
```

Create with tmux pane creation only when requested:

```bash
owt worktree create feature/example --base main --tmux=on
```

Find a worktree:

```bash
owt search example
```

Enter a worktree in a shell command:

```bash
cd "$target_path"
```

Delete a worktree:

```bash
owt worktree delete feature/example
```

Delete the local branch too only when requested:

```bash
owt worktree delete feature/example --branch
```

Use `--force` only after checking and accepting dirty-worktree risk.

Prune stale metadata:

```bash
owt worktree prune
owt worktree prune --dry-run
```

`owt worktree prune` logs every worktree decision as tab-separated output. It also removes non-current worktrees when they are clean and their branch has already been merged into `HEAD`, except the `HEAD` branch worktree itself. `--dry-run` previews metadata pruning, prompts through removable candidates, and records selected candidates without deleting them. It does not delete branches, dirty worktrees, unmerged worktrees, detached worktrees, bare entries, the current worktree, or the `HEAD` branch worktree.

## Guardrails

- Prefer branch/name/path inputs accepted by `owt worktree delete`; do not hand-roll path matching with raw Git mutation commands.
- Treat `status` values from `owt worktree list` as safety signals before delete or cleanup.
- Keep stdout parsing tab-based. Warnings and tmux messages may appear on stderr.
- Use `owt pr status` for GitHub PR state and `owt commit tree` for recent commit graph checks.
- If `owt` is unavailable, use `.agents/skills/owt-install/SKILL.md` before attempting worktree mutations.
