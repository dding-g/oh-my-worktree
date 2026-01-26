# owt (oh-my-worktree)

A TUI tool for managing Git worktrees in bare repositories.

<img width="786" height="580" alt="Image" src="https://github.com/user-attachments/assets/929a7bf2-cd66-4a87-a73e-8b9567cb0a08" />

## What is Git Worktree?

Git worktree allows you to check out multiple branches simultaneously from a single repository. Work on multiple tasks in parallel without stashing or switching branches.

```
project/
├── .bare/                # bare repository (hidden)
├── main/                 # main branch worktree
├── feature-auth/         # feature branch worktree
└── hotfix-payment/       # hotfix branch worktree
```

**owt** makes this workflow effortless with a simple TUI.

## Installation

### npm (Recommended)

```bash
npm install -g oh-my-worktree
```

Run without installation using npx:

```bash
npx oh-my-worktree
```

### Cargo

```bash
cargo install --git https://github.com/mattew8/oh-my-worktree
```

Build from source:

```bash
git clone https://github.com/mattew8/oh-my-worktree.git
cd oh-my-worktree
cargo build --release
# Binary: ./target/release/owt
```

## Getting Started

### New Project

```bash
# Clone as bare repo + create first worktree automatically
owt clone https://github.com/user/repo.git

# Run TUI
cd repo/main
owt
```

### Convert Existing Project

```bash
owt init
```

Shows a step-by-step guide to convert a regular repository to bare + worktree structure.

Manual conversion:

```bash
mv .git .bare
echo "gitdir: ./.bare" > .git
git config --bool core.bare true
git worktree add main main
owt
```

## Usage

```bash
# Run from any worktree
owt

# Specify path
owt /path/to/project
```

### Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Enter` | Enter selected worktree |
| `a` | Add new worktree |
| `d` | Delete worktree |
| `o` | Open in editor |
| `t` | Open in terminal |
| `f` | Fetch all remotes |
| `r` | Refresh list |
| `q` | Quit |

### Status Icons

| Icon | Meaning |
|------|---------|
| `✓` | Clean |
| `+` | Staged changes |
| `~` | Unstaged changes |
| `!` | Conflicts |
| `*` | Staged + Unstaged |

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `EDITOR` | Editor to open worktrees | `vim` |
| `TERMINAL` | Terminal app (macOS) | `Terminal` |

```bash
export EDITOR=code
export TERMINAL=Ghostty
```

## Requirements

- Git 2.5+ (worktree support)
- Bare repository

## License

MIT
