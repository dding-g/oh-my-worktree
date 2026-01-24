# owt (oh-my-worktree)

A TUI tool for managing Git worktrees in bare repositories.

## Why?

Git worktrees allow you to work on multiple branches simultaneously without stashing or switching. Combined with bare repositories, this creates a powerful workflow:

```
project.git/              # bare repository (metadata only)
├── main/                 # main branch worktree
├── feature-auth/         # feature branch worktree
└── hotfix-payment/       # hotfix branch worktree
```

**owt** makes this workflow effortless with a simple TUI dashboard.

## Features

- **Dashboard view** - See all worktrees with their status at a glance
- **Quick actions** - Create, delete, open worktrees with single keystrokes
- **Smart detection** - Run `owt` from any worktree, it finds the bare repo automatically
- **Clone as bare** - `owt clone <url>` sets up bare repo + first worktree in one command
- **Editor/Terminal integration** - Open worktrees in your preferred editor or terminal

## Installation

### Using npm (Recommended)

```bash
npm install -g oh-my-worktree
```

Or use with npx (no installation required):

```bash
npx oh-my-worktree
```

### Using Cargo (requires Rust)

```bash
cargo install --git https://github.com/dding-g/oh-my-worktree
```

Or build from source:

```bash
git clone https://github.com/mattew8/oh-my-worktree.git
cd oh-my-worktree
cargo build --release
# Binary at ./target/release/owt
```

### PATH setup

For cargo installation, ensure `~/.cargo/bin` is in your PATH:

```bash
# Add to ~/.zshrc or ~/.bashrc
export PATH="$HOME/.cargo/bin:$PATH"
```

## Usage

### TUI Mode (default)

```bash
# Run from bare repo or any worktree
owt

# Or specify path
owt /path/to/repo.git
```

### Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `a` | Add new worktree |
| `d` | Delete worktree |
| `o` | Open in editor |
| `t` | Open in terminal |
| `f` | Fetch all remotes |
| `r` | Refresh list |
| `q` | Quit |

### Subcommands

#### Clone as bare repository

```bash
owt clone https://github.com/user/repo.git
```

This creates:
- `repo.git/` - bare repository
- `repo/main/` - first worktree (default branch)

#### Initialize guide

```bash
owt init
```

Shows step-by-step guide to convert an existing regular repository to bare + worktree setup.

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `EDITOR` | Editor to open worktrees | `vim` |
| `TERMINAL` | Terminal app (macOS) | `Terminal` |

Example:

```bash
export EDITOR=code
export TERMINAL=Ghostty
```

## Quick Start

### New project

```bash
owt clone https://github.com/user/repo.git
cd repo.git
owt
```

### Existing project

```bash
# Manual conversion
mv .git ../myproject.git
cd ../myproject.git
git config --bool core.bare true
git worktree add ../myproject/main main
owt
```

Or run `owt init` for guided instructions.

## Status Icons

| Icon | Meaning |
|------|---------|
| `✓` | Clean (no changes) |
| `+` | Staged changes |
| `~` | Unstaged changes |
| `!` | Conflicts |
| `*` | Mixed (staged + unstaged) |

## Requirements

- Git 2.5+ (worktree support)
- A bare repository with worktrees

## Architecture

```mermaid
flowchart TB
    subgraph Entry["Entry Point"]
        START([owt command]) --> PARSE[Parse CLI Args]
        PARSE --> |"--help"| HELP[Print Help]
        PARSE --> |"--version"| VERSION[Print Version]
        PARSE --> |"clone URL"| CLONE[Clone as Bare]
        PARSE --> |"init"| INIT[Show Convert Guide]
        PARSE --> |"default/path"| TUI[Start TUI]
    end

    subgraph BareDetection["Bare Repo Detection"]
        TUI --> CHECK_BARE{".bare folder exists?"}
        CHECK_BARE --> |Yes| USE_BARE[Use .bare path]
        CHECK_BARE --> |No| CHECK_GIT{Is git repo?}
        CHECK_GIT --> |No| ERR_NOT_GIT[Error: Not a git repo]
        CHECK_GIT --> |Yes| GET_COMMON[git rev-parse --git-common-dir]
        GET_COMMON --> IS_BARE{Is bare repo?}
        IS_BARE --> |No| ERR_NOT_BARE[Error: Not bare repo]
        IS_BARE --> |Yes| USE_COMMON[Use common dir]
        USE_BARE --> INIT_APP
        USE_COMMON --> INIT_APP
    end

    subgraph AppInit["App Initialization"]
        INIT_APP[Initialize App] --> LOAD_WT[Load Worktrees]
        LOAD_WT --> LOAD_CFG[Load Config]
        LOAD_CFG --> DETECT_CUR[Detect Current Worktree]
        DETECT_CUR --> MAIN_LOOP[Enter Main Loop]
    end

    subgraph MainLoop["Main Event Loop"]
        MAIN_LOOP --> RENDER[Render UI]
        RENDER --> CHECK_ASYNC{Async op running?}
        CHECK_ASYNC --> |is_fetching| DO_FETCH[Execute Fetch]
        CHECK_ASYNC --> |is_adding| DO_ADD[Execute Add]
        CHECK_ASYNC --> |is_deleting| DO_DEL[Execute Delete]
        CHECK_ASYNC --> |None| WAIT_EVENT[Wait for Event]
        DO_FETCH --> MAIN_LOOP
        DO_ADD --> MAIN_LOOP
        DO_DEL --> MAIN_LOOP
        WAIT_EVENT --> HANDLE_KEY[Handle Key Event]
        HANDLE_KEY --> MAIN_LOOP
    end

    subgraph States["App States"]
        HANDLE_KEY --> |"state=List"| LIST_INPUT[List Input Handler]
        HANDLE_KEY --> |"state=AddModal"| ADD_INPUT[Add Modal Handler]
        HANDLE_KEY --> |"state=ConfirmDelete"| DEL_INPUT[Delete Confirm Handler]

        LIST_INPUT --> |j/k/↑/↓| NAV[Navigate]
        LIST_INPUT --> |Enter| ENTER_WT[Enter Worktree]
        LIST_INPUT --> |a| OPEN_ADD[Open Add Modal]
        LIST_INPUT --> |d| OPEN_DEL[Open Delete Modal]
        LIST_INPUT --> |o| OPEN_EDIT[Open Editor]
        LIST_INPUT --> |t| OPEN_TERM[Open Terminal]
        LIST_INPUT --> |f| START_FETCH[Start Fetch]
        LIST_INPUT --> |r| REFRESH[Refresh List]
        LIST_INPUT --> |c| OPEN_CFG[Open Config Modal]
        LIST_INPUT --> |q| QUIT[Quit App]

        ADD_INPUT --> |Esc| CANCEL_ADD[Cancel → List]
        ADD_INPUT --> |Enter| START_ADD[Start Add Worktree]
        ADD_INPUT --> |Char| TYPE_CHAR[Append to Buffer]
        ADD_INPUT --> |Backspace| DEL_CHAR[Delete from Buffer]

        DEL_INPUT --> |y/Enter| CONFIRM_DEL[Confirm Delete]
        DEL_INPUT --> |n/Esc| CANCEL_DEL[Cancel → List]
        DEL_INPUT --> |b| TOGGLE_BRANCH[Toggle Delete Branch]
    end

    subgraph Exit["Exit Handling"]
        ENTER_WT --> SET_EXIT[Set ExitAction::ChangeDirectory]
        SET_EXIT --> QUIT_APP[should_quit = true]
        QUIT --> QUIT_APP
        QUIT_APP --> EXIT_LOOP[Exit Main Loop]
        EXIT_LOOP --> CHECK_ACTION{Exit Action?}
        CHECK_ACTION --> |ChangeDirectory| PRINT_PATH[Print worktree path]
        CHECK_ACTION --> |Quit| END_APP[End]
        PRINT_PATH --> END_APP
    end
```

## License

MIT
