mod app;
mod git;
mod types;
mod ui;

use anyhow::Result;
use std::env;
use std::path::PathBuf;

fn main() -> Result<()> {
    let path = parse_args();

    // Check if we're in a git repository
    if !git::is_git_repo(&path) {
        print_not_git_repo_error();
        std::process::exit(1);
    }

    // Get the common git directory (works for both bare repos and worktrees)
    let common_dir = git::get_git_common_dir(&path)?;

    // Check if the common dir is a bare repository
    if !git::is_bare_repo(&common_dir)? {
        print_not_bare_repo_error();
        std::process::exit(1);
    }

    // Initialize and run the TUI
    let mut terminal = ratatui::init();
    let result = app::App::new(common_dir)?.run(&mut terminal);
    ratatui::restore();

    result
}

fn parse_args() -> PathBuf {
    let args: Vec<String> = env::args().collect();
    let mut path = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--path" | "-p" => {
                if i + 1 < args.len() {
                    path = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    eprintln!("Error: --path requires an argument");
                    std::process::exit(1);
                }
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            "--version" | "-v" => {
                println!("owt v0.1.0");
                std::process::exit(0);
            }
            arg if arg.starts_with('-') => {
                eprintln!("Error: Unknown option: {}", arg);
                std::process::exit(1);
            }
            _ => {
                // Treat as path if no flag
                path = PathBuf::from(&args[i]);
                i += 1;
            }
        }
    }

    path
}

fn print_help() {
    println!(
        r#"owt - Git Worktree Manager

USAGE:
    owt [OPTIONS] [PATH]

ARGS:
    [PATH]    Path to the bare repository (default: current directory)

OPTIONS:
    -p, --path <PATH>    Path to the bare repository
    -h, --help           Print help information
    -v, --version        Print version information

KEYBINDINGS:
    j/k, ↑/↓    Navigate worktrees
    a           Add new worktree
    d           Delete selected worktree
    o           Open in editor ($EDITOR)
    t           Open in terminal ($TERMINAL)
    f           Fetch all remotes
    r           Refresh worktree list
    q           Quit

ENVIRONMENT:
    EDITOR      Editor to use (default: vim)
    TERMINAL    Terminal app to use (default: Terminal.app on macOS)"#
    );
}

fn print_not_git_repo_error() {
    eprintln!(
        r#"Error: Not a git repository

The current directory is not a git repository.
Please navigate to a git repository or specify the path with --path."#
    );
}

fn print_not_bare_repo_error() {
    eprintln!(
        r#"Error: Not a bare repository

owt requires a bare repository with worktrees.
To convert your existing repository:

  1. Move .git to a new location:
     mv .git ../myproject.git

  2. Configure as bare:
     cd ../myproject.git
     git config --bool core.bare true

  3. Add your first worktree:
     git worktree add ../myproject/main main

  4. Run owt:
     owt

Or clone a new project as bare:
  git clone --bare <url> myproject.git
  cd myproject.git
  git worktree add ../myproject/main main"#
    );
}
