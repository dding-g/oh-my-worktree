mod app;
mod config;
mod git;
mod tmux;
mod types;
mod ui;

use anyhow::{Context, Result};
use config::Config;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

enum Command {
    Tui { path: PathBuf },
    Clone { url: String, path: Option<PathBuf> },
    Init,
    Setup,
    Help(HelpTopic),
    Version,
    TestCd, // Test command for debugging cd functionality
    Worktree(WorktreeCommand),
    Pr(PrCommand),
    Commit(CommitCommand),
    Search(SearchCommand),
}

enum HelpTopic {
    Root,
    Worktree,
    WorktreeList,
    WorktreeCreate,
    WorktreeDelete,
    WorktreePrune,
    Pr,
    PrStatus,
    Commit,
    CommitTree,
    Search,
}

enum WorktreeCommand {
    List {
        path: PathBuf,
        include_pr: bool,
    },
    Create {
        path: PathBuf,
        branch: String,
        base: Option<String>,
        worktree_path: Option<PathBuf>,
        tmux: Option<bool>,
    },
    Delete {
        path: PathBuf,
        target: String,
        force: bool,
        delete_branch: bool,
    },
    Prune {
        path: PathBuf,
    },
}

enum PrCommand {
    Status {
        path: PathBuf,
        branch: Option<String>,
        all: bool,
    },
}

enum CommitCommand {
    Tree { path: PathBuf, limit: usize },
}

enum SearchCommand {
    Query {
        path: PathBuf,
        query: String,
        include_pr: bool,
    },
}

struct RepositoryContext {
    repo_path: PathBuf,
    project_root_path: PathBuf,
    repo_is_bare: bool,
}

struct PrunedWorktree {
    branch: String,
    path: PathBuf,
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
        Command::Help(topic) => {
            print_help(topic);
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
        Command::Worktree(command) => run_worktree_command(command),
        Command::Pr(command) => run_pr_command(command),
        Command::Commit(command) => run_commit_command(command),
        Command::Search(command) => run_search_command(command),
    }
}

fn run_tui(path: PathBuf) -> Result<()> {
    use std::fs::File;
    use std::io::Write;

    // Check if we should write result to a file (for shell integration)
    let output_file = env::var("OWT_OUTPUT_FILE").ok();

    let repo_context = match resolve_repository_context(&path) {
        Ok(context) => context,
        Err(_) => {
            print_not_git_repo_error();
            std::process::exit(1);
        }
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
        repo_context.repo_path,
        repo_context.project_root_path,
        repo_context.repo_is_bare,
        Some(path),
        has_shell_integration,
    )?;
    let result = app.run(&mut terminal);

    // Restore terminal
    crossterm::execute!(tty_for_control, crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;

    let exit_action = app.exit_action.clone();

    // Handle exit action - write path for shell integration
    match &exit_action {
        types::ExitAction::ChangeDirectory(worktree_path) => {
            if let Some(ref output_path) = output_file {
                let mut file = open_shell_output_file(output_path)?;
                writeln!(file, "{}", worktree_path.display())?;
                // Log for debugging
                eprintln!("→ {}", worktree_path.display());
            } else {
                // No shell integration - print only the path to keep stdout
                // machine-readable for shell wrappers and scripts.
                println!("{}", worktree_path.display());
            }
        }
        types::ExitAction::Quit => {
            // Normal quit, no directory change
        }
        types::ExitAction::CreateWorktree(request) => {
            run_post_tui_create_worktree(request, &app.config, output_file.as_deref())?;
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

fn run_post_tui_create_worktree(
    request: &types::WorktreeCreateRequest,
    config: &Config,
    output_file: Option<&str>,
) -> Result<()> {
    eprintln!(
        "Creating worktree: {} (base: {})",
        request.branch, request.base_branch
    );

    let base_branch = Some(request.base_branch.as_str());
    let _ = git::fetch_remote_branch(&request.bare_repo_path, &request.base_branch);
    git::add_worktree(
        &request.bare_repo_path,
        &request.branch,
        &request.worktree_path,
        base_branch,
    )?;

    if !config.copy_files.is_empty() {
        if let Some(source) = request.source_path.as_deref() {
            for warning in copy_configured_files(source, &request.worktree_path, &config.copy_files)
            {
                eprintln!("warning\t{}", plain_field(&warning));
            }
        }
    }

    if config.tmux_worktree_mode {
        let worktree_name = worktree_name_from_path(&request.worktree_path);
        match tmux::open_worktree_pane(&request.worktree_path, &worktree_name) {
            Ok(()) => eprintln!("tmux\topened\t{}", plain_field(&worktree_name)),
            Err(error) => eprintln!("warning\ttmux\t{}", plain_field(&error.to_string())),
        }
    }

    if let Err(error) =
        launch_post_add_script(config, &request.project_root_path, &request.worktree_path)
    {
        eprintln!("warning\tpost_add\t{}", plain_field(&error.to_string()));
    }

    write_shell_handoff(output_file, &request.worktree_path)?;
    Ok(())
}

fn write_shell_handoff(output_file: Option<&str>, worktree_path: &Path) -> Result<()> {
    if let Some(output_path) = output_file {
        let mut file = open_shell_output_file(output_path)?;
        use std::io::Write;
        writeln!(file, "{}", worktree_path.display())?;
        eprintln!("→ {}", worktree_path.display());
    } else {
        println!("{}", worktree_path.display());
    }
    Ok(())
}

fn launch_post_add_script(
    config: &Config,
    project_root_path: &Path,
    worktree_path: &Path,
) -> Result<()> {
    let script_path = config.resolved_post_add_script_path(project_root_path);
    if !script_path.exists() || !config.run_post_add_script_in_tmux {
        return Ok(());
    }

    let worktree_name = worktree_name_from_path(worktree_path);
    let session_name = format!("owt-post-add-{}", std::process::id());
    let command = format!(
        "cd {} && sh {}; status=$?; tmux kill-session -t {}; exit $status",
        shell_quote(worktree_path),
        shell_quote(&script_path),
        session_name
    );
    let output = ProcessCommand::new("tmux")
        .args(["new-session", "-d", "-s", &session_name, &command])
        .output()
        .context("Failed to launch post-add script in tmux")?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to launch post-add script in tmux: {}",
            process_failure_detail(&output)
        );
    }

    eprintln!("post_add\tlaunched\t{}", plain_field(&worktree_name));
    Ok(())
}

fn run_worktree_command(command: WorktreeCommand) -> Result<()> {
    match command {
        WorktreeCommand::List { path, include_pr } => {
            let context = resolve_repository_context(&path)?;
            let mut worktrees = git::list_worktrees(&context.repo_path)?;
            if include_pr {
                refresh_pr_statuses(&context.repo_path, &mut worktrees);
            }
            for worktree in &worktrees {
                print_worktree_record(worktree);
            }
            Ok(())
        }
        WorktreeCommand::Create {
            path,
            branch,
            base,
            worktree_path,
            tmux,
        } => {
            let context = resolve_repository_context(&path)?;
            let config =
                Config::load_with_project(Some(&context.project_root_path)).unwrap_or_default();
            let worktrees = git::list_worktrees(&context.repo_path)?;
            let target_path = worktree_path
                .unwrap_or_else(|| worktree_path_for_branch(&context, &config, &branch));

            if let Some(existing) =
                conflicting_worktree_for_branch(&worktrees, &branch, &target_path)
            {
                anyhow::bail!(
                    "Branch '{}' is already checked out at {}",
                    branch,
                    existing.path.display()
                );
            }

            if let Some(base_branch) = base.as_deref() {
                let _ = git::fetch_remote_branch(&context.repo_path, base_branch);
            }

            git::add_worktree(&context.repo_path, &branch, &target_path, base.as_deref())?;

            let tmux_enabled = tmux.unwrap_or(config.tmux_worktree_mode);
            if tmux_enabled {
                let worktree_name = worktree_name_from_path(&target_path);
                match tmux::open_worktree_pane(&target_path, &worktree_name) {
                    Ok(()) => eprintln!("tmux\topened\t{}", plain_field(&worktree_name)),
                    Err(error) => eprintln!("warning\ttmux\t{}", plain_field(&error.to_string())),
                }
            }

            if !config.copy_files.is_empty() {
                let source = current_worktree_path(&worktrees, &path).or_else(|| {
                    worktrees
                        .iter()
                        .find(|wt| !wt.is_bare)
                        .map(|wt| wt.path.clone())
                });
                if let Some(source) = source {
                    for warning in copy_configured_files(&source, &target_path, &config.copy_files)
                    {
                        eprintln!("warning\t{}", plain_field(&warning));
                    }
                }
            }

            println!(
                "created\t{}\t{}",
                plain_field(&branch),
                plain_field(&target_path.display().to_string())
            );
            Ok(())
        }
        WorktreeCommand::Delete {
            path,
            target,
            force,
            delete_branch,
        } => {
            let context = resolve_repository_context(&path)?;
            let worktrees = git::list_worktrees(&context.repo_path)?;
            let worktree = find_worktree_target(&worktrees, &target)?;

            if worktree.is_bare {
                anyhow::bail!("Cannot delete bare repository");
            }
            if worktree.status != types::WorktreeStatus::Clean && !force {
                anyhow::bail!(
                    "Worktree has uncommitted changes. Re-run with --force to delete it."
                );
            }

            git::remove_worktree(&context.repo_path, &worktree.path, force)?;
            if delete_branch {
                if let Some(branch) = worktree.branch.as_deref() {
                    git::delete_branch(&context.repo_path, branch, force)?;
                }
            }

            println!(
                "deleted\t{}\t{}",
                plain_field(worktree.branch.as_deref().unwrap_or("-")),
                plain_field(&worktree.path.display().to_string())
            );
            Ok(())
        }
        WorktreeCommand::Prune { path } => {
            let context = resolve_repository_context(&path)?;
            let metadata_output = git::prune_worktrees(&context.repo_path)?;
            let pruned_worktrees = prune_clean_merged_worktrees(&context.repo_path, &path)?;

            if metadata_output.is_empty() && pruned_worktrees.is_empty() {
                println!("pruned\t0");
            } else {
                for worktree in pruned_worktrees {
                    println!(
                        "pruned\tworktree\t{}\t{}",
                        plain_field(&worktree.branch),
                        plain_field(&worktree.path.display().to_string())
                    );
                }
                for line in metadata_output.lines() {
                    println!("pruned\tmetadata\t{}", plain_field(line));
                }
            }
            Ok(())
        }
    }
}

fn run_pr_command(command: PrCommand) -> Result<()> {
    match command {
        PrCommand::Status { path, branch, all } => {
            let context = resolve_repository_context(&path)?;
            let worktrees = git::list_worktrees(&context.repo_path)?;
            let targets = pr_status_targets(&worktrees, &path, branch, all);
            let statuses = git::github_pr_statuses_for_worktrees(&context.repo_path, &targets);

            for ((target_path, status), (_, branch)) in
                statuses.into_iter().zip(targets.into_iter())
            {
                println!(
                    "{}\t{}\t{}",
                    plain_field(&branch),
                    plain_field(&target_path.display().to_string()),
                    status.map(|status| status.label()).unwrap_or("-")
                );
            }
            Ok(())
        }
    }
}

fn run_commit_command(command: CommitCommand) -> Result<()> {
    match command {
        CommitCommand::Tree { path, limit } => {
            let worktree_path = git::get_worktree_root(&path)
                .context("commit tree requires a non-bare Git worktree path")?;
            for line in git::get_recent_commit_graph(&worktree_path, limit)? {
                println!("{}", line);
            }
            Ok(())
        }
    }
}

fn run_search_command(command: SearchCommand) -> Result<()> {
    match command {
        SearchCommand::Query {
            path,
            query,
            include_pr,
        } => {
            let context = resolve_repository_context(&path)?;
            let mut worktrees = git::list_worktrees(&context.repo_path)?;
            if include_pr {
                refresh_pr_statuses(&context.repo_path, &mut worktrees);
            }
            let needle = query.to_lowercase();
            for worktree in &worktrees {
                if worktree_matches(worktree, &needle) {
                    print_worktree_record(worktree);
                }
            }
            Ok(())
        }
    }
}

fn resolve_repository_context(path: &Path) -> Result<RepositoryContext> {
    if let Some(bare_path) = git::find_bare_in_parent(path) {
        let project_root = bare_path
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| path.to_path_buf());
        return Ok(RepositoryContext {
            repo_path: bare_path,
            project_root_path: project_root,
            repo_is_bare: true,
        });
    }

    if !git::is_git_repo(path) {
        anyhow::bail!("Not a git repository");
    }

    let common_dir = git::get_git_common_dir(path)?;
    if git::is_bare_repo(&common_dir)? {
        let project_root = common_dir
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| path.to_path_buf());
        Ok(RepositoryContext {
            repo_path: common_dir,
            project_root_path: project_root,
            repo_is_bare: true,
        })
    } else {
        let worktree_root = git::get_worktree_root(path)?;
        Ok(RepositoryContext {
            repo_path: worktree_root.clone(),
            project_root_path: worktree_root,
            repo_is_bare: false,
        })
    }
}

fn refresh_pr_statuses(repo_path: &Path, worktrees: &mut [types::Worktree]) {
    let targets: Vec<(PathBuf, String)> = worktrees
        .iter()
        .filter_map(|worktree| {
            if worktree.is_bare {
                return None;
            }
            Some((worktree.path.clone(), worktree.branch.clone()?))
        })
        .collect();

    for (path, status) in git::github_pr_statuses_for_worktrees(repo_path, &targets) {
        if let Some(worktree) = worktrees.iter_mut().find(|worktree| worktree.path == path) {
            worktree.github_pr_status = status;
        }
    }
}

fn worktree_path_for_branch(context: &RepositoryContext, config: &Config, branch: &str) -> PathBuf {
    if context.repo_is_bare {
        return context
            .repo_path
            .parent()
            .map(|parent| parent.join(branch))
            .unwrap_or_else(|| PathBuf::from(branch));
    }

    config
        .resolved_worktree_root()
        .join(repo_namespace(&context.project_root_path))
        .join(branch)
}

fn worktree_name_from_path(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "worktree".to_string())
}

fn repo_namespace(project_root_path: &Path) -> String {
    project_root_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "repo".to_string())
}

fn conflicting_worktree_for_branch<'a>(
    worktrees: &'a [types::Worktree],
    branch: &str,
    desired_path: &Path,
) -> Option<&'a types::Worktree> {
    worktrees.iter().find(|worktree| {
        !worktree.is_bare
            && worktree.branch.as_deref() == Some(branch)
            && !paths_refer_to_same_location(&worktree.path, desired_path)
    })
}

fn find_worktree_target(worktrees: &[types::Worktree], target: &str) -> Result<types::Worktree> {
    let target_path = PathBuf::from(target);
    let matches: Vec<&types::Worktree> = worktrees
        .iter()
        .filter(|worktree| {
            worktree.branch.as_deref() == Some(target)
                || worktree.display_name() == target
                || worktree.path == target_path
                || paths_refer_to_same_location(&worktree.path, &target_path)
        })
        .collect();

    match matches.as_slice() {
        [worktree] => Ok((*worktree).clone()),
        [] => anyhow::bail!("No worktree matches '{}'", target),
        _ => anyhow::bail!(
            "Multiple worktrees match '{}'; use an absolute path",
            target
        ),
    }
}

fn prune_clean_merged_worktrees(
    repo_path: &Path,
    launch_path: &Path,
) -> Result<Vec<PrunedWorktree>> {
    let worktrees = git::list_worktrees(repo_path)?;
    let current_path = current_worktree_path(&worktrees, launch_path);
    let mut pruned = Vec::new();

    for worktree in worktrees {
        if worktree.is_bare || worktree.status != types::WorktreeStatus::Clean {
            continue;
        }
        if current_path
            .as_ref()
            .map(|path| paths_refer_to_same_location(path, &worktree.path))
            .unwrap_or(false)
        {
            continue;
        }
        let Some(branch) = worktree.branch.as_deref() else {
            continue;
        };
        if !git::is_branch_merged(repo_path, branch, "HEAD")? {
            continue;
        }

        git::remove_worktree(repo_path, &worktree.path, false)?;
        pruned.push(PrunedWorktree {
            branch: branch.to_string(),
            path: worktree.path,
        });
    }

    Ok(pruned)
}

fn current_worktree_path(worktrees: &[types::Worktree], launch_path: &Path) -> Option<PathBuf> {
    let canonical_launch = launch_path.canonicalize().ok()?;
    worktrees
        .iter()
        .find(|worktree| {
            !worktree.is_bare
                && worktree
                    .path
                    .canonicalize()
                    .ok()
                    .map(|path| canonical_launch.starts_with(path))
                    .unwrap_or(false)
        })
        .map(|worktree| worktree.path.clone())
}

fn pr_status_targets(
    worktrees: &[types::Worktree],
    launch_path: &Path,
    branch: Option<String>,
    all: bool,
) -> Vec<(PathBuf, String)> {
    if all {
        return worktrees
            .iter()
            .filter_map(|worktree| Some((worktree.path.clone(), worktree.branch.clone()?)))
            .collect();
    }

    if let Some(branch) = branch {
        return worktrees
            .iter()
            .find(|worktree| worktree.branch.as_deref() == Some(branch.as_str()))
            .map(|worktree| vec![(worktree.path.clone(), branch.clone())])
            .unwrap_or_else(|| vec![(PathBuf::from("-"), branch)]);
    }

    current_worktree_path(worktrees, launch_path)
        .and_then(|path| {
            worktrees
                .iter()
                .find(|worktree| worktree.path == path)
                .and_then(|worktree| Some((worktree.path.clone(), worktree.branch.clone()?)))
        })
        .or_else(|| {
            worktrees
                .iter()
                .find(|worktree| !worktree.is_bare)
                .and_then(|worktree| Some((worktree.path.clone(), worktree.branch.clone()?)))
        })
        .into_iter()
        .collect()
}

fn worktree_matches(worktree: &types::Worktree, needle: &str) -> bool {
    let pr_status = worktree.github_pr_status.map(|status| status.label());
    [
        worktree.path.display().to_string(),
        worktree.display_name(),
        worktree.branch_display(),
        worktree.status.label().to_string(),
        pr_status.unwrap_or("-").to_string(),
    ]
    .iter()
    .any(|value| value.to_lowercase().contains(needle))
}

fn print_worktree_record(worktree: &types::Worktree) {
    let (ahead, behind) = worktree
        .ahead_behind
        .as_ref()
        .map(|ahead_behind| {
            (
                ahead_behind.ahead.to_string(),
                ahead_behind.behind.to_string(),
            )
        })
        .unwrap_or_else(|| ("-".to_string(), "-".to_string()));

    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        if worktree.is_bare { "bare" } else { "worktree" },
        plain_field(&worktree.path.display().to_string()),
        plain_field(&worktree.branch_display()),
        worktree.status.label(),
        plain_field(worktree.last_commit_time.as_deref().unwrap_or("-")),
        ahead,
        behind,
        worktree.github_pr_display()
    );
}

fn copy_configured_files(source: &Path, destination: &Path, files: &[String]) -> Vec<String> {
    files
        .iter()
        .filter_map(|file| {
            copy_configured_file(source, destination, file)
                .err()
                .map(|error| error.to_string())
        })
        .collect()
}

fn copy_configured_file(source: &Path, destination: &Path, file: &str) -> Result<()> {
    let src = source.join(file);
    let dst = destination.join(file);

    if !src.exists() {
        anyhow::bail!("{} source file missing at {}", file, src.display());
    }
    if !src.is_file() {
        anyhow::bail!("{} source is not a file at {}", file, src.display());
    }
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("could not create {}", parent.display()))?;
    }
    std::fs::copy(&src, &dst)
        .with_context(|| format!("could not copy {} to {}", src.display(), dst.display()))?;
    Ok(())
}

fn paths_refer_to_same_location(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }

    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn plain_field(value: &str) -> String {
    value.replace(['\t', '\n', '\r'], " ")
}

fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}

fn process_failure_detail(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stderr.is_empty() {
        stdout
    } else {
        stderr
    }
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
        "--help" | "-h" | "help" => Command::Help(HelpTopic::Root),
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
        "worktree" => parse_worktree_command(&args[2..], current_dir()),
        "pr" => parse_pr_command(&args[2..], current_dir()),
        "commit" => parse_commit_command(&args[2..], current_dir()),
        "search" => parse_search_command(&args[2..], current_dir()),
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

fn parse_worktree_command(args: &[String], default_path: PathBuf) -> Command {
    if args.is_empty() || is_help_arg(&args[0]) {
        return Command::Help(HelpTopic::Worktree);
    }

    match args[0].as_str() {
        "list" | "ls" => {
            if has_help_arg(&args[1..]) {
                return Command::Help(HelpTopic::WorktreeList);
            }
            let mut path = default_path;
            let mut include_pr = false;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--path" | "-p" => {
                        path = PathBuf::from(option_value(args, i, "--path"));
                        i += 2;
                    }
                    "--pr" => {
                        include_pr = true;
                        i += 1;
                    }
                    arg => unknown_arg("owt worktree list", arg),
                }
            }
            Command::Worktree(WorktreeCommand::List { path, include_pr })
        }
        "create" => {
            if has_help_arg(&args[1..]) {
                return Command::Help(HelpTopic::WorktreeCreate);
            }
            let mut path = default_path;
            let mut base = None;
            let mut worktree_path = None;
            let mut tmux = None;
            let mut branch = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--path" | "-p" => {
                        path = PathBuf::from(option_value(args, i, "--path"));
                        i += 2;
                    }
                    "--base" | "-b" => {
                        base = Some(option_value(args, i, "--base").to_string());
                        i += 2;
                    }
                    "--worktree-path" => {
                        worktree_path =
                            Some(PathBuf::from(option_value(args, i, "--worktree-path")));
                        i += 2;
                    }
                    arg if arg.starts_with("--tmux=") => {
                        tmux = Some(parse_on_off_value("--tmux", &arg["--tmux=".len()..]));
                        i += 1;
                    }
                    "--tmux" => {
                        if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                            tmux = Some(parse_on_off_value("--tmux", &args[i + 1]));
                            i += 2;
                        } else {
                            tmux = Some(true);
                            i += 1;
                        }
                    }
                    arg if arg.starts_with('-') => unknown_arg("owt worktree create", arg),
                    arg => {
                        if branch.replace(arg.to_string()).is_some() {
                            unknown_arg("owt worktree create", arg);
                        }
                        i += 1;
                    }
                }
            }
            let branch = branch.unwrap_or_else(|| missing_arg("owt worktree create", "<branch>"));
            Command::Worktree(WorktreeCommand::Create {
                path,
                branch,
                base,
                worktree_path,
                tmux,
            })
        }
        "delete" | "remove" | "rm" => {
            if has_help_arg(&args[1..]) {
                return Command::Help(HelpTopic::WorktreeDelete);
            }
            let mut path = default_path;
            let mut force = false;
            let mut delete_branch = false;
            let mut target = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--path" | "-p" => {
                        path = PathBuf::from(option_value(args, i, "--path"));
                        i += 2;
                    }
                    "--force" | "-f" => {
                        force = true;
                        i += 1;
                    }
                    "--branch" => {
                        delete_branch = true;
                        i += 1;
                    }
                    arg if arg.starts_with('-') => unknown_arg("owt worktree delete", arg),
                    arg => {
                        if target.replace(arg.to_string()).is_some() {
                            unknown_arg("owt worktree delete", arg);
                        }
                        i += 1;
                    }
                }
            }
            let target = target.unwrap_or_else(|| missing_arg("owt worktree delete", "<target>"));
            Command::Worktree(WorktreeCommand::Delete {
                path,
                target,
                force,
                delete_branch,
            })
        }
        "prune" => {
            if has_help_arg(&args[1..]) {
                return Command::Help(HelpTopic::WorktreePrune);
            }
            let mut path = default_path;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--path" | "-p" => {
                        path = PathBuf::from(option_value(args, i, "--path"));
                        i += 2;
                    }
                    arg => unknown_arg("owt worktree prune", arg),
                }
            }
            Command::Worktree(WorktreeCommand::Prune { path })
        }
        _ => Command::Help(HelpTopic::Worktree),
    }
}

fn parse_pr_command(args: &[String], default_path: PathBuf) -> Command {
    if args.is_empty() || is_help_arg(&args[0]) {
        return Command::Help(HelpTopic::Pr);
    }

    match args[0].as_str() {
        "status" => {
            if has_help_arg(&args[1..]) {
                return Command::Help(HelpTopic::PrStatus);
            }
            let mut path = default_path;
            let mut branch = None;
            let mut all = false;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--path" | "-p" => {
                        path = PathBuf::from(option_value(args, i, "--path"));
                        i += 2;
                    }
                    "--branch" | "-b" => {
                        branch = Some(option_value(args, i, "--branch").to_string());
                        i += 2;
                    }
                    "--all" => {
                        all = true;
                        i += 1;
                    }
                    arg => unknown_arg("owt pr status", arg),
                }
            }
            Command::Pr(PrCommand::Status { path, branch, all })
        }
        _ => Command::Help(HelpTopic::Pr),
    }
}

fn parse_commit_command(args: &[String], default_path: PathBuf) -> Command {
    if args.is_empty() || is_help_arg(&args[0]) {
        return Command::Help(HelpTopic::Commit);
    }

    match args[0].as_str() {
        "tree" => {
            if has_help_arg(&args[1..]) {
                return Command::Help(HelpTopic::CommitTree);
            }
            let mut path = default_path;
            let mut limit = 8;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--path" | "-p" => {
                        path = PathBuf::from(option_value(args, i, "--path"));
                        i += 2;
                    }
                    "--limit" | "-n" => {
                        limit = option_value(args, i, "--limit")
                            .parse()
                            .unwrap_or_else(|_| {
                                eprintln!("Error: --limit requires a positive integer");
                                std::process::exit(1);
                            });
                        i += 2;
                    }
                    arg => unknown_arg("owt commit tree", arg),
                }
            }
            Command::Commit(CommitCommand::Tree { path, limit })
        }
        _ => Command::Help(HelpTopic::Commit),
    }
}

fn parse_search_command(args: &[String], default_path: PathBuf) -> Command {
    if args.is_empty() || is_help_arg(&args[0]) {
        return Command::Help(HelpTopic::Search);
    }

    let mut path = default_path;
    let mut include_pr = false;
    let mut query = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--path" | "-p" => {
                path = PathBuf::from(option_value(args, i, "--path"));
                i += 2;
            }
            "--pr" => {
                include_pr = true;
                i += 1;
            }
            arg if arg.starts_with('-') => unknown_arg("owt search", arg),
            arg => {
                if query.replace(arg.to_string()).is_some() {
                    unknown_arg("owt search", arg);
                }
                i += 1;
            }
        }
    }
    let query = query.unwrap_or_else(|| missing_arg("owt search", "<query>"));
    Command::Search(SearchCommand::Query {
        path,
        query,
        include_pr,
    })
}

fn option_value<'a>(args: &'a [String], index: usize, flag: &str) -> &'a str {
    args.get(index + 1)
        .map(String::as_str)
        .unwrap_or_else(|| missing_arg(flag, "<value>"))
}

fn parse_on_off_value(flag: &str, value: &str) -> bool {
    match value {
        "on" | "true" | "1" | "yes" => true,
        "off" | "false" | "0" | "no" => false,
        _ => {
            eprintln!("Error: {} requires on or off", flag);
            std::process::exit(1);
        }
    }
}

fn is_help_arg(arg: &str) -> bool {
    matches!(arg, "--help" | "-h" | "help")
}

fn has_help_arg(args: &[String]) -> bool {
    args.iter().any(|arg| is_help_arg(arg))
}

fn unknown_arg(command: &str, arg: &str) -> ! {
    eprintln!("Error: unexpected argument '{}' for {}", arg, command);
    eprintln!("Run '{} --help' for usage.", command);
    std::process::exit(1);
}

fn missing_arg(command: &str, arg: &str) -> ! {
    eprintln!("Error: {} requires {}", command, arg);
    std::process::exit(1);
}

fn print_help(topic: HelpTopic) {
    match topic {
        HelpTopic::Root => print_root_help(),
        HelpTopic::Worktree => print_worktree_help(),
        HelpTopic::WorktreeList => print_worktree_list_help(),
        HelpTopic::WorktreeCreate => print_worktree_create_help(),
        HelpTopic::WorktreeDelete => print_worktree_delete_help(),
        HelpTopic::WorktreePrune => print_worktree_prune_help(),
        HelpTopic::Pr => print_pr_help(),
        HelpTopic::PrStatus => print_pr_status_help(),
        HelpTopic::Commit => print_commit_help(),
        HelpTopic::CommitTree => print_commit_tree_help(),
        HelpTopic::Search => print_search_help(),
    }
}

fn print_root_help() {
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
    worktree             Manage worktrees with plain CLI output
    pr                   Inspect GitHub PR merge status
    commit               Inspect commit history
    search               Search worktrees

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
    owt --path ~/repos/myproject.git
    owt worktree list
    owt worktree create feature/login --base main
    owt pr status --branch feature/login
    owt commit tree -n 12
    owt search login"#
    );
}

fn print_worktree_help() {
    println!(
        r#"Manage worktrees with plain CLI output.

USAGE:
    owt worktree <COMMAND>

COMMANDS:
    list      List worktrees as tab-separated records
    create    Create a worktree for a branch
    delete    Delete a worktree by branch, name, or path
    prune     Prune missing metadata and clean merged worktrees

EXAMPLES:
    owt worktree list
    owt worktree list --pr
    owt worktree create feature/login --base main
    owt worktree delete feature/login --branch --force"#
    );
}

fn print_worktree_list_help() {
    println!(
        r#"List worktrees as tab-separated records.

USAGE:
    owt worktree list [OPTIONS]

OPTIONS:
    -p, --path <PATH>    Repository or worktree path (default: current directory)
        --pr             Include GitHub PR status from gh
    -h, --help           Print help information

OUTPUT:
    kind<TAB>path<TAB>branch<TAB>status<TAB>last_commit<TAB>ahead<TAB>behind<TAB>pr"#
    );
}

fn print_worktree_create_help() {
    println!(
        r#"Create a worktree for a branch.

USAGE:
    owt worktree create <BRANCH> [OPTIONS]

OPTIONS:
    -p, --path <PATH>             Repository or worktree path (default: current directory)
    -b, --base <BRANCH>           Base branch for a new branch
        --worktree-path <PATH>    Explicit destination path
        --tmux=on|off             Override tmux worktree pane mode for this create
    -h, --help                    Print help information

OUTPUT:
    created<TAB>branch<TAB>path"#
    );
}

fn print_worktree_delete_help() {
    println!(
        r#"Delete a worktree by branch, name, or path.

USAGE:
    owt worktree delete <TARGET> [OPTIONS]

OPTIONS:
    -p, --path <PATH>    Repository or worktree path (default: current directory)
    -f, --force          Delete even with uncommitted changes
        --branch         Delete the local branch after removing the worktree
    -h, --help           Print help information

OUTPUT:
    deleted<TAB>branch<TAB>path"#
    );
}

fn print_worktree_prune_help() {
    println!(
        r#"Prune missing metadata and clean merged worktrees.

USAGE:
    owt worktree prune [OPTIONS]

OPTIONS:
    -p, --path <PATH>    Repository or worktree path (default: current directory)
    -h, --help           Print help information

OUTPUT:
    pruned<TAB>0
    pruned<TAB>worktree<TAB>branch<TAB>path
    pruned<TAB>metadata<TAB>result

NOTES:
    Removes non-current worktrees only when they are clean and their branch is already merged into HEAD. Branches are not deleted."#
    );
}

fn print_pr_help() {
    println!(
        r#"Inspect GitHub PR status through gh.

USAGE:
    owt pr <COMMAND>

COMMANDS:
    status    Show PR status for a branch or worktree

EXAMPLES:
    owt pr status
    owt pr status --branch feature/login
    owt pr status --all"#
    );
}

fn print_pr_status_help() {
    println!(
        r#"Show GitHub PR status for a branch or worktree.

USAGE:
    owt pr status [OPTIONS]

OPTIONS:
    -p, --path <PATH>       Repository or worktree path (default: current directory)
    -b, --branch <BRANCH>   Branch to inspect
        --all               Inspect every non-bare worktree
    -h, --help              Print help information

OUTPUT:
    branch<TAB>path<TAB>status"#
    );
}

fn print_commit_help() {
    println!(
        r#"Inspect commit history.

USAGE:
    owt commit <COMMAND>

COMMANDS:
    tree    Print recent commits as a git graph

EXAMPLES:
    owt commit tree
    owt commit tree -n 20"#
    );
}

fn print_commit_tree_help() {
    println!(
        r#"Print recent commits as a git graph.

USAGE:
    owt commit tree [OPTIONS]

OPTIONS:
    -p, --path <PATH>     Worktree path (default: current directory)
    -n, --limit <COUNT>   Number of commits to print (default: 8)
    -h, --help            Print help information"#
    );
}

fn print_search_help() {
    println!(
        r#"Search worktrees by path, name, branch, status, or PR status.

USAGE:
    owt search <QUERY> [OPTIONS]

OPTIONS:
    -p, --path <PATH>    Repository or worktree path (default: current directory)
        --pr             Include GitHub PR status before searching
    -h, --help           Print help information

OUTPUT:
    kind<TAB>path<TAB>branch<TAB>status<TAB>last_commit<TAB>ahead<TAB>behind<TAB>pr"#
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

    fn add_worktree_branch(repo: &Path, path: &Path, branch: &str) {
        assert_git_success(
            git_cmd()
                .current_dir(repo)
                .args([
                    "worktree",
                    "add",
                    "-b",
                    branch,
                    &path.to_string_lossy(),
                    "main",
                ])
                .output()
                .unwrap(),
            "git worktree add failed",
        );
    }

    fn commit_file(repo: &Path, relative_path: &str, contents: &str, message: &str) {
        fs::write(repo.join(relative_path), contents).unwrap();
        assert_git_success(
            git_cmd()
                .current_dir(repo)
                .args(["add", relative_path])
                .output()
                .unwrap(),
            "git add failed",
        );
        assert_git_success(
            git_cmd()
                .current_dir(repo)
                .args(["commit", "-m", message])
                .output()
                .unwrap(),
            "git commit failed",
        );
    }

    fn merge_branch(repo: &Path, branch: &str) {
        assert_git_success(
            git_cmd()
                .current_dir(repo)
                .args(["merge", "--no-ff", branch, "-m", &format!("merge {branch}")])
                .output()
                .unwrap(),
            "git merge failed",
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
            Command::Help(HelpTopic::Root)
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
    fn parse_args_recognizes_worktree_cli_commands() {
        assert!(matches!(
            parse_args_from(
                vec![
                    "owt".to_string(),
                    "worktree".to_string(),
                    "list".to_string(),
                    "--path".to_string(),
                    "/repo".to_string(),
                    "--pr".to_string(),
                ],
                || PathBuf::from("/cwd")
            ),
            Command::Worktree(WorktreeCommand::List { path, include_pr })
                if path == PathBuf::from("/repo") && include_pr
        ));
        assert!(matches!(
            parse_args_from(
                vec![
                    "owt".to_string(),
                    "worktree".to_string(),
                    "create".to_string(),
                    "feature/login".to_string(),
                    "--base".to_string(),
                    "main".to_string(),
                    "--worktree-path".to_string(),
                    "/tmp/login".to_string(),
                    "--tmux=on".to_string(),
                ],
                || PathBuf::from("/cwd")
            ),
            Command::Worktree(WorktreeCommand::Create {
                path,
                branch,
                base,
                worktree_path,
                tmux
            }) if path == PathBuf::from("/cwd")
                && branch == "feature/login"
                && base == Some("main".to_string())
                && worktree_path == Some(PathBuf::from("/tmp/login"))
                && tmux == Some(true)
        ));
        assert!(matches!(
            parse_args_from(
                vec![
                    "owt".to_string(),
                    "worktree".to_string(),
                    "create".to_string(),
                    "feature/off".to_string(),
                    "--tmux".to_string(),
                    "off".to_string(),
                ],
                PathBuf::new
            ),
            Command::Worktree(WorktreeCommand::Create {
                branch,
                tmux: Some(false),
                ..
            }) if branch == "feature/off"
        ));
        assert!(matches!(
            parse_args_from(
                vec![
                    "owt".to_string(),
                    "worktree".to_string(),
                    "delete".to_string(),
                    "feature/login".to_string(),
                    "--branch".to_string(),
                    "--force".to_string(),
                ],
                PathBuf::new
            ),
            Command::Worktree(WorktreeCommand::Delete {
                target,
                force: true,
                delete_branch: true,
                ..
            }) if target == "feature/login"
        ));
        assert!(matches!(
            parse_args_from(
                vec![
                    "owt".to_string(),
                    "worktree".to_string(),
                    "prune".to_string(),
                    "--path".to_string(),
                    "/repo".to_string(),
                ],
                PathBuf::new
            ),
            Command::Worktree(WorktreeCommand::Prune { path })
                if path == PathBuf::from("/repo")
        ));
    }

    #[test]
    fn parse_args_recognizes_plain_cli_help_topics() {
        assert!(matches!(
            parse_args_from(
                vec![
                    "owt".to_string(),
                    "worktree".to_string(),
                    "--help".to_string(),
                ],
                PathBuf::new
            ),
            Command::Help(HelpTopic::Worktree)
        ));
        assert!(matches!(
            parse_args_from(
                vec![
                    "owt".to_string(),
                    "worktree".to_string(),
                    "create".to_string(),
                    "--help".to_string(),
                ],
                PathBuf::new
            ),
            Command::Help(HelpTopic::WorktreeCreate)
        ));
        assert!(matches!(
            parse_args_from(
                vec![
                    "owt".to_string(),
                    "worktree".to_string(),
                    "prune".to_string(),
                    "--help".to_string(),
                ],
                PathBuf::new
            ),
            Command::Help(HelpTopic::WorktreePrune)
        ));
        assert!(matches!(
            parse_args_from(
                vec![
                    "owt".to_string(),
                    "pr".to_string(),
                    "status".to_string(),
                    "--help".to_string(),
                ],
                PathBuf::new
            ),
            Command::Help(HelpTopic::PrStatus)
        ));
    }

    #[test]
    fn parse_args_recognizes_pr_commit_and_search_cli_commands() {
        assert!(matches!(
            parse_args_from(
                vec![
                    "owt".to_string(),
                    "pr".to_string(),
                    "status".to_string(),
                    "--branch".to_string(),
                    "feature/login".to_string(),
                    "--all".to_string(),
                ],
                || PathBuf::from("/repo")
            ),
            Command::Pr(PrCommand::Status { path, branch, all: true })
                if path == PathBuf::from("/repo") && branch == Some("feature/login".to_string())
        ));
        assert!(matches!(
            parse_args_from(
                vec![
                    "owt".to_string(),
                    "commit".to_string(),
                    "tree".to_string(),
                    "-n".to_string(),
                    "12".to_string(),
                ],
                || PathBuf::from("/repo")
            ),
            Command::Commit(CommitCommand::Tree { path, limit: 12 })
                if path == PathBuf::from("/repo")
        ));
        assert!(matches!(
            parse_args_from(
                vec![
                    "owt".to_string(),
                    "search".to_string(),
                    "login".to_string(),
                    "--pr".to_string(),
                ],
                || PathBuf::from("/repo")
            ),
            Command::Search(SearchCommand::Query {
                path,
                query,
                include_pr: true
            }) if path == PathBuf::from("/repo") && query == "login"
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
    fn worktree_prune_removes_clean_merged_worktrees_only() {
        let base = temp_dir("prune_clean_merged");
        let repo = base.join("repo");
        let merged = base.join("merged");
        let merged_second = base.join("merged-second");
        let unmerged = base.join("unmerged");
        let dirty = base.join("dirty");

        create_source_repo(&repo);

        add_worktree_branch(&repo, &merged, "feature/merged");
        commit_file(&merged, "merged.txt", "merged\n", "feature merged");
        add_worktree_branch(&repo, &merged_second, "feature/merged-second");
        commit_file(
            &merged_second,
            "merged-second.txt",
            "merged second\n",
            "feature merged second",
        );
        add_worktree_branch(&repo, &unmerged, "feature/unmerged");
        commit_file(&unmerged, "unmerged.txt", "unmerged\n", "feature unmerged");
        add_worktree_branch(&repo, &dirty, "feature/dirty");
        commit_file(&dirty, "dirty.txt", "dirty\n", "feature dirty");

        merge_branch(&repo, "feature/merged");
        merge_branch(&repo, "feature/merged-second");
        merge_branch(&repo, "feature/dirty");
        fs::write(dirty.join("dirty.txt"), "dirty\nuncommitted\n").unwrap();

        run_worktree_command(WorktreeCommand::Prune { path: repo.clone() }).unwrap();

        assert!(!merged.exists(), "clean merged worktree should be removed");
        assert!(
            !merged_second.exists(),
            "all clean merged worktrees should be removed"
        );
        assert!(unmerged.exists(), "clean unmerged worktree should stay");
        assert!(dirty.exists(), "dirty merged worktree should stay");
        assert!(repo.exists(), "current worktree should stay");

        let list_output = git_cmd()
            .current_dir(&repo)
            .args(["worktree", "list", "--porcelain"])
            .output()
            .unwrap();
        assert!(
            list_output.status.success(),
            "git worktree list failed: {}",
            String::from_utf8_lossy(&list_output.stderr)
        );
        let list_stdout = String::from_utf8_lossy(&list_output.stdout);
        assert!(!list_stdout.contains(&merged.to_string_lossy().to_string()));
        assert!(!list_stdout.contains(&merged_second.to_string_lossy().to_string()));
        assert!(list_stdout.contains(&unmerged.to_string_lossy().to_string()));
        assert!(list_stdout.contains(&dirty.to_string_lossy().to_string()));
        assert!(list_stdout.contains(&repo.to_string_lossy().to_string()));

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn post_tui_create_worktree_creates_copies_and_writes_handoff_path() {
        let base = temp_dir("post_tui_create");
        let source = base.join("source-repo");
        let worktree_path = base.join("feature-post-tui");
        let output_path = base.join("owt-output");
        create_source_repo(&source);
        fs::create_dir_all(source.join("config")).unwrap();
        fs::write(source.join("config/local.env"), "TOKEN=secret\n").unwrap();
        fs::write(&output_path, "").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&output_path, fs::Permissions::from_mode(0o600)).unwrap();
        }

        let request = types::WorktreeCreateRequest {
            bare_repo_path: source.clone(),
            project_root_path: source.clone(),
            branch: "feature/post-tui".to_string(),
            base_branch: "main".to_string(),
            worktree_path: worktree_path.clone(),
            source_path: Some(source.clone()),
        };
        let mut config = Config::default();
        config.copy_files = vec!["config/local.env".to_string()];

        run_post_tui_create_worktree(&request, &config, Some(output_path.to_str().unwrap()))
            .unwrap();

        assert!(worktree_path.exists());
        assert_eq!(
            fs::read_to_string(worktree_path.join("config/local.env")).unwrap(),
            "TOKEN=secret\n"
        );
        assert_eq!(
            fs::read_to_string(output_path).unwrap(),
            format!("{}\n", worktree_path.display())
        );

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
    fn stdout_path_fallback_stays_machine_readable() {
        let source = include_str!("main.rs");
        let fallback_start = source
            .find("No shell integration - print only the path")
            .expect("path-only fallback comment should exist");
        let fallback_end = source[fallback_start..]
            .find("types::ExitAction::Quit")
            .expect("quit match arm should follow fallback")
            + fallback_start;
        let fallback_block = &source[fallback_start..fallback_end];

        assert!(fallback_block.contains("println!(\"{}\", worktree_path.display())"));
        assert!(!fallback_block.contains("eprintln!"));
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
