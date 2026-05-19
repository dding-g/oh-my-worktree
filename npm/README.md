# owt (oh-my-worktree)

[한국어](./README.ko.md) | [English](./README.md)

A fast terminal UI for developers who use Git branches as working contexts, not bookmarks.

<img width="786" height="580" alt="Image" src="./owt.png" />

## Why owt exists

Modern development rarely happens on one branch at a time. You might be reviewing a PR, testing a hotfix, keeping a long-running feature open, and checking main before a release. Plain `git switch` makes that workflow expensive because every context switch drags along uncommitted files, dependencies, editor state, and mental state.

Git worktrees solve the underlying problem. `owt` makes them easy enough to use every day.

Open `owt`, pick a worktree, create another one, delete the stale ones, fetch, pull, push, or merge without remembering the exact Git incantation. It works from a normal repository and from the `.bare` layout if you prefer keeping all worktrees side by side.

## What you get

- A keyboard-first TUI for browsing and managing worktrees
- First-class support for existing regular repositories
- Optional `.bare` project layout for teams that like sibling worktrees
- Fast worktree creation from local or remote branches
- Dirty-state, ahead/behind, last-commit, and GitHub PR status visibility
- Built-in fetch, pull, push, upstream merge, branch merge, editor open, terminal open, and path copy
- Shell integration so `Enter` can move your shell into the selected worktree

## Install

```bash
npm install -g oh-my-worktree
```

Or run it without installing:

```bash
npx oh-my-worktree
```

From source:

```bash
git clone https://github.com/dding-g/oh-my-worktree.git
cd oh-my-worktree
cargo build --release
```

## Start from the repo you already have

You do not need to convert your repository.

```bash
cd ~/src/my-app
owt
```

In a regular repository, new worktrees are created under:

```text
~/.owt/worktree/<repo-name>/
```

Set `worktree_root` if you want them somewhere else.

## Or start with a `.bare` workspace

If you like all worktrees living inside one project folder, use `owt clone`.

```bash
owt clone https://github.com/user/repo.git
cd repo/main
owt
```

That creates a layout like this:

```text
repo/
├── .bare/
├── main/
├── feature-login/
└── hotfix-api/
```

`owt init` prints a conversion guide if you want to move an existing repository into this layout manually.

## Daily workflow

```bash
owt
```

Then use the TUI:

| Key | Action |
| --- | --- |
| `j` / `k` | Move selection |
| `Enter` | Enter the selected worktree |
| `a` | Add a worktree |
| `d` | Delete a worktree |
| `f` | Fetch remotes |
| `p` / `P` | Pull / push |
| `m` / `M` | Merge upstream / merge selected branch |
| `o` / `t` | Open in editor / terminal |
| `y` | Copy path |
| `/` | Filter |
| `s` | Cycle sort mode |
| `c` | View config |
| `?` | Help |
| `q` | Quit |

## What the list tells you

| Signal | Meaning |
| --- | --- |
| `✓ clean` | No local changes |
| `+ staged` | Staged changes |
| `~ unstaged` | Unstaged changes |
| `! conflict` | Merge conflict |
| `* mixed` | Staged and unstaged changes |
| `↑N` / `↓N` | Ahead / behind upstream |
| `PR` | GitHub PR state: `open`, `closed`, `merged`, `draft`, or `-` |

The `PR` column is GitHub-only and best-effort. No PR, non-GitHub remotes, missing auth, network failures, and unknown states all show `-` so the worktree list stays fast and reliable.

## Shell integration

Install the shell helper:

```bash
owt setup
```

Reload your shell. After that, pressing `Enter` in the TUI exits `owt` and moves the current shell into the selected worktree. Without shell integration, `owt` still prints the selected path for wrapper scripts and manual use.

## Configuration

Config file:

```text
~/.config/owt/config.toml
```

Example:

```toml
editor = "code"
terminal = "Ghostty"
worktree_root = "~/.owt/worktree"
copy_files = [".env", ".envrc"]
run_post_add_script_in_tmux = false
```

Useful options:

| Option | Purpose |
| --- | --- |
| `editor` | Command used by `o` |
| `terminal` | Terminal app used by `t` |
| `worktree_root` | Root for new worktrees in regular repositories |
| `copy_files` | Files copied into new worktrees |
| `run_post_add_script_in_tmux` | Run `.owt/post-add.sh` in detached tmux after creating a worktree |

## Commands

| Command | Purpose |
| --- | --- |
| `owt [PATH]` | Open the TUI for a repository or worktree |
| `owt clone <URL> [PATH]` | Clone into the `.bare` layout and create the first worktree |
| `owt init` | Print a manual conversion guide for `.bare` layout |
| `owt setup` | Install shell integration |
| `owt --version` | Print version |

## Requirements

- Git 2.5+
- A regular Git repository or a `.bare` worktree layout
- Optional: GitHub CLI `gh` for PR status
- Optional: tmux for post-add setup scripts

## License

MIT
