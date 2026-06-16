---
title: AI Agent Project Map for owt
date: 2026-05-18
category: docs/solutions/best-practices
module: project_architecture
problem_type: best_practice
component: documentation
severity: medium
applies_when:
  - onboarding an AI agent to the repository
  - changing worktree behavior, TUI flows, or documentation policy
  - debugging cross-module behavior in the Rust CLI
tags: [ai-onboarding, architecture, rust-tui, worktrees]
---

# AI Agent Project Map for owt

## Context

`owt` is a Rust 2021 TUI CLI for managing Git worktrees. The product now supports both regular non-bare repositories and bare repository layouts, so future agents must avoid assuming `.bare` is the only supported shape.

This document gives future agents a fast, durable map of the project before they edit code or docs.

## Guidance

Start with these files in order:

1. `AGENTS.md` - operational rules for agents, test expectations, and repo-layout policy.
2. `docs/ssot/00-ssot-index.md` - SSOT map for product behavior, policy boundaries, and user-case contracts.
3. `docs/ssot/01-repository-worktree-policy.md` - canonical policy for regular repo vs `.bare` behavior and docs positioning.
4. `src/main.rs` - CLI commands, repository-mode detection, TUI boot, shell integration output handoff.
5. `src/app.rs` - central TUI state machine, event loop, modal dispatch, background operation polling.
6. `src/git.rs` - all Git CLI wrappers and worktree behavior.
7. `src/config.rs` - config loading, project/global precedence, and script trust boundary.
8. `src/types.rs` - shared domain types for app, UI, and Git behavior.
9. `docs/usage/worktrees.md` and `docs/reference/configuration.md` - user-facing behavior contracts.

## Architecture Map

| Area | Files | Responsibility |
|---|---|---|
| CLI entry | `src/main.rs` | Parses commands: default TUI, `clone`, `init`, `setup`, `test-cd`, help, version, plus noun-first plain CLI groups (`worktree`, `pr`, `commit`, `search`). Detects repo layout before TUI or plain CLI operations. |
| App state | `src/app.rs` | Owns worktrees, selected row, modal state, messages, shell integration state, background ops, selected details, sorting, filtering. |
| Git integration | `src/git.rs` | Runs `git` via `std::process::Command`; lists/adds/removes worktrees; fetch/pull/push/merge; derives status, details, dates. |
| Config | `src/config.rs` | Loads global and project config; project config may override safe values but must not enable trusted post-add auto-run. |
| Domain types | `src/types.rs` | `Worktree`, `WorktreeStatus`, `AppState`, `ExitAction`, `OpKind`, `OpResult`, details and message types. |
| UI rendering | `src/ui/` | Ratatui views and modals. UI modules render from `App`; app logic should stay outside UI renderers. |
| Distribution | `npm/` | npm wrapper and installer for released binaries. |
| Docs site | `docs/` | Jekyll/GitHub Pages docs, homepage assets, SSOT, usage/reference/concepts docs. |

## Key Execution Flows

### TUI Launch

`src/main.rs` decides the repo mode:

1. Prefer `.bare` in the current path when present.
2. Else, if inside Git, use `git rev-parse --git-common-dir`.
3. If the common dir is bare, treat it as a bare layout.
4. Otherwise, treat the current repo as regular non-bare and use the worktree root as project root.
5. Create `App::new(repo_path, project_root_path, repo_is_bare, launch_path, has_shell_integration)`.

### Plain CLI Surface

Agent/script use cases should prefer the noun-first plain CLI instead of driving the TUI:

- `owt worktree list/create/delete/prune`
- `owt pr status`
- `owt commit tree`
- `owt search <QUERY>`

These commands follow the GitHub CLI help pattern (`owt <noun> --help`, action-level `--help`) and keep stdout parseable. Worktree listing/search output is tab-separated as `kind path branch status last_commit ahead behind pr`. Decorative tables, color, and TUI escape sequences do not belong on this surface.

Agent bootstrap assets live under `.agents/`: `.agents/prompts/install-owt.md` is the copy/paste setup prompt, `.agents/skills/owt-install/SKILL.md` verifies or installs the CLI, and `.agents/skills/owt-worktree/SKILL.md` directs worktree mutations through `owt worktree ...` instead of raw `git worktree`.

`owt worktree prune` is a cleanup command, not just a `git worktree prune` wrapper: it removes stale metadata and also deletes non-current worktrees only when they are clean and their local branch is already merged into `HEAD`. It never deletes the branch.

### Shell Integration

`owt setup` installs a shell function that sets `OWT_OUTPUT_FILE`. When the user presses `Enter` on a worktree, `App` sets `ExitAction::ChangeDirectory(path)`. When the user confirms the add modal, `App` sets `ExitAction::CreateWorktree(request)` and quits the TUI. After the terminal is restored, `main.rs` writes the selected or created worktree path into the secure output file. `open_shell_output_file` rejects symlinks and group/world-accessible files.

### Worktree Creation

The TUI add flow is split between `App` and `main.rs`:

- The add modal collects branch/base input and queues `ExitAction::CreateWorktree`; it does not create the worktree while ratatui owns the terminal.
- `main.rs` runs the post-TUI create workflow after terminal restore: fetch base branch, `git worktree add`, configured file copies, optional post-add script, optional tmux pane, then shell handoff.
- Git operations run through `git_command()`, which strips inherited `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`, and `GIT_COMMON_DIR`.
- Regular repositories create new worktrees under `~/.owt/worktree/<repo-name>/` unless `worktree_root` is configured.
- `.bare` layouts keep sibling worktree behavior next to existing worktrees.
- `tmux_worktree_mode` is separate from detached post-add script execution: it opens/focuses tmux panes for worktrees and can be controlled by project config or `owt worktree create --tmux=on`.

### Background Operations

Git operations that may block the TUI use background operation types in `types.rs` and receiver polling in `App`. Preserve the pattern: UI remains responsive, operation status is represented in app state, and results are surfaced as `AppMessage`/details refreshes.

## Why This Matters

Most mistakes in this repo come from crossing boundaries:

- Treating `.bare` as the only product layout after regular repo support was added.
- Putting behavior into UI rendering modules instead of `App`/`git`.
- Letting project-local config enable trusted script execution.
- Running Git commands in an inherited hook/shell environment without clearing `GIT_*` variables.
- Changing keybindings or visible behavior without updating docs.

## When to Apply

- Before feature work that touches `src/main.rs`, `src/app.rs`, `src/git.rs`, `src/config.rs`, or docs.
- Before refactoring UI modules or moving app state.
- Before updating README, docs, or release assets.

## Examples

Recent commit history signals the project’s real fault lines:

- `22e8b00 feat: support non-bare worktree creation` made regular repositories first-class.
- `cb3d130 docs: align worktree layout documentation` turned that behavior into docs/SSOT policy.
- `dcb6dab fix: isolate git commands from hook env` established the `git_command()` environment-clearing pattern.
- `27515e2 fix: harden shell integration output file` established the secure `OWT_OUTPUT_FILE` handoff pattern.
- `c4cb724 feat: show dates in recent commit graph` shows UI display changes often touch both `src/git.rs` data formatting and `src/ui/main_view.rs` rendering.
- Favicon/dark screenshot commits show docs assets should be archived under `assets/` and published under `docs/` intentionally.

## Related

- `AGENTS.md`
- `docs/ssot/01-repository-worktree-policy.md`
- `docs/solutions/documentation-gaps/repository-layout-documentation-contract.md`
- `docs/solutions/developer-experience/compound-engineering-workflow.md`
