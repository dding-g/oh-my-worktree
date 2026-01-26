# owt (oh-my-worktree)

A TUI tool for managing Git worktrees in bare repositories.

## Installation

```bash
npm install -g oh-my-worktree
```

Or use with npx:

```bash
npx oh-my-worktree
```

## Quick Start

```bash
# Clone a repository as bare with first worktree
owt clone https://github.com/user/repo.git
cd repo/main

# Run TUI
owt
```

## Key Bindings

### Navigation
| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `g` | Jump to current worktree |
| `gg` | Jump to top |
| `G` | Jump to bottom |
| `/` | Search/filter |

### Worktree Actions
| Key | Action |
|-----|--------|
| `Enter` | Enter selected worktree |
| `a` | Add new worktree |
| `d` | Delete worktree |
| `o` | Open in editor |
| `t` | Open in terminal |
| `y` | Copy path to clipboard |

### Other
| Key | Action |
|-----|--------|
| `f` | Fetch remotes |
| `r` | Refresh list |
| `s` | Cycle sort mode |
| `c` | Open config |
| `?` | Show help |
| `q` | Quit |

## Branch Types (v0.4.1+)

When adding a new worktree, select branch type for automatic base branch:

| Key | Type | Base |
|-----|------|------|
| `f` | feature | develop |
| `h` | hotfix | main |
| `r` | release | develop |
| `b` | bugfix | develop |
| `c` | custom | (select manually) |

In branch input screen:
- `Shift+F` - Fetch remote base branch
- `Shift+U` - Use remote as base
- `Shift+L` - Use local as base

## Status Icons

| Icon | Meaning |
|------|---------|
| `✓` | Clean |
| `+` | Staged changes |
| `~` | Unstaged changes |
| `!` | Conflicts |
| `*` | Staged + Unstaged |

## Configuration

Config file: `~/.config/owt/config.toml`

```toml
editor = "code"
terminal = "Ghostty"
copy_files = [".env", ".envrc"]

[[branch_types]]
name = "feature"
prefix = "feature/"
base = "develop"
shortcut = "f"
```

## Requirements

- Git 2.5+ (for worktree support)
- A bare Git repository

## More Information

For more details, visit the [GitHub repository](https://github.com/dding-g/oh-my-worktree).
