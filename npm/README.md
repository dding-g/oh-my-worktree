# owt (oh-my-worktree)

A TUI tool for managing Git worktrees from either regular repositories or bare `.bare` layouts.

Run it from a `.bare` worktree layout or from a regular non-bare Git repository. For regular repositories, new worktrees are created under `~/.owt/worktree/<repo-name>/` by default.

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
# Run directly inside an existing regular Git repository
cd /path/to/regular-git-repo
owt

# Or clone into the .bare sibling layout
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

## Adding Worktrees

Press `a`, type the new branch name, and press `Enter`. Use `Tab` in the add dialog to cycle the base branch for the new worktree.

## Status Icons

| Icon | Meaning |
|------|---------|
| `✓` | Clean |
| `+` | Staged changes |
| `~` | Unstaged changes |
| `!` | Conflicts |
| `*` | Staged + Unstaged |

The worktree list also includes a `PR` column for GitHub pull request state. It shows only `open`, `closed`, `merged`, or `draft`; branches without a GitHub PR, non-GitHub remotes, lookup failures, and unknown states show `-`.

## Configuration

Config file: `~/.config/owt/config.toml`

```toml
editor = "code"
terminal = "Ghostty"
worktree_root = "~/.owt/worktree"
copy_files = [".env", ".envrc"]
```

## Requirements

- Git 2.5+ (for worktree support)
- A Git repository

## More Information

For more details, visit the [GitHub repository](https://github.com/dding-g/oh-my-worktree).
