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

## Usage

```bash
# Run in a bare repository or worktree directory
owt

# Clone a repository as bare with first worktree
owt clone <url>

# Show guide for converting existing repo to bare
owt init
```

## Key Bindings

| Key | Action |
|-----|--------|
| `↑/k` | Previous item |
| `↓/j` | Next item |
| `a` | Add new worktree |
| `d` | Delete worktree |
| `o` | Open in editor |
| `t` | Open terminal |
| `f` | Fetch all |
| `r` | Refresh list |
| `q` | Quit |

## Requirements

- Git 2.5+ (for worktree support)
- A bare Git repository

## More Information

For more details, visit the [GitHub repository](https://github.com/dding-g/oh-my-worktree).
