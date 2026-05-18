---
layout: default
title: Configuration
parent: Reference
nav_order: 2
---

# Configuration

owt can be configured via a config file and environment variables.

## Config File

The config file is located at `~/.config/owt/config.toml`.

### Example

```toml
# Editor to use when pressing 'o'
editor = "code"

# Terminal app (macOS)
terminal = "Ghostty"

# Root directory for new worktrees from regular non-bare repositories
worktree_root = "~/.owt/worktree"

# Files to copy when creating a new worktree
copy_files = [".env", ".env.local"]

# Launch .owt/post-add.sh in a detached tmux session after worktree creation
run_post_add_script_in_tmux = false

```

### Options

| Option | Type | Description |
|:-------|:-----|:------------|
| `editor` | string | Editor command to open worktrees |
| `terminal` | string | Terminal app name (macOS) or command (Linux) |
| `worktree_root` | string | Root directory for new worktrees from regular non-bare repositories. Defaults to `~/.owt/worktree` |
| `copy_files` | array | Files to copy to new worktrees |
| `run_post_add_script_in_tmux` | boolean | Run `.owt/post-add.sh` in tmux after creating a worktree. This must be enabled from global config; project config cannot enable script auto-run. |

## Environment Variables

| Variable | Description | Default |
|:---------|:------------|:--------|
| `EDITOR` | Editor command | `vim` |
| `TERMINAL` | Terminal app | `Terminal` (macOS) |

Environment variables override config file settings.

### Examples

```bash
# Use VS Code
export EDITOR=code

# Use Ghostty on macOS
export TERMINAL=Ghostty
```

## Project-Level Configuration

You can create a project-specific config in `.owt/config.toml` at the project root. For the `.bare` convention, this lives next to your `.bare` folder:

```
project/
├── .bare/
├── .owt/
│   └── config.toml      <- Project config
└── main/
```

Project config overrides global config.

## Post-Add Script

Create `.owt/post-add.sh` to run commands after creating a worktree, then enable tmux execution in config:

```bash
#!/bin/bash
# Runs in the new worktree directory

npm install
cp .env.example .env
```

Make it executable:

```bash
chmod +x .owt/post-add.sh
```

```toml
run_post_add_script_in_tmux = true
```

Post-add scripts are tmux-only. If `run_post_add_script_in_tmux` is `false`, owt does not run the script. When enabled from global config, owt starts a detached tmux session in the new worktree and the session is removed after the script finishes. Project config can define the script path, but cannot enable automatic script execution.

## Editing Config in TUI

Press `c` to open the config modal:

1. Navigate with `j`/`k`
2. Press `Enter` to edit a value
3. Press `s` to save changes
4. Press `Esc` to close

## Default Configuration

If no config file exists, owt uses these defaults:

```toml
editor = "$EDITOR or vim"
terminal = "$TERMINAL or Terminal"
worktree_root = "~/.owt/worktree"
copy_files = []
run_post_add_script_in_tmux = false
```
