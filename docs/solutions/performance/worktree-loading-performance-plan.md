---
title: Worktree Loading Performance Plan
description: Durable plan for reducing owt startup and list latency when a repository has many worktrees.
when_to_apply:
  - TUI startup feels slow in repositories with many worktrees.
  - Plain CLI list/search performance changes touch worktree status, commit, ahead/behind, or PR status loading.
  - A future change introduces caching, background loading, or batched Git operations for worktree metadata.
related:
  - docs/solutions/best-practices/ai-agent-project-map.md
  - docs/ssot/03-cli-tui-use-case-contract.md
  - docs/ssot/04-git-operation-safety-policy.md
  - src/git.rs
  - src/app.rs
  - src/main.rs
---

# Worktree Loading Performance Plan

## Problem

`owt` currently treats worktree listing as a fully detailed scan. `git::list_worktrees` starts with one `git worktree list --porcelain`, but then enriches every non-bare worktree immediately:

- `get_status(path)` runs `git status --porcelain`.
- `get_last_commit_time(path)` runs a Git log command.
- `get_ahead_behind(path)` runs `git rev-list --left-right --count @{upstream}...HEAD`.
- PR status is optional, but callers such as `worktree list --pr`, `search --pr`, and prune use `github_pr_statuses_for_worktrees`.

This creates an O(N) subprocess fan-out with several Git commands per worktree. In repositories with dozens of worktrees, TUI startup is blocked by metadata that is not required to draw the first screen.

## Goal

Make `owt` feel instant when opening a large workspace:

1. Show the first TUI frame from cheap worktree metadata only.
2. Fill expensive row metadata in the background.
3. Keep plain CLI output deterministic and script-friendly.
4. Preserve destructive-operation safety by rechecking live state before delete/prune.

## Non-Goals

- Do not make prune/delete rely on cached status.
- Do not silently drop existing plain CLI fields without a compatibility decision in SSOT.
- Do not introduce background mutation of Git state. Background loading should read only.

## Recommended Architecture

### 1. Split Fast and Detailed Worktree Loading

Add a cheap loader:

- `git::list_worktrees_fast(repo_path) -> Vec<Worktree>`
- Source: only `git worktree list --porcelain`.
- Fill path, branch, bare flag.
- Use conservative placeholders for expensive fields:
  - `status`: either a new `WorktreeStatus::Unknown` or keep `Clean` only if UI can visually distinguish "not loaded" elsewhere.
  - `last_commit_time`: `None`
  - `ahead_behind`: `None`
  - `github_pr_status`: `None`

Keep detailed loading separate:

- `git::load_worktree_details(path)` for one worktree.
- `git::refresh_worktree_metadata(repo_path, targets)` for background refresh.
- Existing `git::list_worktrees` can remain detailed for compatibility, but its name should eventually become explicit, for example `list_worktrees_detailed`.

### 2. TUI Startup Uses Fast Loading

`App::new` should call the fast loader and render immediately. After the first frame:

- Queue a background metadata refresh.
- Prioritize visible rows and cursor-adjacent rows.
- Use bounded concurrency, for example 4 to 8 workers, rather than one thread per worktree.
- Apply results back into existing rows by path, not by index, because worktree lists can change.

The UI should distinguish unloaded values from clean/empty values. A blank ahead/behind field can remain acceptable, but dirty status must not look definitively clean before status has loaded.

### 3. Batch What Can Be Batched

Some metadata does not require per-worktree commands:

- Last commit information can be read by branch ref with `git for-each-ref --format=... refs/heads/`.
- Upstream branch mapping can also come from `for-each-ref` fields such as upstream short name.
- PR status already uses one `gh pr list` batch query. Keep this contract and avoid per-branch `gh` calls.

Working tree status still needs each worktree path because it depends on the checkout. Optimize it with visible-first scheduling and bounded concurrency rather than pretending it is batchable.

### 4. Cache Read-Only Metadata

Add a short-lived cache only after the fast/background split exists.

Suggested cache file:

- `.owt/cache/worktrees.json` for project-local `.bare` layouts.
- A keyed cache under the configured owt state directory for regular repositories.

Cache candidates:

- path
- branch
- last commit time
- ahead/behind
- PR status
- previous status with `loaded_at`

Cache rules:

- Startup may display cached values immediately if marked stale.
- Background refresh should replace cached values.
- Destructive commands must ignore cached status and re-run live checks.
- Cache invalidation can be TTL-based at first. A 5 to 30 second TTL is enough to avoid repeated startup scans during active sessions.

### 5. Plain CLI Compatibility

Plain CLI is used by scripts and agents, so changes need explicit behavior:

- Keep `owt worktree list` output shape stable unless SSOT is updated.
- Prefer adding `--fast` for cheap output or `--details` for explicit expensive output before changing defaults.
- `owt search` can use fast data by default for path/name/branch matches and only load details when the query can match status, ahead/behind, last commit, or PR status.
- `--pr` should continue to mean PR status is loaded before output.

### 6. Safety Boundary for Mutations

Fast loading and cache are display optimizations only.

Before these operations mutate anything, they must use live checks:

- `owt worktree delete`
- `owt worktree prune`
- batch delete from TUI
- pull/push/merge safety checks that depend on dirty status

For prune specifically:

- It may use the batched PR status lookup for completed PR state.
- It must call live status for each candidate before removal.
- It must preserve current protections for current worktree, HEAD branch worktree, dirty worktree, bare entry, detached entry, and branch deletion.

## Implementation Order

1. Measure current command counts and wall time in a large repo.
   - Record `owt worktree list`, `owt worktree list --pr`, and TUI startup time.
   - Add lightweight instrumentation behind an env var, for example `OWT_TRACE_GIT=1`, if needed.

2. Introduce `list_worktrees_fast`.
   - Keep existing tests for detailed behavior.
   - Add tests that fast loading does not call status/log/ahead-behind helpers.

3. Wire TUI startup to fast loading.
   - Render immediately with placeholder metadata.
   - Add background refresh for detailed fields.

4. Add bounded metadata refresh.
   - Start with status only.
   - Add last commit and ahead/behind after status is stable.

5. Batch branch-derived metadata.
   - Replace per-worktree last commit calls with a branch map.
   - Evaluate upstream/ahead/behind batching separately because upstream can be missing.

6. Add cache.
   - Cache after background refresh is working.
   - Make stale state visible enough that users do not confuse cached dirty state with live safety.

7. Revisit CLI defaults.
   - Decide in SSOT whether `worktree list` remains detailed by default or gains a fast default in a breaking/minor release.

## Testing Strategy

Use isolated repositories with many worktrees:

- 1 bare worktree root with 30 to 100 worktrees.
- Mix clean, staged, unstaged, detached, and missing-upstream worktrees.
- Include nested branch names and `.bare` sibling layout.
- Include at least one submodule worktree because prune has a special fallback for Git's submodule remove refusal.

Regression tests should assert:

- Fast loader returns all worktree paths and branches.
- TUI can construct `App` without detailed metadata.
- Background refresh updates rows by path.
- Dirty status is not treated as clean while unknown.
- Delete/prune still perform live checks.

## Expected Impact

The biggest user-visible win is not reducing total metadata refresh time. It is removing metadata refresh from the startup critical path.

After phase 1 and 2, the first TUI frame should depend mostly on a single `git worktree list --porcelain` call. Detailed status may still take time, but it should no longer block opening `owt`.
