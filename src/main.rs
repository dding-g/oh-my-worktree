mod app;
mod config;
mod git;
mod types;
mod ui;

use anyhow::Result;
use std::env;
use std::path::PathBuf;

enum Command {
    Tui { path: PathBuf },
    Clone { url: String, path: Option<PathBuf> },
    Init,
    Setup,
    Help,
    Version,
    TestCd, // Test command for debugging cd functionality
}

#[cfg(test)]
impl Command {
    fn tui_path(&self) -> Option<&std::path::Path> {
        match self {
            Command::Tui { path } => Some(path.as_path()),
            _ => None,
        }
    }
}

const SHELL_FUNCTION: &str = r#"
# owt shell integration - enables 'Enter' key to change directory
owt() {
  local output_file
  output_file=$(mktemp) || return

  OWT_OUTPUT_FILE="$output_file" command owt "$@"
  local exit_code=$?

  if [ -f "$output_file" ]; then
    local target
    target=$(cat "$output_file")
    rm -f "$output_file"

    if [ -n "$target" ] && [ -d "$target" ]; then
      cd "$target" || return
    fi
  fi

  return $exit_code
}
"#;

fn main() -> Result<()> {
    match parse_args() {
        Command::Help => {
            print_help();
            Ok(())
        }
        Command::Version => {
            println!("owt v{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::Clone { url, path } => run_clone(&url, path),
        Command::Init => run_init(),
        Command::Setup => run_setup(),
        Command::Tui { path } => run_tui(path),
        Command::TestCd => run_test_cd(),
    }
}

fn run_tui(path: PathBuf) -> Result<()> {
    use std::fs::File;
    use std::io::Write;

    // Check if we should write result to a file (for shell integration)
    let output_file = env::var("OWT_OUTPUT_FILE").ok();

    let (repo_path, project_root_path, repo_is_bare) =
        if let Some(bare_path) = git::find_bare_in_parent(&path) {
            let project_root = bare_path
                .parent()
                .map(PathBuf::from)
                .unwrap_or_else(|| path.clone());
            (bare_path, project_root, true)
        } else if git::is_git_repo(&path) {
            let common_dir = git::get_git_common_dir(&path)?;

            if git::is_bare_repo(&common_dir)? {
                let project_root = common_dir
                    .parent()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| path.clone());
                (common_dir, project_root, true)
            } else {
                let worktree_root = git::get_worktree_root(&path)?;
                (worktree_root.clone(), worktree_root, false)
            }
        } else {
            print_not_git_repo_error();
            std::process::exit(1);
        };

    // Always use /dev/tty for TUI to support shell integration
    let tty = File::options().read(true).write(true).open("/dev/tty")?;
    let mut tty_for_control = tty.try_clone()?;

    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(tty_for_control, crossterm::terminal::EnterAlternateScreen)?;

    let backend = ratatui::backend::CrosstermBackend::new(tty);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let has_shell_integration = output_file.is_some();
    let mut app = app::App::new(
        repo_path,
        project_root_path,
        repo_is_bare,
        Some(path),
        has_shell_integration,
    )?;
    let result = app.run(&mut terminal);

    // Restore terminal
    crossterm::execute!(tty_for_control, crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;

    // Handle exit action - write path for shell integration
    match &app.exit_action {
        types::ExitAction::ChangeDirectory(worktree_path) => {
            if let Some(ref output_path) = output_file {
                let mut file = open_shell_output_file(output_path)?;
                writeln!(file, "{}", worktree_path.display())?;
                // Log for debugging
                eprintln!("→ {}", worktree_path.display());
            } else {
                // No shell integration - just print the path
                // This happens when running the binary directly without the shell function
                eprintln!("To enable directory changing, run: owt setup");
                println!("{}", worktree_path.display());
            }
        }
        types::ExitAction::Quit => {
            // Normal quit, no directory change
        }
    }

    result
}

fn run_test_cd() -> Result<()> {
    use std::io::Write;

    // This tests the cd functionality without TUI
    let output_file = env::var("OWT_OUTPUT_FILE").ok();
    let test_path = env::current_dir()?;

    eprintln!("Testing cd functionality...");
    eprintln!("OWT_OUTPUT_FILE: {:?}", output_file);
    eprintln!("Test path: {}", test_path.display());

    if let Some(ref output_path) = output_file {
        eprintln!("Writing to: {}", output_path);
        let mut file = open_shell_output_file(output_path)?;
        writeln!(file, "{}", test_path.display())?;
        eprintln!("Write successful!");
    } else {
        eprintln!("No OWT_OUTPUT_FILE set - printing to stdout");
        println!("{}", test_path.display());
    }

    Ok(())
}

fn open_shell_output_file(output_path: &str) -> Result<std::fs::File> {
    let metadata = std::fs::symlink_metadata(output_path)?;
    let file_type = metadata.file_type();

    if file_type.is_symlink() || !file_type.is_file() {
        anyhow::bail!("OWT_OUTPUT_FILE must point to an existing regular file");
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        if metadata.permissions().mode() & 0o077 != 0 {
            anyhow::bail!("OWT_OUTPUT_FILE must not be group/world accessible");
        }
    }

    Ok(std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(output_path)?)
}

fn run_clone(url: &str, target_path: Option<PathBuf>) -> Result<()> {
    // Extract repo name from URL
    let repo_name = extract_repo_name(url);

    // Determine paths
    let base_dir =
        target_path.unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let project_dir = base_dir.join(&repo_name);
    let bare_repo_path = project_dir.join(".bare");
    let worktree_path = project_dir.join("main");

    println!("Cloning {} as bare repository...", url);

    // Clone as bare
    git::clone_bare(url, &bare_repo_path)?;
    println!("  Created bare repo: {}", bare_repo_path.display());

    // Get default branch
    let default_branch =
        git::get_default_branch(&bare_repo_path).unwrap_or_else(|_| "main".to_string());

    // Create first worktree
    println!("Creating worktree for '{}'...", default_branch);
    git::add_worktree(&bare_repo_path, &default_branch, &worktree_path, None)?;
    println!("  Created worktree: {}", worktree_path.display());

    println!("\nDone! To start using owt:");
    println!("  cd {}", project_dir.display());
    println!("  owt");

    Ok(())
}

fn run_init() -> Result<()> {
    let current_dir = env::current_dir()?;

    // Check if already a bare repo
    if git::is_bare_repo(&current_dir)? {
        println!("Already a bare repository. Run 'owt' to start.");
        return Ok(());
    }

    // Check if it's a git repo
    if !git::is_git_repo(&current_dir) {
        eprintln!("Error: Not a git repository");
        std::process::exit(1);
    }

    // Check if it's inside a worktree
    let common_dir = git::get_git_common_dir(&current_dir)?;
    if git::is_bare_repo(&common_dir)? {
        println!("This is a worktree of a bare repository.");
        println!("Bare repo: {}", common_dir.display());
        println!("\nRun 'owt' to manage worktrees.");
        return Ok(());
    }

    // It's a regular repo - show conversion guide
    let repo_name = current_dir
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "myproject".to_string());

    println!("This is a regular git repository.");
    println!("\nTo convert to bare repository + worktree setup:\n");
    println!("  # 1. Go to parent directory");
    println!("  cd ..\n");
    println!("  # 2. Move .git to new bare repo");
    println!("  mv {}/.git {}.git", repo_name, repo_name);
    println!("  rm -rf {}\n", repo_name);
    println!("  # 3. Configure as bare");
    println!("  cd {}.git", repo_name);
    println!("  git config --bool core.bare true\n");
    println!("  # 4. Create first worktree");
    println!("  git worktree add ../{}/main main\n", repo_name);
    println!("  # 5. Run owt");
    println!("  owt");

    Ok(())
}

fn run_setup() -> Result<()> {
    use std::fs;
    use std::io::{self, Write};

    // Detect shell from SHELL environment variable
    let shell = env::var("SHELL").unwrap_or_default();
    let home = env::var("HOME").ok().map(PathBuf::from);
    let (shell_name, config_file) = if shell.contains("zsh") {
        ("zsh", home.map(|h| h.join(".zshrc")))
    } else if shell.contains("bash") {
        ("bash", home.map(|h| h.join(".bashrc")))
    } else {
        ("unknown", None)
    };

    let config_path = match config_file {
        Some(path) => path,
        None => {
            eprintln!("Error: Could not detect shell config file.");
            eprintln!("Please manually add the following to your shell config:\n");
            println!("{}", SHELL_FUNCTION);
            return Ok(());
        }
    };

    println!("Detected shell: {}", shell_name);
    println!("Config file: {}", config_path.display());

    // Check if function already exists
    if config_path.exists() {
        let content = fs::read_to_string(&config_path)?;
        if content.contains("owt()") || content.contains("owt ()") {
            println!("\n✓ Shell integration already installed!");
            println!(
                "  If it's not working, try: source {}",
                config_path.display()
            );
            return Ok(());
        }
    }

    // Check if config file is a symlink (e.g., managed by Nix/home-manager)
    let is_symlink = fs::symlink_metadata(&config_path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);

    if is_symlink {
        println!(
            "\n⚠ {} is a symlink (possibly managed by Nix/home-manager)",
            config_path.display()
        );
        println!("  Cannot modify directly.\n");
        println!("Add this to your shell configuration manually:\n");
        println!("{}", SHELL_FUNCTION);

        // Suggest alternative
        if shell_name == "zsh" {
            println!("\nAlternatively, create ~/.zshrc.local and source it from your config:");
            println!("  echo 'source ~/.zshrc.local' # add to your home-manager zsh config");
        }
        return Ok(());
    }

    // Ask for confirmation
    print!(
        "\nAdd owt shell integration to {}? [Y/n] ",
        config_path.display()
    );
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input == "n" || input == "no" {
        println!("Aborted. You can manually add this to your shell config:\n");
        println!("{}", SHELL_FUNCTION);
        return Ok(());
    }

    // Append to config file
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&config_path)?;

    writeln!(file, "{}", SHELL_FUNCTION)?;

    println!("\n✓ Shell integration installed!");
    println!("\nTo activate, run:");
    println!("  source {}", config_path.display());
    println!("\nOr restart your terminal.");

    Ok(())
}

fn extract_repo_name(url: &str) -> String {
    // Handle various URL formats:
    // https://github.com/user/repo.git
    // git@github.com:user/repo.git
    // /path/to/repo.git
    // repo.git

    let url = url.trim_end_matches('/');
    let name = url
        .rsplit('/')
        .next()
        .or_else(|| url.rsplit(':').next())
        .unwrap_or(url);

    name.trim_end_matches(".git").to_string()
}

fn parse_args() -> Command {
    let args: Vec<String> = env::args().collect();
    parse_args_from(args, || {
        env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    })
}

fn parse_args_from(args: Vec<String>, current_dir: impl Fn() -> PathBuf) -> Command {
    let current_dir = &current_dir;

    if args.len() < 2 {
        return Command::Tui {
            path: current_dir(),
        };
    }

    match args[1].as_str() {
        "--help" | "-h" | "help" => Command::Help,
        "--version" | "-v" => Command::Version,
        "clone" => {
            if args.len() < 3 {
                eprintln!("Error: clone requires a URL argument");
                eprintln!("Usage: owt clone <url> [path]");
                std::process::exit(1);
            }
            let url = args[2].clone();
            let path = args.get(3).map(PathBuf::from);
            Command::Clone { url, path }
        }
        "init" => Command::Init,
        "setup" => Command::Setup,
        "test-cd" | "--test-cd" => Command::TestCd,
        arg if arg.starts_with('-') => {
            // Handle flags for TUI mode
            let mut path = current_dir();
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
                    _ => i += 1,
                }
            }
            Command::Tui { path }
        }
        _ => {
            // Treat as path for TUI mode
            Command::Tui {
                path: PathBuf::from(&args[1]),
            }
        }
    }
}

fn print_help() {
    println!(
        r#"owt - Git Worktree Manager

USAGE:
    owt [OPTIONS] [PATH]         Start TUI (default)
    owt clone <URL> [PATH]       Clone as bare repo + create main worktree
    owt init                     Show guide to convert regular repo to bare
    owt setup                    Install shell integration for directory changing

ARGS:
    [PATH]    Path to a Git repository or worktree (default: current directory)

OPTIONS:
    -p, --path <PATH>    Path to a Git repository or worktree
    -h, --help           Print help information
    -v, --version        Print version information

SUBCOMMANDS:
    clone <URL> [PATH]   Clone repository as bare and create first worktree
    init                 Show conversion guide for regular repositories
    setup                Install shell integration (adds function to .zshrc/.bashrc)

KEYBINDINGS (TUI):
    Enter       Enter worktree (cd to directory)
    j/k, ↑/↓    Navigate worktrees
    a           Add new worktree
    d           Delete selected worktree
    o           Open in editor ($EDITOR)
    t           Open in terminal ($TERMINAL)
    f           Fetch all remotes
    r           Refresh worktree list
    c           View config settings
    q           Quit

ENVIRONMENT:
    EDITOR      Editor to use (default: vim)
    TERMINAL    Terminal app to use (default: Terminal.app on macOS)

SHELL INTEGRATION:
    Run `owt setup` to install the secure OWT_OUTPUT_FILE shell integration.
    The TUI draws through /dev/tty so stdout can remain reserved for cd handoff.

EXAMPLES:
    owt clone https://github.com/user/repo.git
    owt clone git@github.com:user/repo.git ~/projects
    owt init
    owt --path ~/repos/myproject.git"#
    );
}

fn print_not_git_repo_error() {
    eprintln!(
        r#"Error: Not a git repository

The current directory is not a git repository.
Please navigate to a git repository or specify the path with --path."#
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::process::{Command as ProcessCommand, Output};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let id = std::process::id();
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("owt_main_unit_{}_{}_{}", name, id, ts));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn git_cmd() -> ProcessCommand {
        let mut cmd = ProcessCommand::new("git");
        cmd.env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_COMMON_DIR");
        cmd
    }

    fn assert_git_success(output: Output, context: &str) {
        assert!(
            output.status.success(),
            "{}: {}",
            context,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn create_source_repo(path: &Path) {
        fs::create_dir_all(path).unwrap();
        assert_git_success(
            git_cmd()
                .current_dir(path)
                .args(["init", "-b", "main"])
                .output()
                .unwrap(),
            "git init failed",
        );
        assert_git_success(
            git_cmd()
                .current_dir(path)
                .args(["config", "user.email", "test@test.com"])
                .output()
                .unwrap(),
            "git config user.email failed",
        );
        assert_git_success(
            git_cmd()
                .current_dir(path)
                .args(["config", "user.name", "Test"])
                .output()
                .unwrap(),
            "git config user.name failed",
        );
        fs::write(path.join("README.md"), "# Test\n").unwrap();
        assert_git_success(
            git_cmd()
                .current_dir(path)
                .args(["add", "."])
                .output()
                .unwrap(),
            "git add failed",
        );
        assert_git_success(
            git_cmd()
                .current_dir(path)
                .args(["commit", "-m", "initial"])
                .output()
                .unwrap(),
            "git commit failed",
        );
    }

    #[test]
    fn parse_args_defaults_to_current_directory_tui() {
        let command = parse_args_from(vec!["owt".to_string()], || PathBuf::from("/repo/main"));

        assert_eq!(command.tui_path(), Some(Path::new("/repo/main")));
    }

    #[test]
    fn parse_args_accepts_path_flag_and_positional_path() {
        let flag_command = parse_args_from(
            vec![
                "owt".to_string(),
                "--path".to_string(),
                "/tmp/repo".to_string(),
            ],
            || PathBuf::from("/repo/main"),
        );
        let positional_command =
            parse_args_from(vec!["owt".to_string(), "/tmp/other".to_string()], || {
                PathBuf::from("/repo/main")
            });

        assert_eq!(flag_command.tui_path(), Some(Path::new("/tmp/repo")));
        assert_eq!(positional_command.tui_path(), Some(Path::new("/tmp/other")));
    }

    #[test]
    fn parse_args_recognizes_documented_subcommands() {
        assert!(matches!(
            parse_args_from(vec!["owt".to_string(), "init".to_string()], PathBuf::new),
            Command::Init
        ));
        assert!(matches!(
            parse_args_from(vec!["owt".to_string(), "setup".to_string()], PathBuf::new),
            Command::Setup
        ));
        assert!(matches!(
            parse_args_from(vec!["owt".to_string(), "test-cd".to_string()], PathBuf::new),
            Command::TestCd
        ));
        assert!(matches!(
            parse_args_from(
                vec!["owt".to_string(), "--version".to_string()],
                PathBuf::new
            ),
            Command::Version
        ));
        assert!(matches!(
            parse_args_from(vec!["owt".to_string(), "help".to_string()], PathBuf::new),
            Command::Help
        ));
        assert!(matches!(
            parse_args_from(
                vec![
                    "owt".to_string(),
                    "clone".to_string(),
                    "https://example.com/repo.git".to_string(),
                    "/tmp/projects".to_string()
                ],
                PathBuf::new
            ),
            Command::Clone { url, path }
                if url == "https://example.com/repo.git" && path == Some(PathBuf::from("/tmp/projects"))
        ));
    }

    #[test]
    fn extract_repo_name_handles_documented_url_forms() {
        let cases = [
            ("https://github.com/user/repo.git", "repo"),
            ("git@github.com:user/repo.git", "repo"),
            ("https://github.com/user/repo", "repo"),
            ("repo.git", "repo"),
            ("/path/to/repo.git", "repo"),
            ("https://github.com/user/repo.git/", "repo"),
        ];

        for (url, expected) in cases {
            assert_eq!(extract_repo_name(url), expected, "failed for {url}");
        }
    }

    #[test]
    fn run_clone_creates_dot_bare_layout_and_first_worktree() {
        let base = temp_dir("run_clone_layout");
        let source = base.join("source-repo");
        let target_parent = base.join("projects");
        create_source_repo(&source);

        run_clone(&source.to_string_lossy(), Some(target_parent.clone())).unwrap();

        let project_dir = target_parent.join("source-repo");
        assert!(project_dir.join(".bare").is_dir());
        assert!(project_dir.join("main").is_dir());
        assert!(git::is_bare_repo(&project_dir.join(".bare")).unwrap());
        assert!(git::is_git_repo(&project_dir.join("main")));

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn shell_function_uses_output_file_and_cd_guard() {
        assert!(SHELL_FUNCTION.contains("OWT_OUTPUT_FILE=\"$output_file\" command owt \"$@\""));
        assert!(SHELL_FUNCTION.contains("target=$(cat \"$output_file\")"));
        assert!(SHELL_FUNCTION.contains("[ -n \"$target\" ] && [ -d \"$target\" ]"));
        assert!(SHELL_FUNCTION.contains("cd \"$target\""));
        assert!(SHELL_FUNCTION.contains("return $exit_code"));
    }

    #[test]
    fn open_shell_output_file_accepts_private_regular_file_and_truncates() {
        let base = temp_dir("output_file_regular");
        let path = base.join("owt-output");
        fs::write(&path, "stale target").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        }

        let mut file = open_shell_output_file(path.to_str().unwrap()).unwrap();
        writeln!(file, "/tmp/new-target").unwrap();
        drop(file);

        assert_eq!(fs::read_to_string(&path).unwrap(), "/tmp/new-target\n");
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn open_shell_output_file_rejects_non_file() {
        let base = temp_dir("output_file_directory");

        assert!(open_shell_output_file(base.to_str().unwrap()).is_err());

        let _ = fs::remove_dir_all(base);
    }

    #[cfg(unix)]
    #[test]
    fn open_shell_output_file_rejects_symlink_and_group_accessible_file() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let base = temp_dir("output_file_security");
        let target = base.join("target");
        let link = base.join("link");
        fs::write(&target, "").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
        symlink(&target, &link).unwrap();

        assert!(open_shell_output_file(link.to_str().unwrap()).is_err());

        fs::set_permissions(&target, fs::Permissions::from_mode(0o640)).unwrap();
        assert!(open_shell_output_file(target.to_str().unwrap()).is_err());

        let _ = fs::remove_dir_all(base);
    }
}
