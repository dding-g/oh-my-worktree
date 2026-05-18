# AGENTS.md

This file provides guidance to Codex-style coding agents when working in this repository.

## Project Overview

`owt` (oh-my-worktree) is a Rust TUI CLI for managing Git worktrees in both regular non-bare repositories and bare repository layouts.

- Runtime: Rust 2021
- UI stack: `ratatui` + `crossterm`
- Git integration: direct `git` CLI calls via `std::process::Command`
- Distribution: Rust binary + npm wrapper package (`npm/`)

## Build, Test, and Dev Commands

```bash
# Build
cargo build
cargo build --release

# Run TUI
cargo run

# Run tests
cargo test

# Run a single test
cargo test test_is_bare_repo

# Format
cargo fmt
```

## Architecture

- `src/main.rs`: CLI entry, subcommands (`clone`, `init`, `setup`, default TUI)
- `src/app.rs`: app state machine, event loop, key handling, background ops
- `src/git.rs`: git command wrappers (`list_worktrees`, `add_worktree`, `remove_worktree`, merge/pull/push)
- `src/config.rs`: config loading and parsing
- `src/types.rs`: shared app/domain types
- `src/ui/`: TUI rendering (`main_view`, `add_modal`, `confirm_modal`, `help_modal`, etc.)

## Repository Layout Policy

- Regular non-bare repositories are first-class: users can run `owt` directly without converting to a bare repository.
- In regular repositories, new worktrees default to `~/.owt/worktree/<repo-name>/` unless `worktree_root` is configured.
- The `.bare` sibling layout remains supported and is the layout created by `owt clone`; describe it as a recommended project-local layout, not the only supported product shape.
- `owt init` is a conversion guide for users who prefer `.bare`, not a prerequisite for using `owt`.
- Keep the Korean SSOT policy in `docs/ssot/01-repository-worktree-policy.md` aligned with README and usage documentation when changing repository-layout behavior or positioning.

## Development Conventions

- Preserve existing module boundaries: UI in `src/ui`, git behavior in `src/git.rs`.
- Prefer minimal, focused changes over broad refactors.
- When changing keybindings or user-facing behavior, update relevant docs (`README.md`, `README.ko.md`, `docs/`).
- When changing repository-layout behavior or wording, update both `README.md` and `README.ko.md`, then verify `docs/ssot/01-repository-worktree-policy.md` still matches the behavior.
- For git/worktree behavior changes, add or update regression tests first.

## Testing Expectations

- Primary quality gate: `cargo test`.
- For worktree edge cases, use isolated temporary repositories in tests (see `tests/git_test.rs` and `src/git.rs` tests).
- Do not claim completion without running tests that cover the modified behavior.

## Release Notes

- Release command: `npm run release` (interactive `release-it` flow).
- Release bump updates:
  - `Cargo.toml`
  - `Cargo.lock`
  - `package.json`
  - `npm/package.json`
- If release is interrupted, these files may remain modified and should be handled explicitly.

## Compatibility Note

`CLAUDE.md` is kept for Claude Code compatibility.
For Codex workflows, treat this `AGENTS.md` as the primary operational guide.
