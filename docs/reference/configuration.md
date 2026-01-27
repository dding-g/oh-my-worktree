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

# Files to copy when creating a new worktree
copy_files = [".env", ".env.local"]

# Branch type shortcuts
[[branch_types]]
name = "feature"
prefix = "feature/"
base = "main"
shortcut = "f"

[[branch_types]]
name = "bugfix"
prefix = "bugfix/"
base = "main"
shortcut = "b"

[[branch_types]]
name = "hotfix"
prefix = "hotfix/"
base = "main"
shortcut = "h"
```

### Options

| Option | Type | Description |
|:-------|:-----|:------------|
| `editor` | string | Editor command to open worktrees |
| `terminal` | string | Terminal app name (macOS) or command (Linux) |
| `copy_files` | array | Files to copy to new worktrees |
| `branch_types` | array | Branch type configurations |

### Branch Type Configuration

Each branch type has:

| Field | Description |
|:------|:------------|
| `name` | Display name |
| `prefix` | Prefix added to branch names |
| `base` | Base branch for new worktrees |
| `shortcut` | Single character shortcut |

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

You can create a project-specific config in `.owt/config.toml` next to your `.bare` folder:

```
project/
├── .bare/
├── .owt/
│   └── config.toml      <- Project config
└── main/
```

Project config overrides global config.

## Post-Add Script

Create `.owt/post-add.sh` to run commands after creating a worktree:

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
copy_files = []

[[branch_types]]
name = "feature"
prefix = "feature/"
base = "main"
shortcut = "f"

[[branch_types]]
name = "bugfix"
prefix = "bugfix/"
base = "main"
shortcut = "b"

[[branch_types]]
name = "hotfix"
prefix = "hotfix/"
base = "main"
shortcut = "h"
```
