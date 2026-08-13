use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{backend::Backend, Frame, Terminal};
use std::cell::Cell;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::git;
use crate::tmux;
use crate::types::{
    ActiveOp, AddModalField, AppMessage, AppState, ExitAction, GithubPrStatus, OpKind, OpResult,
    ScriptStatus, SortMode, Worktree, WorktreeCreateRequest, WorktreeDetails, WorktreeStatus,
};
use crate::ui::theme::Theme;
use crate::ui::{add_modal, config_modal, confirm_modal, help_modal, main_view};

const G_SEQUENCE_TIMEOUT: Duration = Duration::from_millis(300);
type PrStatusResults = Vec<(PathBuf, Option<GithubPrStatus>)>;
type PrQueryResults = Vec<(String, PathBuf, Option<GithubPrStatus>)>;

pub struct ScriptResult {
    pub success: bool,
    pub message: String,
}

pub struct App {
    pub worktrees: Vec<Worktree>,
    pub selected_index: usize,
    pub selected_worktree_paths: HashSet<PathBuf>,
    pub state: AppState,
    pub message: Option<AppMessage>,
    pub bare_repo_path: PathBuf,
    pub project_root_path: PathBuf,
    pub repo_is_bare: bool,
    pub input_buffer: String,
    pub should_quit: bool,
    pub config: Config,
    pub exit_action: ExitAction,
    pub current_worktree_path: Option<PathBuf>, // Path where owt was launched from
    pub merge_source_branch: Option<String>,    // Branch to merge from
    pub has_shell_integration: bool,            // Whether OWT_OUTPUT_FILE is set
    pub filter_text: String,                    // Search/filter text
    pub is_filtering: bool,                     // Whether in filter mode
    pub last_key: Option<char>,                 // For gg detection
    pending_g_since: Option<Instant>,
    pub sort_mode: SortMode,                 // Current sort mode
    pub verbose: bool,                       // Show detailed git command output
    pub last_command_detail: Option<String>, // Last git command detail for verbose mode
    pub spinner_tick: usize,                 // Spinner animation tick
    pub theme: Theme,                        // Active color theme
    pub viewport_height: Cell<u16>,          // Table viewport height (set during render)
    pub help_scroll_offset: u16,             // Scroll offset for help modal
    pub script_status: ScriptStatus,         // Background script status
    pub script_receiver: Option<mpsc::Receiver<ScriptResult>>, // Channel for script completion
    pub pr_status_receiver: Option<mpsc::Receiver<PrStatusResults>>,
    pub pr_query_receiver: Option<mpsc::Receiver<PrQueryResults>>,
    pub pr_query_input: String,
    pub pr_query_editing: bool,
    pub pr_query_results: Vec<(String, PathBuf, Option<GithubPrStatus>)>,
    pub active_op: Option<(OpKind, mpsc::Receiver<OpResult>)>,
    pub active_op_info: Option<ActiveOp>,
    pub selected_details: Option<WorktreeDetails>,
    pub add_base_branch: String,
    pub add_modal_field: AddModalField,
    pub add_worktree_path: String,
    pub add_tmux_override: Option<bool>,
    pub cleanup_preview: Option<crate::worktree_prune::PruneReport>,
}

impl App {
    pub fn new(
        bare_repo_path: PathBuf,
        project_root_path: PathBuf,
        repo_is_bare: bool,
        launch_path: Option<PathBuf>,
        has_shell_integration: bool,
    ) -> Result<Self> {
        let worktrees = git::list_worktrees(&bare_repo_path)?;
        // Load config with project-level override support
        let config = Config::load_with_project(Some(&project_root_path)).unwrap_or_default();

        // Determine current worktree from launch path
        let current_worktree_path = launch_path.and_then(|lp| {
            let canonical_lp = lp.canonicalize().ok()?;
            worktrees
                .iter()
                .find(|wt| {
                    if wt.is_bare {
                        return false;
                    }
                    wt.path
                        .canonicalize()
                        .ok()
                        .map(|p| canonical_lp.starts_with(&p))
                        .unwrap_or(false)
                })
                .map(|wt| wt.path.clone())
        });

        // Set initial selection to current worktree if found, otherwise first non-bare worktree
        let selected_index = current_worktree_path
            .as_ref()
            .and_then(|cp| worktrees.iter().position(|wt| wt.path == *cp))
            .unwrap_or_else(|| {
                // Find first non-bare worktree
                worktrees.iter().position(|wt| !wt.is_bare).unwrap_or(0)
            });

        // Show initial message about shell integration if not set up
        let initial_message = if !has_shell_integration {
            Some(AppMessage::info(
                "Tip: Run 'owt setup' then reload shell for Enter key to change directory",
            ))
        } else {
            None
        };

        let mut app = Self {
            worktrees,
            selected_index,
            selected_worktree_paths: HashSet::new(),
            state: AppState::List,
            message: initial_message,
            bare_repo_path,
            project_root_path,
            repo_is_bare,
            input_buffer: String::new(),
            should_quit: false,
            config,
            exit_action: ExitAction::Quit,
            current_worktree_path,
            merge_source_branch: None,
            has_shell_integration,
            filter_text: String::new(),
            is_filtering: false,
            last_key: None,
            pending_g_since: None,
            sort_mode: SortMode::default(),
            verbose: false,
            last_command_detail: None,
            spinner_tick: 0,
            theme: crate::ui::theme::detect_theme(),
            viewport_height: Cell::new(0),
            help_scroll_offset: 0,
            script_status: ScriptStatus::Idle,
            script_receiver: None,
            pr_status_receiver: None,
            pr_query_receiver: None,
            pr_query_input: String::new(),
            pr_query_editing: false,
            pr_query_results: Vec::new(),
            active_op: None,
            active_op_info: None,
            selected_details: None,
            add_base_branch: "main".to_string(),
            add_modal_field: AddModalField::default(),
            add_worktree_path: String::new(),
            add_tmux_override: None,
            cleanup_preview: None,
        };
        app.update_selected_details();
        app.start_pr_status_refresh();
        Ok(app)
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        while !self.should_quit {
            self.resolve_pending_g_if_expired();
            terminal.draw(|frame| self.draw(frame))?;
            self.poll_script_status();
            self.poll_pr_status();
            self.poll_pr_query();
            self.poll_background_op();

            self.handle_events(terminal)?;
        }
        Ok(())
    }

    fn start_pr_status_refresh(&mut self) {
        let bare_repo_path = self.bare_repo_path.clone();
        let worktrees: Vec<(PathBuf, String)> = self
            .worktrees
            .iter()
            .filter_map(|wt| {
                if wt.is_bare {
                    return None;
                }
                wt.branch
                    .as_ref()
                    .map(|branch| (wt.path.clone(), branch.clone()))
            })
            .collect();

        if worktrees.is_empty() {
            self.pr_status_receiver = None;
            return;
        }

        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let statuses = git::github_pr_statuses_for_worktrees(&bare_repo_path, &worktrees);
            let _ = tx.send(statuses);
        });
        self.pr_status_receiver = Some(rx);
    }

    fn poll_pr_status(&mut self) {
        let result = if let Some(rx) = self.pr_status_receiver.as_ref() {
            match rx.try_recv() {
                Ok(statuses) => Some(Ok(statuses)),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => Some(Err(())),
            }
        } else {
            None
        };

        match result {
            Some(Ok(statuses)) => {
                for (path, status) in statuses {
                    if let Some(wt) = self.worktrees.iter_mut().find(|wt| wt.path == path) {
                        wt.github_pr_status = status;
                    }
                }
                self.pr_status_receiver = None;
            }
            Some(Err(())) => {
                self.pr_status_receiver = None;
            }
            Option::None => {}
        }
    }

    fn poll_pr_query(&mut self) {
        let result = self
            .pr_query_receiver
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok());
        if let Some(results) = result {
            self.pr_query_results = results;
            self.pr_query_receiver = None;
        }
    }

    fn open_pr_status_modal(&mut self) {
        self.pr_query_input.clear();
        self.pr_query_editing = false;
        self.pr_query_results.clear();
        self.state = AppState::PrStatusModal;
        self.query_pr_statuses(self.selected_pr_targets());
    }

    fn selected_pr_targets(&self) -> Vec<(PathBuf, String)> {
        self.selected_worktree()
            .filter(|worktree| !worktree.is_bare)
            .and_then(|worktree| {
                worktree
                    .branch
                    .as_ref()
                    .map(|branch| vec![(worktree.path.clone(), branch.clone())])
            })
            .unwrap_or_default()
    }

    fn pr_query_anchor_path(&self) -> Option<PathBuf> {
        self.selected_worktree()
            .filter(|worktree| !worktree.is_bare)
            .or_else(|| self.worktrees.iter().find(|worktree| !worktree.is_bare))
            .map(|worktree| worktree.path.clone())
    }

    fn all_pr_targets(&self) -> Vec<(PathBuf, String)> {
        self.worktrees
            .iter()
            .filter(|worktree| !worktree.is_bare)
            .filter_map(|worktree| {
                worktree
                    .branch
                    .as_ref()
                    .map(|branch| (worktree.path.clone(), branch.clone()))
            })
            .collect()
    }

    fn query_pr_statuses(&mut self, targets: Vec<(PathBuf, String)>) {
        if targets.is_empty() {
            self.pr_query_results.clear();
            return;
        }
        let repo_path = self.bare_repo_path.clone();
        let targets_for_thread = targets.clone();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let statuses = git::github_pr_statuses_for_worktrees(&repo_path, &targets_for_thread);
            let results = targets
                .into_iter()
                .zip(statuses)
                .map(|((_, branch), (path, status))| (branch, path, status))
                .collect();
            let _ = sender.send(results);
        });
        self.pr_query_results.clear();
        self.pr_query_receiver = Some(receiver);
    }

    fn handle_pr_status_modal_input(&mut self, code: KeyCode) {
        if self.pr_query_editing {
            match code {
                KeyCode::Esc => {
                    self.pr_query_editing = false;
                    self.pr_query_input.clear();
                }
                KeyCode::Enter => {
                    let branch = self.pr_query_input.trim().to_string();
                    self.pr_query_editing = false;
                    if !branch.is_empty() {
                        if let Some(path) = self.pr_query_anchor_path() {
                            self.query_pr_statuses(vec![(path, branch)]);
                        }
                    }
                }
                KeyCode::Backspace => {
                    self.pr_query_input.pop();
                }
                KeyCode::Char(character) => self.pr_query_input.push(character),
                _ => {}
            }
            return;
        }

        match code {
            KeyCode::Esc | KeyCode::Char('q') => self.state = AppState::List,
            KeyCode::Char('a') => self.query_pr_statuses(self.all_pr_targets()),
            KeyCode::Char('b') => {
                self.pr_query_editing = true;
                self.pr_query_input.clear();
            }
            KeyCode::Char('s') | KeyCode::Enter => {
                self.query_pr_statuses(self.selected_pr_targets())
            }
            _ => {}
        }
    }

    fn poll_script_status(&mut self) {
        if let Some(ref rx) = self.script_receiver {
            match rx.try_recv() {
                Ok(result) => {
                    let status_msg = if result.success {
                        format!("Setup script {}", result.message)
                    } else {
                        format!("Setup script failed: {}", result.message)
                    };
                    self.message = Some(if result.success {
                        AppMessage::info(status_msg)
                    } else {
                        AppMessage::error(status_msg)
                    });
                    self.script_status = ScriptStatus::Idle;
                    self.script_receiver = None;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // Still running, tick spinner
                    self.spinner_tick = self.spinner_tick.wrapping_add(1);
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.script_status = ScriptStatus::Idle;
                    self.script_receiver = None;
                }
            }
        }
    }

    fn poll_background_op(&mut self) {
        let result = if let Some((_, rx)) = self.active_op.as_ref() {
            match rx.try_recv() {
                Ok(result) => Some(Ok(result)),
                Err(mpsc::TryRecvError::Empty) => {
                    self.spinner_tick = self.spinner_tick.wrapping_add(1);
                    None
                }
                Err(mpsc::TryRecvError::Disconnected) => Some(Err(())),
            }
        } else {
            None
        };

        match result {
            Some(Ok(result)) => {
                self.handle_op_result(result);
                self.active_op = None;
                self.active_op_info = None;
                self.state = AppState::List;
            }
            Some(Err(())) => {
                self.message = Some(AppMessage::error("Operation failed unexpectedly"));
                self.active_op = None;
                self.active_op_info = None;
                self.state = AppState::List;
            }
            Option::None => {}
        }
    }

    fn handle_op_result(&mut self, result: OpResult) {
        let OpResult {
            kind,
            success,
            message,
            cmd_detail,
            worktree_path,
            affected_paths,
            ..
        } = result;

        if kind == OpKind::Merge {
            self.merge_source_branch = None;
        }

        if success {
            let mut msg = message;
            match kind {
                OpKind::Delete => {
                    let removed_paths: HashSet<PathBuf> = affected_paths.into_iter().collect();
                    self.worktrees
                        .retain(|wt| !removed_paths.contains(&wt.path));
                    self.selected_worktree_paths
                        .retain(|path| !removed_paths.contains(path));
                    self.clamp_selection_to_non_bare();
                    self.update_selected_details();
                }
                OpKind::Add => {
                    self.refresh_worktrees();
                    if let Some(idx) = self
                        .worktrees
                        .iter()
                        .position(|wt| paths_refer_to_same_location(&wt.path, &worktree_path))
                    {
                        self.selected_index = idx;
                    }
                    self.update_selected_details();
                    self.run_post_add_script(&worktree_path);
                    if self.config.tmux_worktree_mode {
                        let worktree_name = self
                            .selected_worktree()
                            .map(Worktree::display_name)
                            .unwrap_or_else(|| worktree_name_from_path(&worktree_path));
                        match tmux::open_worktree_pane(&worktree_path, &worktree_name) {
                            Ok(()) => {
                                msg = format!("{}\nOpened tmux pane: {}", msg, worktree_name);
                            }
                            Err(error) => {
                                msg = format!("{}\nTmux pane warning: {}", msg, error);
                            }
                        }
                    }
                }
                OpKind::Fetch | OpKind::Pull | OpKind::Push | OpKind::Prune | OpKind::Merge => {
                    self.refresh_worktrees();
                    self.update_selected_details();
                }
            }

            if self.verbose {
                self.last_command_detail = Some(cmd_detail.clone());
                msg = format!("{}\n$ {}", msg, cmd_detail);
            }
            self.message = Some(AppMessage::info(msg));
        } else {
            if kind == OpKind::Delete {
                for path in affected_paths {
                    self.worktrees.retain(|wt| wt.path != path);
                    self.selected_worktree_paths.remove(&path);
                }
                self.clamp_selection_to_non_bare();
                self.update_selected_details();
            }
            let mut msg = format!("Failed: {}", message);
            if self.verbose {
                self.last_command_detail = Some(cmd_detail.clone());
                msg = format!("{}\n$ {}", msg, cmd_detail);
            }
            self.message = Some(AppMessage::error(msg));
        }
    }

    fn draw(&self, frame: &mut Frame) {
        match self.state {
            AppState::List => main_view::render(frame, self),
            AppState::CleanupPreview => {
                main_view::render(frame, self);
                crate::ui::cleanup_modal::render(frame, self);
            }
            AppState::AddModal => {
                main_view::render(frame, self);
                add_modal::render(frame, self);
            }
            AppState::ConfirmDelete { .. } => {
                main_view::render(frame, self);
                confirm_modal::render(frame, self);
            }
            AppState::ConfigModal { .. } => {
                main_view::render(frame, self);
                config_modal::render(frame, self);
            }
            AppState::HelpModal => {
                main_view::render(frame, self);
                help_modal::render(frame, self);
            }
            AppState::PrStatusModal => {
                main_view::render(frame, self);
                crate::ui::pr_status_modal::render(frame, self);
            }
            AppState::MergeBranchSelect { .. } => {
                main_view::render(frame, self);
                crate::ui::merge_modal::render(frame, self);
            }
        }
    }

    fn handle_events<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press {
                        return Ok(());
                    }

                    // Clear message on any key press
                    self.message = None;
                    self.last_command_detail = None;

                    match self.state.clone() {
                        AppState::List => self.handle_list_input(key.code, key.modifiers),
                        AppState::CleanupPreview => self.handle_cleanup_preview_input(key.code),
                        AppState::AddModal => self.handle_add_modal_input(key.code, key.modifiers),
                        AppState::ConfirmDelete {
                            delete_branch,
                            force,
                        } => self.handle_confirm_delete_input(key.code, delete_branch, force),
                        AppState::ConfigModal {
                            selected_index,
                            editing,
                        } => self.handle_config_modal_input(key.code, selected_index, editing),
                        AppState::HelpModal => self.handle_help_modal_input(key.code),
                        AppState::PrStatusModal => self.handle_pr_status_modal_input(key.code),
                        AppState::MergeBranchSelect { branches, selected } => {
                            self.handle_merge_branch_select_input(key.code, branches, selected)
                        }
                    }
                }
                Event::Resize(_, _) => {
                    // Force a full redraw on resize
                    terminal.clear()?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_list_input(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        // Handle filter mode separately
        if self.is_filtering {
            self.handle_filter_input(code);
            return;
        }

        if self.last_key == Some('g') && code != KeyCode::Char('g') {
            self.jump_to_current_worktree();
            self.last_key = None;
            self.pending_g_since = None;
        }

        match code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
                // Half page down
                self.move_selection_half_page_down();
                self.last_key = None;
            }
            KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => {
                // Half page up
                self.move_selection_half_page_up();
                self.last_key = None;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_selection_up();
                self.last_key = None;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_selection_down();
                self.last_key = None;
            }
            KeyCode::Char('g') => {
                let is_timely_second_g = self.last_key == Some('g')
                    && self
                        .pending_g_since
                        .map(|started| started.elapsed() < G_SEQUENCE_TIMEOUT)
                        .unwrap_or(false);
                if is_timely_second_g {
                    self.move_to_top();
                    self.last_key = None;
                    self.pending_g_since = None;
                } else {
                    if self.last_key == Some('g') {
                        self.jump_to_current_worktree();
                    }
                    self.last_key = Some('g');
                    self.pending_g_since = Some(Instant::now());
                }
            }
            KeyCode::Char('G') => {
                self.move_to_bottom();
                self.last_key = None;
            }
            KeyCode::Home => {
                self.move_to_top();
                self.last_key = None;
            }
            KeyCode::End => {
                self.move_to_bottom();
                self.last_key = None;
            }
            KeyCode::Enter => {
                self.poll_background_op();
                if self.active_op.is_some() {
                    self.message = Some(AppMessage::info("Operation still in progress"));
                    self.last_key = None;
                    return;
                }
                self.enter_worktree();
                self.last_key = None;
            }
            KeyCode::Char('/') => {
                // Enter filter mode
                self.is_filtering = true;
                self.filter_text.clear();
                self.last_key = None;
            }
            KeyCode::Char('a') => {
                self.state = AppState::AddModal;
                self.input_buffer.clear();
                self.last_key = None;
            }
            KeyCode::Char('d') => {
                let targets = self.action_worktrees();
                if targets.is_empty() {
                    self.message = Some(AppMessage::error("No worktree selected"));
                } else if targets.iter().any(|wt| wt.is_bare) {
                    self.message = Some(AppMessage::error("Cannot delete bare repository"));
                } else {
                    self.state = AppState::ConfirmDelete {
                        delete_branch: false,
                        force: false,
                    };
                }
                self.last_key = None;
            }
            KeyCode::Char(' ') => {
                self.toggle_selected_worktree();
                self.last_key = None;
            }
            KeyCode::Char('o') => {
                self.open_editor();
                self.last_key = None;
            }
            KeyCode::Char('t') => {
                self.open_terminal();
                self.last_key = None;
            }
            KeyCode::Char('f') => {
                self.fetch_all();
                self.last_key = None;
            }
            KeyCode::Char('p') => {
                self.pull_worktree();
                self.last_key = None;
            }
            KeyCode::Char('P') => {
                self.push_worktree();
                self.last_key = None;
            }
            KeyCode::Char('m') => {
                self.merge_upstream();
                self.last_key = None;
            }
            KeyCode::Char('M') => {
                self.open_merge_branch_select();
                self.last_key = None;
            }
            KeyCode::Char('R') => {
                self.open_pr_status_modal();
                self.last_key = None;
            }
            KeyCode::Char('r') => {
                self.refresh_worktrees();
                self.last_key = None;
            }
            KeyCode::Char('x') => {
                self.open_cleanup_preview();
                self.last_key = None;
            }
            KeyCode::Char('s') => {
                self.cycle_sort_mode();
                self.last_key = None;
            }
            KeyCode::Char('c') => {
                self.state = AppState::ConfigModal {
                    selected_index: 0,
                    editing: false,
                };
                self.last_key = None;
            }
            KeyCode::Char('v') => {
                self.verbose = !self.verbose;
                let status = if self.verbose { "ON" } else { "OFF" };
                self.message = Some(AppMessage::info(format!("Verbose mode: {}", status)));
                self.last_key = None;
            }
            KeyCode::Char('?') => {
                self.help_scroll_offset = 0;
                self.state = AppState::HelpModal;
                self.last_key = None;
            }
            KeyCode::Char('y') => {
                self.copy_path_to_clipboard();
                self.last_key = None;
            }
            KeyCode::Esc => {
                // Clear filter if any
                if !self.filter_text.is_empty() {
                    self.filter_text.clear();
                }
                self.last_key = None;
            }
            _ => {
                self.last_key = None;
            }
        }
    }

    fn handle_filter_input(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => {
                // Cancel filter, show all worktrees
                self.is_filtering = false;
                self.filter_text.clear();
            }
            KeyCode::Enter => {
                // Exit filter mode and enter the selected worktree
                self.poll_background_op();
                if self.active_op.is_some() {
                    self.message = Some(AppMessage::info("Operation still in progress"));
                    return;
                }
                self.is_filtering = false;
                self.enter_worktree();
            }
            KeyCode::Backspace => {
                self.filter_text.pop();
            }
            KeyCode::Char(c) => {
                self.filter_text.push(c);
                // Auto-select first matching worktree
                self.select_first_filtered_worktree();
            }
            _ => {}
        }
    }

    fn resolve_pending_g_if_expired(&mut self) {
        let expired = self.last_key == Some('g')
            && self
                .pending_g_since
                .map(|started| started.elapsed() >= G_SEQUENCE_TIMEOUT)
                .unwrap_or(false);
        if expired {
            self.jump_to_current_worktree();
            self.last_key = None;
            self.pending_g_since = None;
        }
    }

    fn select_first_filtered_worktree(&mut self) {
        if self.filter_text.is_empty() {
            return;
        }
        if let Some(idx) = self
            .worktrees
            .iter()
            .position(|wt| crate::worktree_query::matches(wt, &self.filter_text))
        {
            self.selected_index = idx;
        }
    }

    fn handle_add_modal_input(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        match code {
            KeyCode::Esc => {
                self.state = AppState::List;
                self.input_buffer.clear();
                self.add_worktree_path.clear();
                self.add_modal_field = AddModalField::Branch;
                self.add_tmux_override = None;
            }
            KeyCode::Enter => {
                if !self.input_buffer.trim().is_empty() {
                    self.queue_worktree_create_after_exit();
                }
            }
            KeyCode::Tab => {
                self.cycle_add_base_branch();
            }
            KeyCode::Char('p') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.add_modal_field = match self.add_modal_field {
                    AddModalField::Branch => AddModalField::WorktreePath,
                    AddModalField::WorktreePath => AddModalField::Branch,
                };
            }
            KeyCode::Char('t') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.add_tmux_override = match self.add_tmux_override {
                    None => Some(true),
                    Some(true) => Some(false),
                    Some(false) => None,
                };
            }
            KeyCode::Backspace => {
                self.active_add_input().pop();
            }
            KeyCode::Char(c) => {
                self.active_add_input().push(c);
            }
            _ => {}
        }
    }

    fn active_add_input(&mut self) -> &mut String {
        match self.add_modal_field {
            AddModalField::Branch => &mut self.input_buffer,
            AddModalField::WorktreePath => &mut self.add_worktree_path,
        }
    }

    fn handle_confirm_delete_input(&mut self, code: KeyCode, delete_branch: bool, force: bool) {
        match code {
            KeyCode::Esc | KeyCode::Char('n') => {
                self.state = AppState::List;
            }
            KeyCode::Char('y') | KeyCode::Enter => {
                // Require force for dirty worktrees
                let dirty_targets: Vec<String> = self
                    .action_worktrees()
                    .into_iter()
                    .filter(|wt| wt.status != WorktreeStatus::Clean)
                    .map(|wt| wt.display_name())
                    .collect();
                if !dirty_targets.is_empty() && !force {
                    let message = if dirty_targets.len() == 1 {
                        "Worktree has uncommitted changes. Press 'f' to enable force delete."
                            .to_string()
                    } else {
                        format!(
                            "Worktree has uncommitted changes. Press 'f' to enable force delete: {}",
                            dirty_targets.join(", ")
                        )
                    };
                    self.message = Some(AppMessage::error(message));
                    return;
                }
                self.delete_selected_worktree(delete_branch, force);
            }
            KeyCode::Char('b') => {
                // Toggle delete branch option
                self.state = AppState::ConfirmDelete {
                    delete_branch: !delete_branch,
                    force,
                };
            }
            KeyCode::Char('f') => {
                // Toggle force delete option
                self.state = AppState::ConfirmDelete {
                    delete_branch,
                    force: !force,
                };
            }
            _ => {}
        }
    }

    fn handle_config_modal_input(&mut self, code: KeyCode, selected: usize, editing: bool) {
        use crate::ui::config_modal::CONFIG_ITEM_COUNT;

        if editing {
            match code {
                KeyCode::Esc => {
                    // Cancel editing, restore to navigation mode
                    self.input_buffer.clear();
                    self.state = AppState::ConfigModal {
                        selected_index: selected,
                        editing: false,
                    };
                }
                KeyCode::Enter => {
                    // Save the edited value
                    self.apply_config_edit(selected);
                    self.input_buffer.clear();
                    self.state = AppState::ConfigModal {
                        selected_index: selected,
                        editing: false,
                    };
                }
                KeyCode::Char(c) => {
                    self.input_buffer.push(c);
                }
                KeyCode::Backspace => {
                    self.input_buffer.pop();
                }
                _ => {}
            }
        } else {
            match code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.state = AppState::List;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    let new_index = if selected > 0 { selected - 1 } else { 0 };
                    self.state = AppState::ConfigModal {
                        selected_index: new_index,
                        editing: false,
                    };
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let new_index = if selected < CONFIG_ITEM_COUNT - 1 {
                        selected + 1
                    } else {
                        CONFIG_ITEM_COUNT - 1
                    };
                    self.state = AppState::ConfigModal {
                        selected_index: new_index,
                        editing: false,
                    };
                }
                KeyCode::Enter => {
                    if selected == 4 {
                        self.config.tmux_worktree_mode = !self.config.tmux_worktree_mode;
                        let state = if self.config.tmux_worktree_mode {
                            "enabled"
                        } else {
                            "disabled"
                        };
                        self.message = Some(AppMessage::info(format!(
                            "Tmux worktree mode {} (press 's' to save to file)",
                            state
                        )));
                    } else if selected == 5 {
                        self.message = Some(AppMessage::info(
                            "Post-add auto-run can only be changed in global config",
                        ));
                    } else if selected == 6 {
                        self.open_post_add_script_editor();
                    } else {
                        self.input_buffer = self.get_config_value_for_editing(selected);
                        self.state = AppState::ConfigModal {
                            selected_index: selected,
                            editing: true,
                        };
                    }
                }
                KeyCode::Char('s') => {
                    // Save config to file
                    self.save_config();
                }
                _ => {}
            }
        }
    }

    fn get_config_value_for_editing(&self, index: usize) -> String {
        match index {
            0 => self.config.editor.clone().unwrap_or_default(),
            1 => self.config.terminal.clone().unwrap_or_default(),
            2 => self.config.worktree_root.clone().unwrap_or_default(),
            3 => self.config.copy_files.join(", "),
            _ => String::new(),
        }
    }

    fn apply_config_edit(&mut self, index: usize) {
        let value = self.input_buffer.trim().to_string();
        match index {
            0 => {
                // editor
                self.config.editor = if value.is_empty() { None } else { Some(value) };
            }
            1 => {
                // terminal
                self.config.terminal = if value.is_empty() { None } else { Some(value) };
            }
            2 => {
                // worktree_root
                self.config.worktree_root = if value.is_empty() { None } else { Some(value) };
            }
            3 => {
                // copy_files
                self.config.copy_files = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            _ => {}
        }
        self.message = Some(AppMessage::info(
            "Setting updated (press 's' to save to file)",
        ));
    }

    fn save_config(&mut self) {
        match self.config.save_to_project(&self.project_root_path) {
            Ok(()) => {
                self.message = Some(AppMessage::info("Config saved"));
            }
            Err(e) => {
                self.message = Some(AppMessage::error(format!("Failed to save config: {}", e)));
            }
        }
    }

    fn open_post_add_script_editor(&mut self) {
        let script_path = self
            .config
            .resolved_post_add_script_path(&self.project_root_path);
        let editor = self.config.get_editor();

        // Create .owt directory and script file if they don't exist
        if let Some(parent) = script_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if !script_path.exists() {
            let default_content = "#!/bin/bash\n# Post-add script: runs after creating a new worktree\n# Working directory is the new worktree path\n\n";
            let _ = fs::write(&script_path, default_content);
        }

        // Restore terminal before opening editor
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);

        let status = Command::new(&editor).arg(&script_path).status();

        // Restore terminal after editor closes
        let _ = crossterm::terminal::enable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen);

        match status {
            Ok(s) if s.success() => {
                self.message = Some(AppMessage::info("Script editor closed"));
            }
            Ok(_) => {
                self.message = Some(AppMessage::error("Editor exited with error"));
            }
            Err(e) => {
                self.message = Some(AppMessage::error(format!("Failed to open editor: {}", e)));
            }
        }
    }

    fn handle_help_modal_input(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
                self.state = AppState::List;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.help_scroll_offset = self.help_scroll_offset.saturating_add(1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.help_scroll_offset = self.help_scroll_offset.saturating_sub(1);
            }
            _ => {}
        }
    }

    fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.update_selected_details();
        }
    }

    fn move_selection_down(&mut self) {
        if self.selected_index < self.worktrees.len().saturating_sub(1) {
            self.selected_index += 1;
            self.update_selected_details();
        }
    }

    fn move_to_top(&mut self) {
        self.selected_index = 0;
        self.update_selected_details();
    }

    fn move_to_bottom(&mut self) {
        self.selected_index = self.worktrees.len().saturating_sub(1);
        self.update_selected_details();
    }

    fn move_selection_half_page_down(&mut self) {
        let vh = self.viewport_height.get();
        let half_page = if vh > 0 { (vh / 2) as usize } else { 10 };
        let max_index = self.worktrees.len().saturating_sub(1);
        self.selected_index = (self.selected_index + half_page).min(max_index);
        self.update_selected_details();
    }

    fn move_selection_half_page_up(&mut self) {
        let vh = self.viewport_height.get();
        let half_page = if vh > 0 { (vh / 2) as usize } else { 10 };
        self.selected_index = self.selected_index.saturating_sub(half_page);
        self.update_selected_details();
    }

    fn jump_to_current_worktree(&mut self) {
        if let Some(ref current_path) = self.current_worktree_path {
            if let Some(idx) = self
                .worktrees
                .iter()
                .position(|wt| wt.path == *current_path)
            {
                self.selected_index = idx;
                self.message = Some(AppMessage::info("Jumped to current worktree"));
                self.update_selected_details();
            }
        } else {
            self.message = Some(AppMessage::error("No current worktree detected"));
        }
    }

    pub fn selected_worktree(&self) -> Option<&Worktree> {
        self.worktrees.get(self.selected_index)
    }

    pub fn is_worktree_marked(&self, path: &Path) -> bool {
        self.selected_worktree_paths.contains(path)
    }

    pub fn selected_worktree_count(&self) -> usize {
        self.selected_worktree_paths.len()
    }

    fn toggle_selected_worktree(&mut self) {
        let Some(wt) = self.selected_worktree() else {
            return;
        };

        if wt.is_bare {
            self.message = Some(AppMessage::error("Cannot select bare repository"));
            return;
        }

        let path = wt.path.clone();
        let name = wt.display_name();
        if self.selected_worktree_paths.remove(&path) {
            self.message = Some(AppMessage::info(format!("Unselected {}", name)));
        } else {
            self.selected_worktree_paths.insert(path);
            self.message = Some(AppMessage::info(format!("Selected {}", name)));
        }
    }

    pub fn action_worktrees(&self) -> Vec<Worktree> {
        if self.selected_worktree_paths.is_empty() {
            return self.selected_worktree().cloned().into_iter().collect();
        }

        self.worktrees
            .iter()
            .filter(|wt| self.selected_worktree_paths.contains(&wt.path))
            .cloned()
            .collect()
    }

    fn worktree_path_for_branch(&self, branch: &str) -> PathBuf {
        if self.repo_is_bare {
            return self
                .bare_repo_path
                .parent()
                .map(|p| p.join(branch))
                .unwrap_or_else(|| PathBuf::from(branch));
        }

        self.config
            .resolved_worktree_root()
            .join(self.repo_namespace())
            .join(branch)
    }

    fn repo_namespace(&self) -> String {
        self.project_root_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "repo".to_string())
    }

    fn conflicting_worktree_for_branch(
        &self,
        branch: &str,
        desired_path: &Path,
    ) -> Option<&Worktree> {
        self.worktrees.iter().find(|wt| {
            !wt.is_bare
                && wt.branch.as_deref() == Some(branch)
                && !paths_refer_to_same_location(&wt.path, desired_path)
        })
    }

    pub fn add_modal_base_label(&self) -> String {
        format!("Base branch: {}", self.add_base_branch)
    }

    pub fn add_modal_tmux_label(&self) -> &'static str {
        match self.add_tmux_override {
            None => "default",
            Some(true) => "on",
            Some(false) => "off",
        }
    }

    fn cycle_add_base_branch(&mut self) {
        match git::list_local_branches(&self.bare_repo_path) {
            Ok(branches) if branches.is_empty() => {
                self.message = Some(AppMessage::error("No local branches available"));
            }
            Ok(branches) => {
                let current = branches
                    .iter()
                    .position(|branch| branch == &self.add_base_branch);
                let next = current.map_or(0, |idx| (idx + 1) % branches.len());
                self.add_base_branch = branches[next].clone();
                self.message = Some(AppMessage::info(format!(
                    "Base branch: {}",
                    self.add_base_branch
                )));
            }
            Err(e) => {
                self.message = Some(AppMessage::error(format!(
                    "Failed to list base branches: {}",
                    e
                )));
            }
        }
    }

    fn refresh_worktrees(&mut self) {
        match git::list_worktrees(&self.bare_repo_path) {
            Ok(worktrees) => {
                self.worktrees = worktrees;
                self.prune_missing_selected_paths();
                self.apply_sort();
                if self.selected_index >= self.worktrees.len() {
                    self.selected_index = self.worktrees.len().saturating_sub(1);
                }
                self.update_selected_details();
                self.start_pr_status_refresh();
                self.message = Some(AppMessage::info("Refreshed"));
            }
            Err(e) => {
                self.message = Some(AppMessage::error(format!("Failed to refresh: {}", e)));
            }
        }
    }

    fn cycle_sort_mode(&mut self) {
        self.sort_mode = self.sort_mode.next();
        self.apply_sort();
        self.message = Some(AppMessage::info(format!(
            "Sort: {}",
            self.sort_mode.label()
        )));
    }

    fn apply_sort(&mut self) {
        // Remember currently selected worktree path
        let selected_path = self.selected_worktree().map(|wt| wt.path.clone());

        match self.sort_mode {
            SortMode::Name => {
                self.worktrees.sort_by(|a, b| {
                    // Bare repo always first
                    if a.is_bare && !b.is_bare {
                        return std::cmp::Ordering::Less;
                    }
                    if !a.is_bare && b.is_bare {
                        return std::cmp::Ordering::Greater;
                    }
                    a.display_name()
                        .to_lowercase()
                        .cmp(&b.display_name().to_lowercase())
                });
            }
            SortMode::Recent => {
                self.worktrees.sort_by(|a, b| {
                    // Bare repo always first
                    if a.is_bare && !b.is_bare {
                        return std::cmp::Ordering::Less;
                    }
                    if !a.is_bare && b.is_bare {
                        return std::cmp::Ordering::Greater;
                    }
                    // Sort by last commit time (most recent first)
                    b.last_commit_timestamp.cmp(&a.last_commit_timestamp)
                });
            }
            SortMode::Status => {
                self.worktrees.sort_by(|a, b| {
                    // Bare repo always first
                    if a.is_bare && !b.is_bare {
                        return std::cmp::Ordering::Less;
                    }
                    if !a.is_bare && b.is_bare {
                        return std::cmp::Ordering::Greater;
                    }
                    // Sort by status priority (dirty first)
                    let status_order = |s: &WorktreeStatus| match s {
                        WorktreeStatus::Conflict => 0,
                        WorktreeStatus::Mixed => 1,
                        WorktreeStatus::Unstaged => 2,
                        WorktreeStatus::Staged => 3,
                        WorktreeStatus::Unknown => 4,
                        WorktreeStatus::Clean => 5,
                    };
                    status_order(&a.status).cmp(&status_order(&b.status))
                });
            }
        }

        // Restore selection to same worktree after sort
        if let Some(ref path) = selected_path {
            if let Some(idx) = self.worktrees.iter().position(|wt| wt.path == *path) {
                self.selected_index = idx;
            }
        }
        self.update_selected_details();
    }

    fn prune_missing_selected_paths(&mut self) {
        let existing_paths: HashSet<PathBuf> =
            self.worktrees.iter().map(|wt| wt.path.clone()).collect();
        self.selected_worktree_paths
            .retain(|path| existing_paths.contains(path));
    }

    fn clamp_selection_to_non_bare(&mut self) {
        if self.worktrees.is_empty() {
            self.selected_index = 0;
            return;
        }

        if self.selected_index >= self.worktrees.len() {
            self.selected_index = self.worktrees.len().saturating_sub(1);
        }

        if self
            .worktrees
            .get(self.selected_index)
            .map(|wt| wt.is_bare)
            .unwrap_or(false)
        {
            if let Some(idx) = self.worktrees.iter().position(|wt| !wt.is_bare) {
                self.selected_index = idx;
            }
        }
    }

    fn update_selected_details(&mut self) {
        self.selected_details = self
            .selected_worktree()
            .filter(|wt| !wt.is_bare)
            .and_then(|wt| git::get_worktree_details(&wt.path).ok());
    }

    fn queue_worktree_create_after_exit(&mut self) {
        if self.active_op.is_some() {
            self.message = Some(AppMessage::error("Another operation is in progress"));
            self.state = AppState::List;
            return;
        }

        let branch = self.input_buffer.trim().to_string();
        if branch.is_empty() {
            self.message = Some(AppMessage::error("Branch name cannot be empty"));
            return;
        }

        let worktree_path = if self.add_worktree_path.trim().is_empty() {
            self.worktree_path_for_branch(&branch)
        } else {
            PathBuf::from(self.add_worktree_path.trim())
        };
        if let Some(existing) = self.conflicting_worktree_for_branch(&branch, &worktree_path) {
            self.message = Some(AppMessage::error(format!(
                "Branch '{}' is already checked out at {}. Remove or move that worktree first.",
                branch,
                existing.path.display()
            )));
            self.state = AppState::List;
            return;
        }

        let source_path = self.current_worktree_path.clone().or_else(|| {
            self.worktrees
                .iter()
                .find(|wt| !wt.is_bare)
                .map(|wt| wt.path.clone())
        });

        self.exit_action = ExitAction::CreateWorktree(WorktreeCreateRequest {
            bare_repo_path: self.bare_repo_path.clone(),
            project_root_path: self.project_root_path.clone(),
            branch,
            base_branch: self.add_base_branch.clone(),
            worktree_path,
            source_path,
            tmux: self.add_tmux_override,
        });
        self.should_quit = true;
        self.state = AppState::List;
        self.input_buffer.clear();
        self.add_worktree_path.clear();
        self.add_modal_field = AddModalField::Branch;
        self.add_tmux_override = None;
    }

    fn run_post_add_script(&mut self, worktree_path: &Path) {
        let script_path = self
            .config
            .resolved_post_add_script_path(&self.project_root_path);

        if !script_path.exists() {
            return;
        }

        if !self.config.run_post_add_script_in_tmux {
            return;
        }

        let worktree_name = worktree_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let session_name = format!("owt-post-add-{}-{}", std::process::id(), self.spinner_tick);
        let command = format!(
            "cd {} && sh {}; status=$?; tmux kill-session -t {}; exit $status",
            shell_quote(worktree_path),
            shell_quote(&script_path),
            session_name
        );
        let output = Command::new("tmux")
            .args(["new-session", "-d", "-s", &session_name, &command])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let (tx, rx) = mpsc::channel();
                self.script_status = ScriptStatus::Running {
                    worktree_name: worktree_name.clone(),
                };
                self.script_receiver = Some(rx);
                let _ = tx.send(ScriptResult {
                    success: true,
                    message: format!("launched in tmux for {}", worktree_name),
                });
                self.message = Some(AppMessage::info(format!(
                    "Setup script launched in tmux for {}",
                    worktree_name
                )));
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let detail = if stderr.is_empty() { stdout } else { stderr };
                self.message = Some(AppMessage::error(format!(
                    "Failed to launch setup script in tmux: {}",
                    detail
                )));
            }
            Err(e) => {
                self.message = Some(AppMessage::error(format!(
                    "Failed to launch setup script in tmux: {}",
                    e
                )));
            }
        }
    }

    fn delete_selected_worktree(&mut self, delete_branch: bool, force: bool) {
        if self.active_op.is_some() {
            self.message = Some(AppMessage::error("Another operation is in progress"));
            self.state = AppState::List;
            return;
        }

        let worktrees = self.action_worktrees();
        if worktrees.is_empty() {
            return;
        }

        if worktrees.iter().any(|wt| wt.is_bare) {
            self.message = Some(AppMessage::error("Cannot delete bare repository"));
            self.state = AppState::List;
            return;
        }

        let dirty_worktrees: Vec<String> = worktrees
            .iter()
            .filter(|wt| wt.status != WorktreeStatus::Clean)
            .map(Worktree::display_name)
            .collect();
        if !dirty_worktrees.is_empty() && !force {
            let message = if worktrees.len() == 1 {
                "Worktree has uncommitted changes. Press 'f' to enable force delete.".to_string()
            } else {
                format!(
                    "Worktree has uncommitted changes. Press 'f' to enable force delete: {}",
                    dirty_worktrees.join(", ")
                )
            };
            self.message = Some(AppMessage::error(message));
            return;
        }

        let display_name = batch_display_name(&worktrees, "worktree");
        let display_name_for_thread = display_name.clone();
        let display_name_for_state = display_name.clone();
        let worktree_path_for_state = worktrees[0].path.clone();
        let worktree_paths_for_state: Vec<PathBuf> =
            worktrees.iter().map(|wt| wt.path.clone()).collect();

        let force_flag = if force { " --force" } else { "" };
        let cmd_detail = worktrees
            .iter()
            .map(|wt| {
                format!(
                    "git -C {} worktree remove{} {}",
                    self.bare_repo_path.display(),
                    force_flag,
                    wt.path.display()
                )
            })
            .collect::<Vec<_>>()
            .join("\n$ ");

        self.state = AppState::List;
        self.message = Some(AppMessage::info(format!("Deleting: {}...", display_name)));

        let bare_repo_path = self.bare_repo_path.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut deleted = Vec::new();
            let mut failures = Vec::new();
            let total = worktrees.len();

            for wt in worktrees {
                let name = wt.display_name();
                match git::remove_worktree(&bare_repo_path, &wt.path, force) {
                    Ok(()) => {
                        deleted.push(wt.path.clone());
                        if delete_branch {
                            if let Some(ref branch) = wt.branch {
                                if let Err(e) = git::delete_branch(&bare_repo_path, branch, force) {
                                    failures.push(format!("{} branch delete failed: {}", name, e));
                                }
                            }
                        }
                    }
                    Err(e) => failures.push(format!("{}: {}", name, e)),
                }
            }

            let message = if failures.is_empty() {
                if total == 1 {
                    format!("Deleted worktree: {}", display_name_for_thread)
                } else {
                    format!("Deleted {} worktrees", deleted.len())
                }
            } else {
                format!(
                    "Deleted {}/{} worktrees. Failed: {}",
                    deleted.len(),
                    total,
                    failures.join("; ")
                )
            };

            let worktree_path = deleted
                .first()
                .cloned()
                .unwrap_or_else(|| PathBuf::from("."));
            let success = failures.is_empty();
            let _ = tx.send(OpResult {
                kind: OpKind::Delete,
                success,
                message,
                cmd_detail,
                worktree_path,
                affected_paths: deleted,
            });
        });

        self.active_op = Some((OpKind::Delete, rx));
        self.active_op_info = Some(ActiveOp {
            kind: OpKind::Delete,
            worktree_path: worktree_path_for_state,
            worktree_paths: worktree_paths_for_state,
            display_name: display_name_for_state,
        });
    }

    fn open_cleanup_preview(&mut self) {
        if self.active_op.is_some() {
            self.message = Some(AppMessage::info("Operation still in progress"));
            return;
        }

        let launch_path = self
            .current_worktree_path
            .clone()
            .unwrap_or_else(|| self.project_root_path.clone());
        match crate::worktree_prune::run_prune(
            &self.bare_repo_path,
            &launch_path,
            crate::worktree_prune::PruneMode::Preview,
        ) {
            Ok(report) => {
                self.cleanup_preview = Some(report);
                self.state = AppState::CleanupPreview;
            }
            Err(error) => {
                self.message = Some(AppMessage::error(format!(
                    "Cleanup preview failed: {}",
                    error
                )));
            }
        }
    }

    fn handle_cleanup_preview_input(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc | KeyCode::Char('n') => {
                self.cleanup_preview = None;
                self.state = AppState::List;
            }
            KeyCode::Char('y') | KeyCode::Enter => self.execute_cleanup(),
            _ => {}
        }
    }

    fn execute_cleanup(&mut self) {
        if self.active_op.is_some() {
            self.message = Some(AppMessage::info("Operation still in progress"));
            return;
        }

        let candidate_count = self
            .cleanup_preview
            .as_ref()
            .map(crate::worktree_prune::PruneReport::candidate_count)
            .unwrap_or(0);
        let bare_repo_path = self.bare_repo_path.clone();
        let launch_path = self
            .current_worktree_path
            .clone()
            .unwrap_or_else(|| self.project_root_path.clone());
        let display_name = format!("{} cleanup candidate(s)", candidate_count);
        let display_name_for_state = display_name.clone();
        let cmd_detail = format!("owt worktree prune --path {}", bare_repo_path.display());
        let cmd_detail_for_thread = cmd_detail.clone();
        let (tx, rx) = mpsc::channel();

        std::thread::spawn(move || {
            let result = crate::worktree_prune::run_prune(
                &bare_repo_path,
                &launch_path,
                crate::worktree_prune::PruneMode::Execute,
            );
            let (success, message, affected_paths) = match result {
                Ok(report) => {
                    let removed_paths = report
                        .logs
                        .iter()
                        .filter(|log| {
                            matches!(
                                log.action,
                                crate::worktree_prune::PruneWorktreeAction::Removed
                            )
                        })
                        .map(|log| log.path.clone())
                        .collect::<Vec<_>>();
                    (
                        true,
                        format!(
                            "Cleanup completed: removed {} worktree(s)",
                            report.removed_count()
                        ),
                        removed_paths,
                    )
                }
                Err(error) => (false, error.to_string(), Vec::new()),
            };
            let worktree_path = affected_paths
                .first()
                .cloned()
                .unwrap_or_else(|| PathBuf::from("."));
            let _ = tx.send(OpResult {
                kind: OpKind::Prune,
                success,
                message,
                cmd_detail: cmd_detail_for_thread,
                worktree_path,
                affected_paths,
            });
        });

        self.cleanup_preview = None;
        self.state = AppState::List;
        self.message = Some(AppMessage::info(format!("Cleaning up: {}", display_name)));
        self.active_op = Some((OpKind::Prune, rx));
        self.active_op_info = Some(ActiveOp {
            kind: OpKind::Prune,
            worktree_path: PathBuf::from("."),
            worktree_paths: Vec::new(),
            display_name: display_name_for_state,
        });
    }

    fn open_editor(&mut self) {
        if let Some(wt) = self.selected_worktree() {
            if wt.is_bare {
                self.message = Some(AppMessage::error("Cannot open bare repository in editor"));
                return;
            }

            let editor = self.config.get_editor();
            let path = wt.path.clone();

            // We need to restore terminal before opening editor
            let _ = crossterm::terminal::disable_raw_mode();
            let _ =
                crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);

            let status = Command::new(&editor).arg(&path).status();

            // Restore terminal after editor closes
            let _ = crossterm::terminal::enable_raw_mode();
            let _ =
                crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen);

            match status {
                Ok(s) if s.success() => {
                    self.refresh_worktrees();
                }
                Ok(_) => {
                    self.message = Some(AppMessage::error("Editor exited with error"));
                }
                Err(e) => {
                    self.message = Some(AppMessage::error(format!("Failed to open editor: {}", e)));
                }
            }
        }
    }

    fn open_terminal(&mut self) {
        if let Some(wt) = self.selected_worktree() {
            if wt.is_bare {
                self.message = Some(AppMessage::error("Cannot open bare repository in terminal"));
                return;
            }

            #[cfg(any(target_os = "macos", target_os = "linux"))]
            let path = wt.path.clone();
            #[cfg(any(target_os = "macos", target_os = "linux"))]
            let terminal = self.config.get_terminal();

            #[cfg(target_os = "macos")]
            let result = {
                let app = terminal.as_deref().unwrap_or("Terminal");
                Command::new("open")
                    .args(["-a", app, &path.to_string_lossy()])
                    .status()
            };

            #[cfg(target_os = "linux")]
            let result = if let Some(term) = terminal {
                Command::new(&term).current_dir(&path).status()
            } else {
                Command::new("x-terminal-emulator")
                    .arg("--working-directory")
                    .arg(&path)
                    .status()
                    .or_else(|_| {
                        Command::new("gnome-terminal")
                            .arg("--working-directory")
                            .arg(&path)
                            .status()
                    })
            };

            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            let result: Result<std::process::ExitStatus, std::io::Error> =
                Err(std::io::Error::other("Unsupported platform"));

            match result {
                Ok(s) if s.success() => {
                    self.message = Some(AppMessage::info("Opened terminal"));
                }
                Ok(_) => {
                    self.message = Some(AppMessage::error("Failed to open terminal"));
                }
                Err(e) => {
                    self.message =
                        Some(AppMessage::error(format!("Failed to open terminal: {}", e)));
                }
            }
        }
    }

    fn fetch_all(&mut self) {
        if self.active_op.is_some() {
            self.message = Some(AppMessage::error("Another operation is in progress"));
            return;
        }

        let wt = match self.selected_worktree().cloned() {
            Some(wt) if !wt.is_bare => wt,
            Some(_) => {
                self.message = Some(AppMessage::error("Cannot fetch bare repository"));
                return;
            }
            Option::None => return,
        };

        let display_name = wt.display_name();
        let display_name_for_thread = display_name.clone();
        let display_name_for_state = display_name.clone();
        let worktree_path = wt.path.clone();
        let worktree_path_for_thread = worktree_path.clone();
        let worktree_path_for_state = worktree_path.clone();
        let cmd_detail = format!("git -C {} fetch origin", worktree_path.display());
        let cmd_detail_for_thread = cmd_detail.clone();

        self.state = AppState::List;
        self.message = Some(AppMessage::info(format!("Fetching: {}...", display_name)));

        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = git::fetch_worktree(&worktree_path_for_thread);
            let _ = tx.send(OpResult {
                kind: OpKind::Fetch,
                success: result.is_ok(),
                message: match &result {
                    Ok(()) => format!("Fetch completed: {}", display_name_for_thread),
                    Err(e) => format!("Fetch failed: {}", e),
                },
                cmd_detail: cmd_detail_for_thread,
                worktree_path: worktree_path_for_thread.clone(),
                affected_paths: vec![worktree_path_for_thread.clone()],
            });
        });

        self.active_op = Some((OpKind::Fetch, rx));
        self.active_op_info = Some(ActiveOp {
            kind: OpKind::Fetch,
            worktree_path: worktree_path_for_state.clone(),
            worktree_paths: vec![worktree_path_for_state.clone()],
            display_name: display_name_for_state,
        });
    }

    fn enter_worktree(&mut self) {
        if let Some(wt) = self.selected_worktree().cloned() {
            if wt.is_bare {
                self.message = Some(AppMessage::error("Cannot enter bare repository"));
                return;
            }
            if self.config.tmux_worktree_mode
                && tmux::focus_pane_named(&wt.display_name()).unwrap_or(false)
            {
                self.exit_action = ExitAction::Quit;
                self.should_quit = true;
                return;
            }
            // Always allow enter - even without shell integration, we print the path
            // The shell wrapper function (from `owt setup`) will handle the cd
            self.exit_action = ExitAction::ChangeDirectory(wt.path.clone());
            self.should_quit = true;
        } else {
            self.message = Some(AppMessage::error("No worktree selected"));
        }
    }

    fn copy_path_to_clipboard(&mut self) {
        if let Some(wt) = self.selected_worktree() {
            let path_str = wt.path.to_string_lossy().to_string();

            #[cfg(target_os = "macos")]
            let result = {
                use std::io::Write;
                Command::new("pbcopy")
                    .stdin(std::process::Stdio::piped())
                    .spawn()
                    .and_then(|mut child| {
                        if let Some(mut stdin) = child.stdin.take() {
                            stdin.write_all(path_str.as_bytes())?;
                        }
                        child.wait()
                    })
            };

            #[cfg(target_os = "linux")]
            let result = {
                use std::io::Write;
                // Try xclip first, then xsel
                Command::new("xclip")
                    .args(["-selection", "clipboard"])
                    .stdin(std::process::Stdio::piped())
                    .spawn()
                    .or_else(|_| {
                        Command::new("xsel")
                            .args(["--clipboard", "--input"])
                            .stdin(std::process::Stdio::piped())
                            .spawn()
                    })
                    .and_then(|mut child| {
                        if let Some(mut stdin) = child.stdin.take() {
                            stdin.write_all(path_str.as_bytes())?;
                        }
                        child.wait()
                    })
            };

            #[cfg(not(any(target_os = "macos", target_os = "linux")))]
            let result: Result<std::process::ExitStatus, std::io::Error> = Err(
                std::io::Error::other("Clipboard not supported on this platform"),
            );

            match result {
                Ok(status) if status.success() => {
                    self.message = Some(AppMessage::info(format!("Copied: {}", path_str)));
                }
                _ => {
                    self.message = Some(AppMessage::error("Failed to copy to clipboard"));
                }
            }
        }
    }

    fn pull_worktree(&mut self) {
        if self.active_op.is_some() {
            self.message = Some(AppMessage::error("Another operation is in progress"));
            return;
        }

        let worktrees = self.action_worktrees();
        if worktrees.is_empty() {
            return;
        }

        if worktrees.iter().any(|wt| wt.is_bare) {
            self.message = Some(AppMessage::error("Cannot pull bare repository"));
            return;
        }
        let dirty_worktrees: Vec<String> = worktrees
            .iter()
            .filter(|wt| wt.status != WorktreeStatus::Clean)
            .map(Worktree::display_name)
            .collect();
        if !dirty_worktrees.is_empty() {
            self.message = Some(AppMessage::error(format!(
                "Cannot pull: worktree has uncommitted changes: {}",
                dirty_worktrees.join(", ")
            )));
            return;
        }

        let display_name = batch_display_name(&worktrees, "worktree");
        let display_name_for_thread = display_name.clone();
        let display_name_for_state = display_name.clone();
        let worktree_path_for_state = worktrees[0].path.clone();
        let worktree_paths_for_state: Vec<PathBuf> =
            worktrees.iter().map(|wt| wt.path.clone()).collect();
        let cmd_detail = worktrees
            .iter()
            .map(|wt| format!("git -C {} pull", wt.path.display()))
            .collect::<Vec<_>>()
            .join("\n$ ");

        self.state = AppState::List;
        self.message = Some(AppMessage::info(format!("Pulling: {}...", display_name)));

        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut pulled = Vec::new();
            let mut failures = Vec::new();
            let total = worktrees.len();

            for wt in worktrees {
                let name = wt.display_name();
                match git::pull_worktree(&wt.path) {
                    Ok(_) => pulled.push(wt.path.clone()),
                    Err(e) => failures.push(format!("{}: {}", name, e)),
                }
            }

            let message = if failures.is_empty() {
                if total == 1 {
                    format!("Pull completed: {}", display_name_for_thread)
                } else {
                    format!("Pull completed: {} worktrees", pulled.len())
                }
            } else {
                format!(
                    "Pulled {}/{} worktrees. Failed: {}",
                    pulled.len(),
                    total,
                    failures.join("; ")
                )
            };

            let worktree_path = pulled
                .first()
                .cloned()
                .unwrap_or_else(|| PathBuf::from("."));
            let _ = tx.send(OpResult {
                kind: OpKind::Pull,
                success: failures.is_empty(),
                message,
                cmd_detail,
                worktree_path,
                affected_paths: pulled,
            });
        });

        self.active_op = Some((OpKind::Pull, rx));
        self.active_op_info = Some(ActiveOp {
            kind: OpKind::Pull,
            worktree_path: worktree_path_for_state,
            worktree_paths: worktree_paths_for_state,
            display_name: display_name_for_state,
        });
    }

    fn push_worktree(&mut self) {
        if self.active_op.is_some() {
            self.message = Some(AppMessage::error("Another operation is in progress"));
            return;
        }

        let wt = match self.selected_worktree().cloned() {
            Some(wt) => wt,
            Option::None => return,
        };

        if wt.is_bare {
            self.message = Some(AppMessage::error("Cannot push bare repository"));
            return;
        }

        let display_name = wt.display_name();
        let display_name_for_thread = display_name.clone();
        let display_name_for_state = display_name.clone();
        let worktree_path = wt.path.clone();
        let worktree_path_for_thread = worktree_path.clone();
        let worktree_path_for_state = worktree_path.clone();
        let cmd_detail = format!("git -C {} push", worktree_path.display());
        let cmd_detail_for_thread = cmd_detail.clone();

        self.state = AppState::List;
        self.message = Some(AppMessage::info(format!("Pushing: {}...", display_name)));

        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = git::push_worktree(&worktree_path_for_thread);
            let message = match &result {
                Ok(msg) => {
                    if msg.is_empty() || msg.contains("Everything up-to-date") {
                        "Push completed: Everything up-to-date".to_string()
                    } else {
                        format!("Push completed: {}", display_name_for_thread)
                    }
                }
                Err(e) => format!("Push failed: {}", e),
            };

            let _ = tx.send(OpResult {
                kind: OpKind::Push,
                success: result.is_ok(),
                message,
                cmd_detail: cmd_detail_for_thread,
                worktree_path: worktree_path_for_thread.clone(),
                affected_paths: vec![worktree_path_for_thread.clone()],
            });
        });

        self.active_op = Some((OpKind::Push, rx));
        self.active_op_info = Some(ActiveOp {
            kind: OpKind::Push,
            worktree_path: worktree_path_for_state.clone(),
            worktree_paths: vec![worktree_path_for_state.clone()],
            display_name: display_name_for_state,
        });
    }

    fn merge_upstream(&mut self) {
        self.start_merge(None);
    }

    fn open_merge_branch_select(&mut self) {
        if self.active_op.is_some() {
            self.message = Some(AppMessage::error("Another operation is in progress"));
            return;
        }

        let wt_info = self
            .selected_worktree()
            .map(|wt| (wt.is_bare, wt.status.clone()));

        if let Some((is_bare, status)) = wt_info {
            if is_bare {
                self.message = Some(AppMessage::error("Cannot merge into bare repository"));
                return;
            }
            if status != WorktreeStatus::Clean {
                self.message = Some(AppMessage::error(
                    "Cannot merge: worktree has uncommitted changes",
                ));
                return;
            }

            match git::list_local_branches(&self.bare_repo_path) {
                Ok(branches) => {
                    if branches.is_empty() {
                        self.message = Some(AppMessage::error("No branches available to merge"));
                        return;
                    }
                    self.state = AppState::MergeBranchSelect {
                        branches,
                        selected: 0,
                    };
                }
                Err(e) => {
                    self.message =
                        Some(AppMessage::error(format!("Failed to list branches: {}", e)));
                }
            }
        }
    }

    fn handle_merge_branch_select_input(
        &mut self,
        code: KeyCode,
        branches: Vec<String>,
        selected: usize,
    ) {
        match code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.state = AppState::List;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let new_selected = if selected > 0 { selected - 1 } else { 0 };
                self.state = AppState::MergeBranchSelect {
                    branches,
                    selected: new_selected,
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let new_selected = if selected < branches.len().saturating_sub(1) {
                    selected + 1
                } else {
                    branches.len().saturating_sub(1)
                };
                self.state = AppState::MergeBranchSelect {
                    branches,
                    selected: new_selected,
                };
            }
            KeyCode::Enter => {
                if let Some(branch) = branches.get(selected) {
                    self.start_merge(Some(branch.clone()));
                }
            }
            _ => {}
        }
    }

    fn start_merge(&mut self, source_branch: Option<String>) {
        if self.active_op.is_some() {
            self.message = Some(AppMessage::error("Another operation is in progress"));
            self.state = AppState::List;
            return;
        }

        let wt = match self.selected_worktree().cloned() {
            Some(wt) => wt,
            Option::None => {
                self.state = AppState::List;
                return;
            }
        };

        if wt.is_bare {
            self.message = Some(AppMessage::error("Cannot merge into bare repository"));
            self.state = AppState::List;
            return;
        }
        if wt.status != WorktreeStatus::Clean {
            self.message = Some(AppMessage::error(
                "Cannot merge: worktree has uncommitted changes",
            ));
            self.state = AppState::List;
            return;
        }

        let display_name = wt.display_name();
        let display_name_for_state = display_name.clone();
        let worktree_path = wt.path.clone();
        let worktree_path_for_thread = worktree_path.clone();
        let worktree_path_for_state = worktree_path.clone();
        let source_branch_for_thread = source_branch.clone();

        let cmd_detail = match source_branch.as_deref() {
            Some(branch) => format!("git -C {} merge {}", worktree_path.display(), branch),
            Option::None => format!("git -C {} merge @{{upstream}}", worktree_path.display()),
        };
        let cmd_detail_for_thread = cmd_detail.clone();

        self.merge_source_branch = source_branch.clone();
        self.state = AppState::List;
        self.message = Some(AppMessage::info(match source_branch.as_deref() {
            Some(branch) => format!("Merging {}...", branch),
            Option::None => format!("Merging upstream into {}...", display_name),
        }));

        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = if let Some(source) = source_branch_for_thread {
                git::merge_branch(&worktree_path_for_thread, &source)
            } else {
                git::merge_upstream(&worktree_path_for_thread)
            };

            let message = match &result {
                Ok(msg) => {
                    if msg.is_empty() {
                        "Merge completed".to_string()
                    } else {
                        format!("Merge completed: {}", msg)
                    }
                }
                Err(e) => format!("Merge failed: {}", e),
            };

            let _ = tx.send(OpResult {
                kind: OpKind::Merge,
                success: result.is_ok(),
                message,
                cmd_detail: cmd_detail_for_thread,
                worktree_path: worktree_path_for_thread.clone(),
                affected_paths: vec![worktree_path_for_thread.clone()],
            });
        });

        self.active_op = Some((OpKind::Merge, rx));
        self.active_op_info = Some(ActiveOp {
            kind: OpKind::Merge,
            worktree_path: worktree_path_for_state.clone(),
            worktree_paths: vec![worktree_path_for_state.clone()],
            display_name: display_name_for_state,
        });
    }
}

fn batch_display_name(worktrees: &[Worktree], singular: &str) -> String {
    match worktrees {
        [] => format!("0 {}s", singular),
        [wt] => wt.display_name(),
        _ => format!("{} {}s", worktrees.len(), singular),
    }
}

fn shell_quote(path: &std::path::Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}

fn worktree_name_from_path(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "worktree".to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{acquire_test_env_lock, EnvVarGuard};
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let id = std::process::id();
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("owt_app_unit_{}_{}_{}", name, id, ts));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[cfg(unix)]
    fn make_executable(path: &Path) {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[cfg(not(unix))]
    fn make_executable(_path: &Path) {}

    fn git_cmd() -> Command {
        let mut cmd = Command::new("git");
        cmd.env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_COMMON_DIR");
        cmd
    }

    fn assert_git_success(output: std::process::Output, context: &str) {
        assert!(
            output.status.success(),
            "{}: {}",
            context,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn create_test_project(base: &Path) -> (PathBuf, PathBuf) {
        let source_path = base.join("source");
        let bare_path = base.join("repo").join(".bare");
        let main_path = base.join("repo").join("main");

        fs::create_dir_all(&source_path).unwrap();
        assert_git_success(
            git_cmd()
                .current_dir(&source_path)
                .args(["init", "-b", "main"])
                .output()
                .unwrap(),
            "git init failed",
        );
        assert_git_success(
            git_cmd()
                .current_dir(&source_path)
                .args(["config", "user.email", "test@test.com"])
                .output()
                .unwrap(),
            "git config user.email failed",
        );
        assert_git_success(
            git_cmd()
                .current_dir(&source_path)
                .args(["config", "user.name", "Test"])
                .output()
                .unwrap(),
            "git config user.name failed",
        );

        fs::write(source_path.join("README.md"), "# Test\n").unwrap();
        assert_git_success(
            git_cmd()
                .current_dir(&source_path)
                .args(["add", "."])
                .output()
                .unwrap(),
            "git add failed",
        );
        assert_git_success(
            git_cmd()
                .current_dir(&source_path)
                .args(["commit", "-m", "initial"])
                .output()
                .unwrap(),
            "git commit failed",
        );
        assert_git_success(
            git_cmd()
                .args([
                    "clone",
                    "--bare",
                    &source_path.to_string_lossy(),
                    &bare_path.to_string_lossy(),
                ])
                .output()
                .unwrap(),
            "git clone --bare failed",
        );
        assert_git_success(
            git_cmd()
                .args([
                    "-C",
                    &bare_path.to_string_lossy(),
                    "worktree",
                    "add",
                    &main_path.to_string_lossy(),
                    "main",
                ])
                .output()
                .unwrap(),
            "git worktree add main failed",
        );

        (bare_path, main_path)
    }

    fn test_app(worktrees: Vec<Worktree>, selected_index: usize, bare_repo_path: &str) -> App {
        App {
            worktrees,
            selected_index,
            selected_worktree_paths: HashSet::new(),
            state: AppState::List,
            message: None,
            bare_repo_path: PathBuf::from(bare_repo_path),
            project_root_path: PathBuf::from("/repo"),
            repo_is_bare: true,
            input_buffer: String::new(),
            should_quit: false,
            config: Config::default(),
            exit_action: ExitAction::Quit,
            current_worktree_path: None,
            merge_source_branch: None,
            has_shell_integration: false,
            filter_text: String::new(),
            is_filtering: false,
            last_key: None,
            pending_g_since: None,
            sort_mode: SortMode::default(),
            verbose: false,
            last_command_detail: None,
            spinner_tick: 0,
            theme: crate::ui::theme::detect_theme(),
            viewport_height: Cell::new(0),
            help_scroll_offset: 0,
            script_status: ScriptStatus::Idle,
            script_receiver: None,
            pr_status_receiver: None,
            pr_query_receiver: None,
            pr_query_input: String::new(),
            pr_query_editing: false,
            pr_query_results: Vec::new(),
            active_op: None,
            active_op_info: None,
            selected_details: None,
            add_base_branch: "main".to_string(),
            add_modal_field: AddModalField::default(),
            add_worktree_path: String::new(),
            add_tmux_override: None,
            cleanup_preview: None,
        }
    }

    fn test_worktree(name: &str, status: WorktreeStatus) -> Worktree {
        Worktree {
            path: PathBuf::from(format!("/repo/{}", name)),
            branch: Some(name.to_string()),
            is_bare: false,
            status,
            last_commit_time: None,
            last_commit_timestamp: None,
            ahead_behind: None,
            github_pr_status: None,
        }
    }

    #[test]
    fn recent_sort_uses_numeric_commit_timestamp() {
        let mut older = test_worktree("older", WorktreeStatus::Clean);
        older.last_commit_time = Some("1 hour ago".to_string());
        older.last_commit_timestamp = Some(100);
        let mut newer = test_worktree("newer", WorktreeStatus::Clean);
        newer.last_commit_time = Some("2 days ago".to_string());
        newer.last_commit_timestamp = Some(200);
        let mut app = test_app(vec![older, newer], 0, "/repo/.bare");
        app.sort_mode = SortMode::Recent;

        app.apply_sort();

        assert_eq!(app.worktrees[0].branch.as_deref(), Some("newer"));
        assert_eq!(app.worktrees[1].branch.as_deref(), Some("older"));
    }

    #[test]
    fn single_g_jumps_to_current_worktree_after_timeout() {
        let mut app = test_app(
            vec![
                test_worktree("current", WorktreeStatus::Clean),
                test_worktree("other", WorktreeStatus::Clean),
            ],
            1,
            "/repo/.bare",
        );
        app.current_worktree_path = Some(PathBuf::from("/repo/current"));

        app.handle_list_input(KeyCode::Char('g'), KeyModifiers::NONE);
        app.pending_g_since = Some(Instant::now() - G_SEQUENCE_TIMEOUT);
        app.resolve_pending_g_if_expired();

        assert_eq!(app.selected_index, 0);
        assert!(app.last_key.is_none());
    }

    #[test]
    fn timely_gg_moves_to_top() {
        let mut app = test_app(
            vec![
                test_worktree("first", WorktreeStatus::Clean),
                test_worktree("second", WorktreeStatus::Clean),
            ],
            1,
            "/repo/.bare",
        );

        app.handle_list_input(KeyCode::Char('g'), KeyModifiers::NONE);
        app.handle_list_input(KeyCode::Char('g'), KeyModifiers::NONE);

        assert_eq!(app.selected_index, 0);
        assert!(app.last_key.is_none());
    }

    #[test]
    fn cleanup_preview_cancels_without_starting_a_mutation() {
        let mut app = test_app(vec![], 0, "/repo/.bare");
        app.state = AppState::CleanupPreview;
        app.cleanup_preview = Some(crate::worktree_prune::PruneReport {
            metadata_output: String::new(),
            logs: vec![],
        });

        app.handle_cleanup_preview_input(KeyCode::Esc);

        assert!(matches!(app.state, AppState::List));
        assert!(app.cleanup_preview.is_none());
        assert!(app.active_op.is_none());
    }

    #[test]
    fn config_modal_tmux_auto_run_is_read_only_project_modal() {
        let mut app = test_app(vec![], 0, "/repo/.bare");

        app.handle_config_modal_input(KeyCode::Enter, 5, false);

        assert!(!app.config.run_post_add_script_in_tmux);
        assert_eq!(
            app.message.as_ref().map(|message| message.text.as_str()),
            Some("Post-add auto-run can only be changed in global config")
        );

        app.config.run_post_add_script_in_tmux = true;
        app.handle_config_modal_input(KeyCode::Enter, 5, false);

        assert!(app.config.run_post_add_script_in_tmux);
    }

    #[test]
    fn config_modal_toggles_tmux_worktree_mode() {
        let mut app = test_app(vec![], 0, "/repo/.bare");

        app.handle_config_modal_input(KeyCode::Enter, 4, false);

        assert!(app.config.tmux_worktree_mode);
        assert_eq!(
            app.message.as_ref().map(|message| message.text.as_str()),
            Some("Tmux worktree mode enabled (press 's' to save to file)")
        );

        app.handle_config_modal_input(KeyCode::Enter, 4, false);

        assert!(!app.config.tmux_worktree_mode);
    }

    #[test]
    fn enter_worktree_focuses_matching_tmux_pane_in_tmux_mode() {
        let _env_lock = acquire_test_env_lock();
        let base = temp_dir("enter_tmux_pane_focus");
        let fake_bin = base.join("bin");
        fs::create_dir_all(&fake_bin).unwrap();

        let tmux_log = base.join("tmux-args.txt");
        let fake_tmux = fake_bin.join("tmux");
        fs::write(
            &fake_tmux,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\nif [ \"$1\" = \"list-panes\" ]; then printf '%%1\\tfeature\\t0:1.0\\n'; fi\n",
                tmux_log.to_string_lossy().replace('\'', "'\\''")
            ),
        )
        .unwrap();
        make_executable(&fake_tmux);

        let path = if let Some(existing) = std::env::var_os("PATH") {
            let mut paths = vec![fake_bin.clone()];
            paths.extend(std::env::split_paths(&existing));
            std::env::join_paths(paths).unwrap()
        } else {
            fake_bin.clone().into_os_string()
        };
        let _path_guard = EnvVarGuard::set("PATH", path);

        let mut app = test_app(
            vec![test_worktree("feature", WorktreeStatus::Clean)],
            0,
            "/repo/.bare",
        );
        app.config.tmux_worktree_mode = true;

        app.enter_worktree();

        assert!(app.should_quit);
        assert!(matches!(app.exit_action, ExitAction::Quit));
        let tmux_args = fs::read_to_string(tmux_log).unwrap();
        assert!(tmux_args.contains("list-panes -a -F"));
        assert!(tmux_args.contains("switch-client -t 0:1.0"));
        assert!(tmux_args.contains("select-pane -t %1"));

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn post_add_script_configured_path_editor_targets_configured_script() {
        let base = temp_dir("post_add_script_configured_path_editor");
        let project_root = base.join("project");
        fs::create_dir_all(project_root.join(".owt")).unwrap();

        let editor_log = base.join("editor-arg.txt");
        let editor_path = base.join("fake-editor.sh");
        fs::write(
            &editor_path,
            format!(
                "#!/bin/sh\nprintf '%s' \"$1\" > '{}'\n",
                editor_log.to_string_lossy().replace('\'', "'\\''")
            ),
        )
        .unwrap();
        make_executable(&editor_path);

        let mut app = test_app(vec![], 0, "/repo/.bare");
        app.project_root_path = project_root.clone();
        app.config.editor = Some(editor_path.to_string_lossy().to_string());
        app.config.post_add_script = Some("setup.sh".to_string());

        app.open_post_add_script_editor();

        let configured_script = project_root.join("setup.sh");
        assert!(configured_script.exists());
        assert!(!Config::post_add_script_path(&project_root).exists());
        assert_eq!(
            fs::read_to_string(editor_log).unwrap(),
            configured_script.to_string_lossy()
        );

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn post_add_script_configured_path_execution_ignores_default_script() {
        let base = temp_dir("post_add_script_configured_path_default_ignored");
        let project_root = base.join("project");
        let worktree_path = base.join("worktree");
        fs::create_dir_all(project_root.join(".owt")).unwrap();
        fs::create_dir_all(&worktree_path).unwrap();
        fs::write(Config::post_add_script_path(&project_root), "#!/bin/sh\n").unwrap();

        let mut app = test_app(vec![], 0, "/repo/.bare");
        app.project_root_path = project_root;
        app.config.post_add_script = Some("setup.sh".to_string());
        app.config.run_post_add_script_in_tmux = true;

        app.run_post_add_script(&worktree_path);

        assert!(app.script_receiver.is_none());
        assert!(matches!(app.script_status, ScriptStatus::Idle));
        assert!(app.message.is_none());

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn post_add_script_configured_path_execution_launches_configured_script_in_tmux() {
        let _env_lock = acquire_test_env_lock();
        let base = temp_dir("post_add_script_configured_path_tmux");
        let project_root = base.join("project");
        let worktree_path = base.join("worktree");
        let fake_bin = base.join("bin");
        fs::create_dir_all(&project_root).unwrap();
        fs::create_dir_all(&worktree_path).unwrap();
        fs::create_dir_all(&fake_bin).unwrap();

        let configured_script = project_root.join("setup.sh");
        fs::write(&configured_script, "#!/bin/sh\n").unwrap();
        let tmux_log = base.join("tmux-args.txt");
        let fake_tmux = fake_bin.join("tmux");
        fs::write(
            &fake_tmux,
            format!(
                "#!/bin/sh\nprintf '%s' \"$*\" > '{}'\nexit 0\n",
                tmux_log.to_string_lossy().replace('\'', "'\\''")
            ),
        )
        .unwrap();
        make_executable(&fake_tmux);

        let path = if let Some(existing) = std::env::var_os("PATH") {
            let mut paths = vec![fake_bin.clone()];
            paths.extend(std::env::split_paths(&existing));
            std::env::join_paths(paths).unwrap()
        } else {
            fake_bin.clone().into_os_string()
        };
        let _path_guard = EnvVarGuard::set("PATH", path);

        let mut app = test_app(vec![], 0, "/repo/.bare");
        app.project_root_path = project_root.clone();
        app.config.post_add_script = Some("setup.sh".to_string());
        app.config.run_post_add_script_in_tmux = true;

        app.run_post_add_script(&worktree_path);

        let tmux_args = fs::read_to_string(tmux_log).unwrap();
        assert!(tmux_args.contains("new-session -d -s owt-post-add-"));
        assert!(tmux_args.contains(&format!("cd {}", shell_quote(&worktree_path))));
        assert!(tmux_args.contains(&format!("sh {}", shell_quote(&configured_script))));
        assert!(app.script_receiver.is_some());

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn conflicting_worktree_for_branch_reports_existing_checkout_path() {
        let app = test_app(
            vec![Worktree {
                path: PathBuf::from("/tmp/external/staging"),
                branch: Some("staging".to_string()),
                is_bare: false,
                status: WorktreeStatus::Clean,
                last_commit_time: None,
                last_commit_timestamp: None,
                ahead_behind: None,
                github_pr_status: None,
            }],
            0,
            "/repo/.bare",
        );

        let desired_path = app.worktree_path_for_branch("staging");
        let conflict = app
            .conflicting_worktree_for_branch("staging", &desired_path)
            .expect("staging checkout should be reported");

        assert_eq!(desired_path, PathBuf::from("/repo/staging"));
        assert_eq!(conflict.path, PathBuf::from("/tmp/external/staging"));
    }

    #[test]
    fn add_modal_base_label_uses_session_default_branch() {
        let app = test_app(
            vec![Worktree {
                path: PathBuf::from("/repo/staging"),
                branch: Some("staging".to_string()),
                is_bare: false,
                status: WorktreeStatus::Clean,
                last_commit_time: None,
                last_commit_timestamp: None,
                ahead_behind: None,
                github_pr_status: None,
            }],
            0,
            "/repo/.bare",
        );

        assert_eq!(app.add_modal_base_label(), "Base branch: main");
    }

    #[test]
    fn add_modal_enter_queues_post_tui_create_request() {
        let mut app = test_app(
            vec![test_worktree("main", WorktreeStatus::Clean)],
            0,
            "/repo/.bare",
        );
        app.current_worktree_path = Some(PathBuf::from("/repo/main"));
        app.input_buffer = "feature/post-tui".to_string();

        app.handle_add_modal_input(KeyCode::Enter, KeyModifiers::NONE);

        assert!(app.should_quit);
        assert!(app.active_op.is_none());
        assert!(matches!(app.state, AppState::List));
        assert!(app.input_buffer.is_empty());
        match app.exit_action {
            ExitAction::CreateWorktree(request) => {
                assert_eq!(request.bare_repo_path, PathBuf::from("/repo/.bare"));
                assert_eq!(request.project_root_path, PathBuf::from("/repo"));
                assert_eq!(request.branch, "feature/post-tui");
                assert_eq!(request.base_branch, "main");
                assert_eq!(
                    request.worktree_path,
                    PathBuf::from("/repo/feature/post-tui")
                );
                assert_eq!(request.source_path, Some(PathBuf::from("/repo/main")));
                assert_eq!(request.tmux, None);
            }
            other => panic!("expected post-TUI create request, got {other:?}"),
        }
    }

    #[test]
    fn add_modal_queues_explicit_path_and_tmux_override() {
        let mut app = test_app(
            vec![test_worktree("main", WorktreeStatus::Clean)],
            0,
            "/repo/.bare",
        );
        app.input_buffer = "feature/advanced".to_string();

        app.handle_add_modal_input(KeyCode::Char('p'), KeyModifiers::CONTROL);
        app.handle_add_modal_input(KeyCode::Char('/'), KeyModifiers::NONE);
        for character in "tmp/owt-advanced".chars() {
            app.handle_add_modal_input(KeyCode::Char(character), KeyModifiers::NONE);
        }
        app.handle_add_modal_input(KeyCode::Char('t'), KeyModifiers::CONTROL);
        app.handle_add_modal_input(KeyCode::Enter, KeyModifiers::NONE);

        match app.exit_action {
            ExitAction::CreateWorktree(request) => {
                assert_eq!(request.worktree_path, PathBuf::from("/tmp/owt-advanced"));
                assert_eq!(request.tmux, Some(true));
            }
            _ => panic!("add modal should queue a create request"),
        }
    }

    #[test]
    fn non_bare_worktree_path_uses_configurable_root_and_repo_namespace() {
        let mut app = test_app(vec![], 0, "/repo");
        app.repo_is_bare = false;
        app.config.worktree_root = Some("/tmp/owt-worktrees".to_string());

        assert_eq!(
            app.worktree_path_for_branch("feature/login"),
            PathBuf::from("/tmp/owt-worktrees/repo/feature/login")
        );
    }

    #[test]
    fn bare_repo_ignores_worktree_root_and_keeps_sibling_layout() {
        let mut app = test_app(vec![], 0, "/repo/.bare");
        app.config.worktree_root = Some("/tmp/owt-worktrees".to_string());

        assert_eq!(
            app.worktree_path_for_branch("feature/login"),
            PathBuf::from("/repo/feature/login")
        );
    }

    #[test]
    fn bare_repo_without_worktree_root_keeps_sibling_layout() {
        let app = test_app(vec![], 0, "/repo/.bare");

        assert_eq!(
            app.worktree_path_for_branch("feature/login"),
            PathBuf::from("/repo/feature/login")
        );
    }

    #[test]
    fn paths_match_after_canonicalization() {
        let tmp = std::env::temp_dir();
        let canonical = tmp.canonicalize().unwrap_or_else(|_| tmp.clone());

        assert!(paths_refer_to_same_location(&tmp, &canonical));
    }

    #[test]
    fn add_completion_selects_created_worktree_before_enter() {
        let base = temp_dir("add_completion_enter");
        let (bare_path, main_path) = create_test_project(&base);
        let project_root = bare_path.parent().unwrap().to_path_buf();
        let feature_path = project_root.join("feature").join("enter-after-create");

        git::add_worktree(
            &bare_path,
            "feature/enter-after-create",
            &feature_path,
            Some("main"),
        )
        .unwrap();

        let mut app =
            App::new(bare_path.clone(), project_root, true, Some(main_path), true).unwrap();

        app.handle_op_result(OpResult {
            kind: OpKind::Add,
            success: true,
            message: "Created worktree: feature/enter-after-create".to_string(),
            cmd_detail: String::new(),
            worktree_path: feature_path.clone(),
            affected_paths: vec![feature_path.clone()],
        });
        app.enter_worktree();

        match app.exit_action {
            ExitAction::ChangeDirectory(path) => {
                assert!(paths_refer_to_same_location(&path, &feature_path));
            }
            ExitAction::Quit => panic!("enter should request directory change"),
            ExitAction::CreateWorktree(_) => panic!("enter should not create a worktree"),
        }
        assert!(app.should_quit);

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn enter_is_blocked_while_background_operation_is_running() {
        let (_tx, rx) = mpsc::channel();
        let mut app = test_app(
            vec![Worktree {
                path: PathBuf::from("/repo/main"),
                branch: Some("main".to_string()),
                is_bare: false,
                status: WorktreeStatus::Clean,
                last_commit_time: None,
                last_commit_timestamp: None,
                ahead_behind: None,
                github_pr_status: None,
            }],
            0,
            "/repo/.bare",
        );
        app.active_op = Some((OpKind::Add, rx));

        app.handle_list_input(KeyCode::Enter, KeyModifiers::empty());

        assert!(matches!(app.exit_action, ExitAction::Quit));
        assert!(!app.should_quit);
        assert_eq!(
            app.message.as_ref().map(|message| message.text.as_str()),
            Some("Operation still in progress")
        );
    }

    #[test]
    fn enter_rejects_bare_repository_selection() {
        let mut app = test_app(
            vec![Worktree {
                path: PathBuf::from("/repo/.bare"),
                branch: None,
                is_bare: true,
                status: WorktreeStatus::Clean,
                last_commit_time: None,
                last_commit_timestamp: None,
                ahead_behind: None,
                github_pr_status: None,
            }],
            0,
            "/repo/.bare",
        );

        app.enter_worktree();

        assert!(matches!(app.exit_action, ExitAction::Quit));
        assert!(!app.should_quit);
        assert_eq!(
            app.message.as_ref().map(|message| message.text.as_str()),
            Some("Cannot enter bare repository")
        );
    }

    #[test]
    fn delete_confirmation_blocks_dirty_worktree_without_force() {
        let mut app = test_app(
            vec![Worktree {
                path: PathBuf::from("/repo/dirty"),
                branch: Some("dirty".to_string()),
                is_bare: false,
                status: WorktreeStatus::Unstaged,
                last_commit_time: None,
                last_commit_timestamp: None,
                ahead_behind: None,
                github_pr_status: None,
            }],
            0,
            "/repo/.bare",
        );
        app.state = AppState::ConfirmDelete {
            delete_branch: false,
            force: false,
        };

        app.handle_confirm_delete_input(KeyCode::Enter, false, false);

        assert!(app.active_op.is_none());
        assert!(matches!(
            app.state,
            AppState::ConfirmDelete {
                delete_branch: false,
                force: false
            }
        ));
        assert_eq!(
            app.message.as_ref().map(|message| message.text.as_str()),
            Some("Worktree has uncommitted changes. Press 'f' to enable force delete.")
        );
    }

    #[test]
    fn space_toggles_worktree_selection_without_moving_cursor() {
        let mut app = test_app(
            vec![
                test_worktree("main", WorktreeStatus::Clean),
                test_worktree("feature", WorktreeStatus::Clean),
            ],
            1,
            "/repo/.bare",
        );

        app.handle_list_input(KeyCode::Char(' '), KeyModifiers::empty());

        assert_eq!(app.selected_index, 1);
        assert!(app.is_worktree_marked(Path::new("/repo/feature")));
        assert_eq!(app.selected_worktree_count(), 1);

        app.handle_list_input(KeyCode::Char(' '), KeyModifiers::empty());

        assert_eq!(app.selected_worktree_count(), 0);
        assert!(!app.is_worktree_marked(Path::new("/repo/feature")));
    }

    #[test]
    fn action_worktrees_uses_checked_worktrees_before_cursor() {
        let mut app = test_app(
            vec![
                test_worktree("main", WorktreeStatus::Clean),
                test_worktree("feature-a", WorktreeStatus::Clean),
                test_worktree("feature-b", WorktreeStatus::Clean),
            ],
            0,
            "/repo/.bare",
        );
        app.selected_worktree_paths
            .insert(PathBuf::from("/repo/feature-a"));
        app.selected_worktree_paths
            .insert(PathBuf::from("/repo/feature-b"));

        let action_paths: Vec<PathBuf> = app
            .action_worktrees()
            .into_iter()
            .map(|wt| wt.path)
            .collect();

        assert_eq!(
            action_paths,
            vec![
                PathBuf::from("/repo/feature-a"),
                PathBuf::from("/repo/feature-b")
            ]
        );
    }

    #[test]
    fn batch_pull_blocks_when_any_checked_worktree_is_dirty() {
        let mut app = test_app(
            vec![
                test_worktree("main", WorktreeStatus::Clean),
                test_worktree("dirty", WorktreeStatus::Unstaged),
            ],
            0,
            "/repo/.bare",
        );
        app.selected_worktree_paths
            .insert(PathBuf::from("/repo/main"));
        app.selected_worktree_paths
            .insert(PathBuf::from("/repo/dirty"));

        app.pull_worktree();

        assert!(app.active_op.is_none());
        assert_eq!(
            app.message.as_ref().map(|message| message.text.as_str()),
            Some("Cannot pull: worktree has uncommitted changes: dirty")
        );
    }

    #[test]
    fn batch_delete_dirty_guard_checks_checked_worktrees_not_cursor() {
        let mut app = test_app(
            vec![
                test_worktree("dirty-cursor", WorktreeStatus::Unstaged),
                test_worktree("clean-target", WorktreeStatus::Clean),
            ],
            0,
            "/repo/.bare",
        );
        app.selected_worktree_paths
            .insert(PathBuf::from("/repo/clean-target"));
        app.state = AppState::ConfirmDelete {
            delete_branch: false,
            force: false,
        };

        app.handle_confirm_delete_input(KeyCode::Enter, false, false);

        assert!(app.active_op.is_some());
        assert!(matches!(app.state, AppState::List));
    }

    #[test]
    fn filter_enter_changes_to_first_matching_worktree() {
        let mut app = test_app(
            vec![
                Worktree {
                    path: PathBuf::from("/repo/main"),
                    branch: Some("main".to_string()),
                    is_bare: false,
                    status: WorktreeStatus::Clean,
                    last_commit_time: None,
                    last_commit_timestamp: None,
                    ahead_behind: None,
                    github_pr_status: None,
                },
                Worktree {
                    path: PathBuf::from("/repo/feature-search"),
                    branch: Some("feature/search".to_string()),
                    is_bare: false,
                    status: WorktreeStatus::Clean,
                    last_commit_time: None,
                    last_commit_timestamp: None,
                    ahead_behind: None,
                    github_pr_status: None,
                },
            ],
            0,
            "/repo/.bare",
        );
        app.is_filtering = true;

        app.handle_filter_input(KeyCode::Char('s'));
        app.handle_filter_input(KeyCode::Enter);

        match app.exit_action {
            ExitAction::ChangeDirectory(path) => {
                assert_eq!(path, PathBuf::from("/repo/feature-search"));
            }
            ExitAction::Quit => panic!("filter enter should request directory change"),
            ExitAction::CreateWorktree(_) => panic!("filter enter should not create a worktree"),
        }
        assert!(!app.is_filtering);
        assert!(app.should_quit);
    }

    #[test]
    fn filter_selects_status_and_pr_matches_with_cli_semantics() {
        let mut app = test_app(
            vec![
                Worktree {
                    path: PathBuf::from("/repo/main"),
                    branch: Some("main".to_string()),
                    is_bare: false,
                    status: WorktreeStatus::Clean,
                    last_commit_time: None,
                    last_commit_timestamp: None,
                    ahead_behind: None,
                    github_pr_status: None,
                },
                Worktree {
                    path: PathBuf::from("/repo/review-42"),
                    branch: Some("review/42".to_string()),
                    is_bare: false,
                    status: WorktreeStatus::Unstaged,
                    last_commit_time: None,
                    last_commit_timestamp: None,
                    ahead_behind: None,
                    github_pr_status: Some(crate::types::GithubPrStatus::Closed),
                },
            ],
            0,
            "/repo/.bare",
        );

        app.filter_text = "closed".to_string();
        app.select_first_filtered_worktree();
        assert_eq!(app.selected_index, 1);

        app.selected_index = 0;
        app.filter_text = "unstaged".to_string();
        app.select_first_filtered_worktree();
        assert_eq!(app.selected_index, 1);
    }

    #[test]
    fn pr_status_modal_supports_arbitrary_branch_input_with_a_non_bare_anchor() {
        let mut bare = test_worktree(".bare", WorktreeStatus::Clean);
        bare.is_bare = true;
        let mut app = test_app(
            vec![bare, test_worktree("main", WorktreeStatus::Clean)],
            0,
            "/repo/.bare",
        );
        app.state = AppState::PrStatusModal;

        assert_eq!(
            app.pr_query_anchor_path(),
            Some(PathBuf::from("/repo/main"))
        );
        app.handle_pr_status_modal_input(KeyCode::Char('b'));
        for character in "review/123".chars() {
            app.handle_pr_status_modal_input(KeyCode::Char(character));
        }

        assert!(app.pr_query_editing);
        assert_eq!(app.pr_query_input, "review/123");
    }

    #[test]
    fn filter_enter_is_blocked_while_background_operation_is_running() {
        let (_tx, rx) = mpsc::channel();
        let mut app = test_app(
            vec![Worktree {
                path: PathBuf::from("/repo/main"),
                branch: Some("main".to_string()),
                is_bare: false,
                status: WorktreeStatus::Clean,
                last_commit_time: None,
                last_commit_timestamp: None,
                ahead_behind: None,
                github_pr_status: None,
            }],
            0,
            "/repo/.bare",
        );
        app.is_filtering = true;
        app.active_op = Some((OpKind::Fetch, rx));

        app.handle_filter_input(KeyCode::Enter);

        assert!(matches!(app.exit_action, ExitAction::Quit));
        assert!(!app.should_quit);
        assert!(app.is_filtering);
        assert_eq!(
            app.message.as_ref().map(|message| message.text.as_str()),
            Some("Operation still in progress")
        );
    }

    #[test]
    fn poll_pr_status_updates_matching_worktree_without_active_op() {
        let (tx, rx) = mpsc::channel();
        let mut app = test_app(
            vec![Worktree {
                path: PathBuf::from("/repo/feature"),
                branch: Some("feature".to_string()),
                is_bare: false,
                status: WorktreeStatus::Clean,
                last_commit_time: None,
                last_commit_timestamp: None,
                ahead_behind: None,
                github_pr_status: None,
            }],
            0,
            "/repo/.bare",
        );
        let (_op_tx, op_rx) = mpsc::channel();
        app.active_op = Some((OpKind::Fetch, op_rx));
        app.pr_status_receiver = Some(rx);

        tx.send(vec![(
            PathBuf::from("/repo/feature"),
            Some(GithubPrStatus::Open),
        )])
        .unwrap();
        app.poll_pr_status();

        assert_eq!(
            app.worktrees[0].github_pr_status,
            Some(GithubPrStatus::Open)
        );
        assert!(app.pr_status_receiver.is_none());
        assert!(app.active_op.is_some());
    }
}
