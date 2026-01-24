# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

owt (oh-my-worktree) is a TUI tool for managing Git worktrees in bare repositories. It uses ratatui + crossterm for the terminal UI.

## Build & Development Commands

```bash
# Build
cargo build
cargo build --release

# Run
cargo run

# Install locally
cargo install --path .

# Run tests
cargo test

# Run single test
cargo test test_is_bare_repo
```

## Architecture

### Module Structure

- `src/main.rs` - CLI entry point, argument parsing, subcommands (clone, init, tui)
- `src/app.rs` - Application state machine, event loop, keyboard handling
- `src/git.rs` - Git command wrappers (list_worktrees, add_worktree, remove_worktree, etc.)
- `src/types.rs` - Core data types (Worktree, WorktreeStatus, AppState, AppMessage)
- `src/config.rs` - Config file parsing (~/.config/owt/config.toml)
- `src/ui/` - UI rendering modules
  - `main_view.rs` - Main worktree list view
  - `add_modal.rs` - Add worktree modal
  - `confirm_modal.rs` - Delete confirmation modal

### Key Patterns

**Bare Repository Detection**: The app supports multiple detection methods:
1. `.bare` folder pattern (e.g., `project/.bare/` with worktrees as siblings)
2. Direct bare repo path (e.g., `project.git/`)
3. Detection from within a worktree via `git rev-parse --git-common-dir`

**State Machine**: AppState enum controls UI mode (List, AddModal, ConfirmDelete)

**Git Operations**: All git operations use `std::process::Command` to call git CLI directly

## npm Distribution

The `npm/` directory contains the npm package structure:
- `install.js` - Postinstall script that downloads the appropriate binary from GitHub Releases
- `bin/owt` - Node.js wrapper that executes the downloaded binary

Release workflow (`.github/workflows/release.yml`) builds binaries for darwin-x64, darwin-arm64, linux-x64, linux-arm64, win32-x64.
