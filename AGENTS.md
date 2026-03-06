# AGENTS.md

This file provides guidance to Codex-style coding agents when working in this repository.

## Project Overview

`owt` (oh-my-worktree) is a Rust TUI CLI for managing Git worktrees in bare repositories.

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

## Development Conventions

- Preserve existing module boundaries: UI in `src/ui`, git behavior in `src/git.rs`.
- Prefer minimal, focused changes over broad refactors.
- When changing keybindings or user-facing behavior, update relevant docs (`README.md`, `README.ko.md`, `docs/`).
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
