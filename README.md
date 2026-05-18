# owt (oh-my-worktree)

[한국어](./README.ko.md) | [English](./README.md)

A TUI tool for managing Git worktrees in bare and regular repositories.

**[GitHub](https://github.com/dding-g/oh-my-worktree)**

<img width="786" height="580" alt="Image" src="./owt.png" />

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

You can also run `owt` inside a regular non-bare Git repository. New worktrees are created under `~/.owt/worktree/<repo-name>/` by default unless you configure another root.

## Commands

| Command | Description |
|---------|-------------|
| `owt` | Launch TUI (default) |
| `owt clone <URL> [PATH]` | Clone as bare repo + create first worktree |
| `owt init` | Guide to convert existing repo to bare structure |
| `owt setup` | Install shell integration |

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
cargo install --git https://github.com/dding-g/oh-my-worktree
```

Build from source:

```bash
git clone https://github.com/dding-g/oh-my-worktree.git
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

You can run `owt` directly inside an existing regular Git repository. You do not need to convert it to a bare repository first.

```bash
cd /path/to/regular-git-repo
owt
```

When you create a worktree from a regular repository, `owt` creates it under `~/.owt/worktree/<repo-name>/` by default. Configure `worktree_root` if you want a different location.

If you prefer the `.bare` sibling layout, run:

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
| `p` | Pull from remote |
| `P` | Push to remote |
| `m` | Merge upstream |
| `M` | Merge branch (select) |
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

Config file: `~/.config/owt/config.toml`

```toml
editor = "code"
terminal = "Ghostty"
worktree_root = "~/.owt/worktree"
copy_files = [".env", ".envrc"]

# Disabled by default. When enabled, .owt/post-add.sh is launched in a
# detached tmux session after worktree creation and the session exits when
# the script completes. There is no direct-shell fallback.
run_post_add_script_in_tmux = false
```

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
