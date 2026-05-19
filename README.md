# owt (oh-my-worktree)

[한국어](./README.ko.md) | [English](./README.md)

A TUI tool for managing Git worktrees from either regular repositories or bare `.bare` layouts.

**[GitHub](https://github.com/dding-g/oh-my-worktree)**

<img width="786" height="580" alt="Image" src="./owt.png" />

## What is Git Worktree?

Git worktree allows you to check out multiple branches simultaneously from a single repository. Work on multiple tasks in parallel without stashing or switching branches.

Regular repository layout:

```
repo/                       # existing non-bare repository
└── .git/

~/.owt/worktree/repo/
├── feature-auth/           # new worktree created by owt
└── hotfix-payment/         # another worktree
```

Bare `.bare` layout:

```
project/
├── .bare/                # bare repository (hidden)
├── main/                 # main branch worktree
├── feature-auth/         # feature branch worktree
└── hotfix-payment/       # hotfix branch worktree
```

**owt** makes this workflow effortless with a simple TUI.

Run `owt` directly inside an existing regular Git repository, or use `owt clone` to create the project-local `.bare` sibling layout. In regular repositories, new worktrees are created under `~/.owt/worktree/<repo-name>/` by default unless you configure another `worktree_root`.

## Commands

| Command | Description |
|---------|-------------|
| `owt` | Launch TUI (default) |
| `owt clone <URL> [PATH]` | Clone into the `.bare` layout + create the first worktree |
| `owt init` | Show a guide for converting an existing repo to the `.bare` layout |
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

### Existing Regular Repository

Start where you already work. No conversion is required.

```bash
cd /path/to/regular-git-repo
owt
```

When you create a worktree from a regular repository, `owt` creates it under `~/.owt/worktree/<repo-name>/` by default. Configure `worktree_root` if you want a different location.

### New Project with `.bare`

```bash
# Clone into the .bare sibling layout + create the first worktree automatically
owt clone https://github.com/user/repo.git

# Run TUI
cd repo/main
owt
```

### Optional: Convert Existing Project to `.bare`

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

The worktree list also includes a `PR` column for GitHub pull request state. It shows only `open`, `closed`, `merged`, or `draft`; branches without a GitHub PR, non-GitHub remotes, lookup failures, and unknown states show `-`.

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
- A regular Git repository or a bare repository layout

## License

MIT
