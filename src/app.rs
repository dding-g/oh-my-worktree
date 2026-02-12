use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{backend::Backend, Frame, Terminal};
use std::cell::Cell;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

use crate::config::Config;
use crate::git;
use crate::types::{AppMessage, AppState, ExitAction, ScriptStatus, SortMode, Worktree, WorktreeStatus};
use crate::ui::{add_modal, config_modal, confirm_modal, help_modal, main_view};
use crate::ui::theme::Theme;

pub struct ScriptResult {
    pub success: bool,
    pub message: String,
}

pub struct DeleteResult {
    pub success: bool,
    pub message: String,
    pub worktree_path: PathBuf,
    pub cmd_detail: String,
}

pub struct App {
    pub worktrees: Vec<Worktree>,
    pub selected_index: usize,
    pub state: AppState,
    pub message: Option<AppMessage>,
    pub bare_repo_path: PathBuf,
    pub input_buffer: String,
    pub should_quit: bool,
    pub config: Config,
    pub exit_action: ExitAction,
    pub is_fetching: bool,
    pub current_worktree_path: Option<PathBuf>, // Path where owt was launched from
    pub is_adding: bool,
    pub is_deleting: bool,
    pub is_pulling: bool,
    pub is_pushing: bool,
    pub is_merging: bool,
    pub merge_source_branch: Option<String>,  // Branch to merge from
    pub has_shell_integration: bool, // Whether OWT_OUTPUT_FILE is set
    pub filter_text: String,         // Search/filter text
    pub is_filtering: bool,          // Whether in filter mode
    pub last_key: Option<char>,      // For gg detection
    pub sort_mode: SortMode,         // Current sort mode
    pub verbose: bool,               // Show detailed git command output
    pub last_command_detail: Option<String>, // Last git command detail for verbose mode
    pub spinner_tick: usize,         // Spinner animation tick
    pub theme: Theme,                // Active color theme
    pub viewport_height: Cell<u16>,  // Table viewport height (set during render)
    pub help_scroll_offset: u16,     // Scroll offset for help modal
    pub script_status: ScriptStatus,                       // Background script status
    pub script_receiver: Option<mpsc::Receiver<ScriptResult>>, // Channel for script completion
    pub delete_receiver: Option<mpsc::Receiver<DeleteResult>>, // Channel for async worktree deletion
}

impl App {
    pub fn new(bare_repo_path: PathBuf, launch_path: Option<PathBuf>, has_shell_integration: bool) -> Result<Self> {
        let worktrees = git::list_worktrees(&bare_repo_path)?;
        // Load config with project-level override support
        let config = Config::load_with_project(Some(&bare_repo_path)).unwrap_or_default();

        // Determine current worktree from launch path
        let current_worktree_path = launch_path.and_then(|lp| {
            let canonical_lp = lp.canonicalize().ok()?;
            worktrees.iter().find(|wt| {
                if wt.is_bare { return false; }
                wt.path.canonicalize().ok()
                    .map(|p| canonical_lp.starts_with(&p))
                    .unwrap_or(false)
            }).map(|wt| wt.path.clone())
        });

        // Set initial selection to current worktree if found, otherwise first non-bare worktree
        let selected_index = current_worktree_path.as_ref()
            .and_then(|cp| worktrees.iter().position(|wt| wt.path == *cp))
            .unwrap_or_else(|| {
                // Find first non-bare worktree
                worktrees.iter().position(|wt| !wt.is_bare).unwrap_or(0)
            });

        // Show initial message about shell integration if not set up
        let initial_message = if !has_shell_integration {
            Some(AppMessage::info("Tip: Run 'owt setup' then reload shell for Enter key to change directory"))
        } else {
            None
        };

        Ok(Self {
            worktrees,
            selected_index,
            state: AppState::List,
            message: initial_message,
            bare_repo_path,
            input_buffer: String::new(),
            should_quit: false,
            config,
            exit_action: ExitAction::Quit,
            is_fetching: false,
            current_worktree_path,
            is_adding: false,
            is_deleting: false,
            is_pulling: false,
            is_pushing: false,
            is_merging: false,
            merge_source_branch: None,
            has_shell_integration,
            filter_text: String::new(),
            is_filtering: false,
            last_key: None,
            sort_mode: SortMode::default(),
            verbose: false,
            last_command_detail: None,
            spinner_tick: 0,
            theme: crate::ui::theme::detect_theme(),
            viewport_height: Cell::new(0),
            help_scroll_offset: 0,
            script_status: ScriptStatus::Idle,
            script_receiver: None,
            delete_receiver: None,
        })
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| self.draw(frame))?;

            // Handle async-like operations (show UI first, then execute)
            if self.is_fetching {
                self.do_fetch();
                continue;
            }

            if self.is_adding {
                self.do_add_worktree();
                // Drain pending key events to prevent accidental enter_worktree
                while event::poll(Duration::from_millis(0))? {
                    let _ = event::read()?;
                }
                continue;
            }

            if self.is_deleting {
                self.do_delete_worktree();
                continue;
            }

            if self.is_pulling {
                self.do_pull();
                while event::poll(Duration::from_millis(0))? {
                    let _ = event::read()?;
                }
                continue;
            }

            if self.is_pushing {
                self.do_push();
                while event::poll(Duration::from_millis(0))? {
                    let _ = event::read()?;
                }
                continue;
            }

            if self.is_merging {
                self.do_merge();
                while event::poll(Duration::from_millis(0))? {
                    let _ = event::read()?;
                }
                continue;
            }

            // Poll background operations
            self.poll_script_status();
            self.poll_delete_status();

            self.handle_events(terminal)?;
        }
        Ok(())
    }

    fn poll_script_status(&mut self) {
        if let Some(ref rx) = self.script_receiver {
            match rx.try_recv() {
                Ok(result) => {
                    let status_msg = if result.success {
                        format!("Setup script completed: {}", result.message)
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

    fn poll_delete_status(&mut self) {
        if let Some(ref rx) = self.delete_receiver {
            match rx.try_recv() {
                Ok(result) => {
                    if result.success {
                        let mut msg = result.message.clone();
                        if self.verbose {
                            self.last_command_detail = Some(result.cmd_detail.clone());
                            msg = format!("{}\n$ {}", msg, result.cmd_detail);
                        }
                        self.message = Some(AppMessage::info(msg));

                        // Remove deleted worktree from in-memory list (no blocking refresh)
                        self.worktrees.retain(|wt| wt.path != result.worktree_path);
                        if self.selected_index >= self.worktrees.len() {
                            self.selected_index = self.worktrees.len().saturating_sub(1);
                        }
                    } else {
                        let mut msg = format!("Failed to delete: {}", result.message);
                        if self.verbose {
                            self.last_command_detail = Some(result.cmd_detail.clone());
                            msg = format!("{}\n$ {}", msg, result.cmd_detail);
                        }
                        self.message = Some(AppMessage::error(msg));
                    }
                    self.delete_receiver = None;
                    self.state = AppState::List;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // Still running, tick spinner
                    self.spinner_tick = self.spinner_tick.wrapping_add(1);
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.delete_receiver = None;
                    self.state = AppState::List;
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame) {
        match self.state {
            AppState::List | AppState::Fetching | AppState::Adding | AppState::Deleting
            | AppState::Pulling | AppState::Pushing | AppState::Merging => {
                main_view::render(frame, self)
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
            AppState::MergeBranchSelect { .. } => {
                main_view::render(frame, self);
                crate::ui::merge_modal::render(frame, self);
            }
        }
    }

    fn handle_events<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        // Update spinner tick during loading states
        if self.is_adding || self.is_deleting || self.is_fetching || self.is_pulling || self.is_pushing || self.is_merging {
            self.spinner_tick = self.spinner_tick.wrapping_add(1);
        }

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
                        AppState::AddModal => self.handle_add_modal_input(key.code),
                        AppState::ConfirmDelete { delete_branch, force } => {
                            self.handle_confirm_delete_input(key.code, delete_branch, force)
                        }
                        AppState::ConfigModal { selected_index, editing } => {
                            self.handle_config_modal_input(key.code, selected_index, editing)
                        }
                        AppState::HelpModal => self.handle_help_modal_input(key.code),
                        AppState::MergeBranchSelect { branches, selected } => {
                            self.handle_merge_branch_select_input(key.code, branches, selected)
                        }
                        AppState::Fetching | AppState::Adding | AppState::Deleting
                        | AppState::Pulling | AppState::Pushing | AppState::Merging => {
                            // Ignore input during operations
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
                // Check for 'gg' (go to top) or single 'g' (go to current worktree)
                if self.last_key == Some('g') {
                    self.move_to_top();
                    self.last_key = None;
                } else {
                    // First 'g' press - wait for next key
                    self.last_key = Some('g');
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
                if let Some(wt) = self.selected_worktree() {
                    if wt.is_bare {
                        self.message = Some(AppMessage::error("Cannot delete bare repository"));
                    } else {
                        self.state = AppState::ConfirmDelete { delete_branch: false, force: false };
                    }
                }
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
            KeyCode::Char('r') => {
                self.refresh_worktrees();
                self.last_key = None;
            }
            KeyCode::Char('x') => {
                self.prune_worktrees();
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
            KeyCode::Char('0') => {
                // Go to current worktree (if single g was pressed before, treat as timeout)
                if self.last_key == Some('g') {
                    self.jump_to_current_worktree();
                }
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
                // If we were waiting for 'g' and got something else
                if self.last_key == Some('g') {
                    self.jump_to_current_worktree();
                }
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

    fn select_first_filtered_worktree(&mut self) {
        if self.filter_text.is_empty() {
            return;
        }
        let filter_lower = self.filter_text.to_lowercase();
        if let Some(idx) = self.worktrees.iter().position(|wt| {
            wt.display_name().to_lowercase().contains(&filter_lower)
                || wt.branch_display().to_lowercase().contains(&filter_lower)
        }) {
            self.selected_index = idx;
        }
    }

    fn handle_add_modal_input(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => {
                self.state = AppState::List;
                self.input_buffer.clear();
            }
            KeyCode::Enter => {
                if !self.input_buffer.trim().is_empty() {
                    self.add_worktree();
                }
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
            }
            _ => {}
        }
    }

    fn handle_confirm_delete_input(&mut self, code: KeyCode, delete_branch: bool, force: bool) {
        match code {
            KeyCode::Esc | KeyCode::Char('n') => {
                self.state = AppState::List;
            }
            KeyCode::Char('y') | KeyCode::Enter => {
                // Require force for dirty worktrees
                if let Some(wt) = self.selected_worktree() {
                    if wt.status != WorktreeStatus::Clean && !force {
                        self.message = Some(AppMessage::error(
                            "Worktree has uncommitted changes. Press 'f' to enable force delete."
                        ));
                        return;
                    }
                }
                self.delete_selected_worktree(delete_branch, force);
            }
            KeyCode::Char('b') => {
                // Toggle delete branch option
                self.state = AppState::ConfirmDelete { delete_branch: !delete_branch, force };
            }
            KeyCode::Char('f') => {
                // Toggle force delete option
                self.state = AppState::ConfirmDelete { delete_branch, force: !force };
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
                    if selected == 3 {
                        // post_add_script - open with $EDITOR
                        self.open_post_add_script_editor();
                    } else {
                        // Enter inline editing mode
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
            2 => self.config.copy_files.join(", "),
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
                // copy_files
                self.config.copy_files = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            _ => {}
        }
        self.message = Some(AppMessage::info("Setting updated (press 's' to save to file)"));
    }

    fn save_config(&mut self) {
        match self.config.save() {
            Ok(()) => {
                self.message = Some(AppMessage::info("Config saved"));
            }
            Err(e) => {
                self.message = Some(AppMessage::error(format!("Failed to save config: {}", e)));
            }
        }
    }

    fn open_post_add_script_editor(&mut self) {
        let script_path = Config::post_add_script_path(&self.bare_repo_path);
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
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen
        );

        let status = Command::new(&editor).arg(&script_path).status();

        // Restore terminal after editor closes
        let _ = crossterm::terminal::enable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::EnterAlternateScreen
        );

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
        }
    }

    fn move_selection_down(&mut self) {
        if self.selected_index < self.worktrees.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    fn move_to_top(&mut self) {
        self.selected_index = 0;
    }

    fn move_to_bottom(&mut self) {
        self.selected_index = self.worktrees.len().saturating_sub(1);
    }

    fn move_selection_half_page_down(&mut self) {
        let vh = self.viewport_height.get();
        let half_page = if vh > 0 { (vh / 2) as usize } else { 10 };
        let max_index = self.worktrees.len().saturating_sub(1);
        self.selected_index = (self.selected_index + half_page).min(max_index);
    }

    fn move_selection_half_page_up(&mut self) {
        let vh = self.viewport_height.get();
        let half_page = if vh > 0 { (vh / 2) as usize } else { 10 };
        self.selected_index = self.selected_index.saturating_sub(half_page);
    }

    fn jump_to_current_worktree(&mut self) {
        if let Some(ref current_path) = self.current_worktree_path {
            if let Some(idx) = self.worktrees.iter().position(|wt| wt.path == *current_path) {
                self.selected_index = idx;
                self.message = Some(AppMessage::info("Jumped to current worktree"));
            }
        } else {
            self.message = Some(AppMessage::error("No current worktree detected"));
        }
    }

    pub fn selected_worktree(&self) -> Option<&Worktree> {
        self.worktrees.get(self.selected_index)
    }

    fn refresh_worktrees(&mut self) {
        match git::list_worktrees(&self.bare_repo_path) {
            Ok(worktrees) => {
                self.worktrees = worktrees;
                self.apply_sort();
                if self.selected_index >= self.worktrees.len() {
                    self.selected_index = self.worktrees.len().saturating_sub(1);
                }
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
        self.message = Some(AppMessage::info(format!("Sort: {}", self.sort_mode.label())));
    }

    fn apply_sort(&mut self) {
        // Remember currently selected worktree path
        let selected_path = self.selected_worktree().map(|wt| wt.path.clone());

        match self.sort_mode {
            SortMode::Name => {
                self.worktrees.sort_by(|a, b| {
                    // Bare repo always first
                    if a.is_bare && !b.is_bare { return std::cmp::Ordering::Less; }
                    if !a.is_bare && b.is_bare { return std::cmp::Ordering::Greater; }
                    a.display_name().to_lowercase().cmp(&b.display_name().to_lowercase())
                });
            }
            SortMode::Recent => {
                self.worktrees.sort_by(|a, b| {
                    // Bare repo always first
                    if a.is_bare && !b.is_bare { return std::cmp::Ordering::Less; }
                    if !a.is_bare && b.is_bare { return std::cmp::Ordering::Greater; }
                    // Sort by last commit time (most recent first)
                    b.last_commit_time.cmp(&a.last_commit_time)
                });
            }
            SortMode::Status => {
                self.worktrees.sort_by(|a, b| {
                    // Bare repo always first
                    if a.is_bare && !b.is_bare { return std::cmp::Ordering::Less; }
                    if !a.is_bare && b.is_bare { return std::cmp::Ordering::Greater; }
                    // Sort by status priority (dirty first)
                    let status_order = |s: &WorktreeStatus| match s {
                        WorktreeStatus::Conflict => 0,
                        WorktreeStatus::Mixed => 1,
                        WorktreeStatus::Unstaged => 2,
                        WorktreeStatus::Staged => 3,
                        WorktreeStatus::Clean => 4,
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
    }

    fn add_worktree(&mut self) {
        let branch = self.input_buffer.trim().to_string();
        if branch.is_empty() {
            self.message = Some(AppMessage::error("Branch name cannot be empty"));
            return;
        }

        self.is_adding = true;
        self.state = AppState::Adding;
        self.message = Some(AppMessage::info(format!("Creating worktree: {}...", branch)));
    }

    fn do_add_worktree(&mut self) {
        let branch = self.input_buffer.trim().to_string();

        // Generate worktree path: sibling to bare repo with branch name
        let worktree_path = self
            .bare_repo_path
            .parent()
            .map(|p| p.join(&branch))
            .unwrap_or_else(|| PathBuf::from(&branch));

        let default_branch = git::get_default_branch(&self.bare_repo_path)
            .unwrap_or_else(|_| "main".to_string());

        // Build verbose detail
        let cmd_detail = git::build_add_worktree_command_detail(
            &self.bare_repo_path, &branch, &worktree_path, Some(&default_branch),
        );

        let result = git::add_worktree(&self.bare_repo_path, &branch, &worktree_path, Some(&default_branch));

        if self.verbose {
            self.last_command_detail = Some(cmd_detail.clone());
        }

        match result {
            Ok(()) => {
                // Copy files if configured
                self.copy_configured_files(&worktree_path);

                // Run post-add script if exists (in background)
                self.run_post_add_script(&worktree_path);

                let mut msg = if matches!(self.script_status, ScriptStatus::Running { .. }) {
                    format!("Created worktree: {} (running setup script...)", branch)
                } else {
                    format!("Created worktree: {}", branch)
                };
                if self.verbose {
                    msg = format!("{}\n$ {}", msg, cmd_detail);
                }
                self.message = Some(AppMessage::info(msg));
                self.refresh_worktrees();

                // Select the newly added worktree
                if let Some(idx) = self.worktrees.iter().position(|wt| wt.path == worktree_path) {
                    self.selected_index = idx;
                }
            }
            Err(e) => {
                let mut msg = format!("Failed to create: {}", e);
                if self.verbose {
                    msg = format!("{}\n$ {}", msg, cmd_detail);
                }
                self.message = Some(AppMessage::error(msg));
            }
        }

        self.is_adding = false;
        self.state = AppState::List;
        self.input_buffer.clear();
    }

    fn copy_configured_files(&self, target_path: &PathBuf) {
        if self.config.copy_files.is_empty() {
            return;
        }

        // Find a source worktree to copy from (prefer current, then first non-bare)
        let source_path = self.current_worktree_path.clone()
            .or_else(|| {
                self.worktrees.iter()
                    .find(|wt| !wt.is_bare)
                    .map(|wt| wt.path.clone())
            });

        if let Some(source) = source_path {
            for file in &self.config.copy_files {
                let src = source.join(file);
                let dst = target_path.join(file);

                if src.exists() {
                    // Create parent directories if needed
                    if let Some(parent) = dst.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    let _ = fs::copy(&src, &dst);
                }
            }
        }
    }

    fn run_post_add_script(&mut self, worktree_path: &PathBuf) {
        let script_path = Config::post_add_script_path(&self.bare_repo_path);

        if !script_path.exists() {
            return;
        }

        let worktree_name = worktree_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let (tx, rx) = mpsc::channel();
        let script = script_path.clone();
        let wt_path = worktree_path.clone();

        self.script_status = ScriptStatus::Running {
            worktree_name: worktree_name.clone(),
        };
        self.script_receiver = Some(rx);

        std::thread::spawn(move || {
            let output = Command::new("sh")
                .arg(&script)
                .current_dir(&wt_path)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output();

            let result = match output {
                Ok(out) => {
                    if out.status.success() {
                        ScriptResult {
                            success: true,
                            message: worktree_name,
                        }
                    } else {
                        let stderr = String::from_utf8_lossy(&out.stderr);
                        ScriptResult {
                            success: false,
                            message: stderr.trim().to_string(),
                        }
                    }
                }
                Err(e) => ScriptResult {
                    success: false,
                    message: e.to_string(),
                },
            };
            let _ = tx.send(result);
        });
    }

    fn delete_selected_worktree(&mut self, delete_branch: bool, force: bool) {
        if let Some(wt) = self.selected_worktree().cloned() {
            if wt.is_bare {
                self.message = Some(AppMessage::error("Cannot delete bare repository"));
                self.state = AppState::List;
                return;
            }

            // Store delete info and start async delete
            self.is_deleting = true;
            self.state = AppState::Deleting;
            self.message = Some(AppMessage::info(format!("Deleting worktree: {}...", wt.display_name())));
            // Encode flags in input_buffer
            self.input_buffer = format!(
                "{}:{}",
                if delete_branch { "1" } else { "0" },
                if force { "1" } else { "0" }
            );
        }
    }

    fn do_delete_worktree(&mut self) {
        let parts: Vec<&str> = self.input_buffer.split(':').collect();
        let delete_branch = parts.first() == Some(&"1");
        let force = parts.get(1) == Some(&"1");

        if let Some(wt) = self.selected_worktree().cloned() {
            let branch_name = wt.branch.clone();
            let display_name = wt.display_name();

            let force_flag = if force { " --force" } else { "" };
            let cmd_detail = format!(
                "git -C {} worktree remove{}  {}",
                self.bare_repo_path.display(), force_flag, wt.path.display()
            );

            let bare_repo_path = self.bare_repo_path.clone();
            let worktree_path = wt.path.clone();

            let (tx, rx) = mpsc::channel();
            self.delete_receiver = Some(rx);

            std::thread::spawn(move || {
                let result = git::remove_worktree(&bare_repo_path, &worktree_path, force);
                let mut msg = match &result {
                    Ok(()) => format!("Deleted worktree: {}", display_name),
                    Err(e) => e.to_string(),
                };

                // Delete branch in background thread too (avoid blocking main thread)
                if result.is_ok() && delete_branch {
                    if let Some(ref branch) = branch_name {
                        match git::delete_branch(&bare_repo_path, branch, force) {
                            Ok(()) => msg.push_str(&format!(" (branch '{}' deleted)", branch)),
                            Err(e) => msg.push_str(&format!(" (branch delete failed: {})", e)),
                        }
                    }
                }

                let _ = tx.send(DeleteResult {
                    success: result.is_ok(),
                    message: msg,
                    worktree_path,
                    cmd_detail,
                });
            });
        }

        self.is_deleting = false;
        self.input_buffer.clear();
        // Keep state as AppState::Deleting - resolved when poll_delete_status gets the result
    }

    fn prune_worktrees(&mut self) {
        let cmd_detail = format!(
            "git -C {} worktree prune -v",
            self.bare_repo_path.display()
        );

        match git::prune_worktrees(&self.bare_repo_path) {
            Ok(output) => {
                let mut msg = if output.is_empty() {
                    "Prune completed: nothing to prune".to_string()
                } else {
                    format!("Pruned: {}", output)
                };
                if self.verbose {
                    msg = format!("{}\n$ {}", msg, cmd_detail);
                    self.last_command_detail = Some(cmd_detail);
                }
                self.message = Some(AppMessage::info(msg));
                self.refresh_worktrees();
            }
            Err(e) => {
                let mut msg = format!("Prune failed: {}", e);
                if self.verbose {
                    msg = format!("{}\n$ {}", msg, cmd_detail);
                    self.last_command_detail = Some(cmd_detail);
                }
                self.message = Some(AppMessage::error(msg));
            }
        }
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
            let _ = crossterm::execute!(
                std::io::stdout(),
                crossterm::terminal::LeaveAlternateScreen
            );

            let status = Command::new(&editor).arg(&path).status();

            // Restore terminal after editor closes
            let _ = crossterm::terminal::enable_raw_mode();
            let _ = crossterm::execute!(
                std::io::stdout(),
                crossterm::terminal::EnterAlternateScreen
            );

            match status {
                Ok(s) if s.success() => {
                    self.refresh_worktrees();
                }
                Ok(_) => {
                    self.message = Some(AppMessage::error("Editor exited with error"));
                }
                Err(e) => {
                    self.message =
                        Some(AppMessage::error(format!("Failed to open editor: {}", e)));
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

            let path = wt.path.clone();
            let terminal = self.config.get_terminal();

            #[cfg(target_os = "macos")]
            let result = {
                let app = terminal.as_deref().unwrap_or("Terminal");
                Command::new("open").args(["-a", app, &path.to_string_lossy()]).status()
            };

            #[cfg(target_os = "linux")]
            let result = if let Some(term) = terminal {
                Command::new(&term)
                    .current_dir(&path)
                    .status()
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
                Err(std::io::Error::new(std::io::ErrorKind::Other, "Unsupported platform"));

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
        let wt_info = self.selected_worktree().map(|wt| (wt.is_bare, wt.display_name()));

        if let Some((is_bare, name)) = wt_info {
            if is_bare {
                self.message = Some(AppMessage::error("Cannot fetch bare repository"));
                return;
            }
            self.is_fetching = true;
            self.state = AppState::Fetching;
            self.message = Some(AppMessage::info(format!("Fetching: {}...", name)));
        }
    }

    pub fn do_fetch(&mut self) {
        if let Some(wt) = self.selected_worktree().cloned() {
            let cmd_detail = format!(
                "git -C {} fetch origin",
                wt.path.display()
            );

            let result = git::fetch_worktree(&wt.path);

            if self.verbose {
                self.last_command_detail = Some(cmd_detail.clone());
            }

            match result {
                Ok(()) => {
                    let mut msg = format!("Fetch completed: {}", wt.display_name());
                    if self.verbose {
                        msg = format!("{}\n$ {}", msg, cmd_detail);
                    }
                    self.message = Some(AppMessage::info(msg));
                    self.refresh_worktrees();
                }
                Err(e) => {
                    let mut msg = format!("Fetch failed: {}", e);
                    if self.verbose {
                        msg = format!("{}\n$ {}", msg, cmd_detail);
                    }
                    self.message = Some(AppMessage::error(msg));
                }
            }
        }
        self.is_fetching = false;
        self.state = AppState::List;
    }

    fn enter_worktree(&mut self) {
        if let Some(wt) = self.selected_worktree().cloned() {
            if wt.is_bare {
                self.message = Some(AppMessage::error("Cannot enter bare repository"));
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
            let result: Result<std::process::ExitStatus, std::io::Error> =
                Err(std::io::Error::new(std::io::ErrorKind::Other, "Clipboard not supported on this platform"));

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
        let wt_info = self.selected_worktree().map(|wt| (wt.is_bare, wt.display_name(), wt.status.clone()));

        if let Some((is_bare, name, status)) = wt_info {
            if is_bare {
                self.message = Some(AppMessage::error("Cannot pull bare repository"));
                return;
            }
            if status != WorktreeStatus::Clean {
                self.message = Some(AppMessage::error("Cannot pull: worktree has uncommitted changes"));
                return;
            }
            self.is_pulling = true;
            self.state = AppState::Pulling;
            self.message = Some(AppMessage::info(format!("Pulling: {}...", name)));
        }
    }

    fn do_pull(&mut self) {
        if let Some(wt) = self.selected_worktree().cloned() {
            match git::pull_worktree(&wt.path) {
                Ok(msg) => {
                    let display = if msg.is_empty() {
                        format!("Pull completed: {}", wt.display_name())
                    } else {
                        format!("Pull completed: {}", msg)
                    };
                    self.message = Some(AppMessage::info(display));
                    self.refresh_worktrees();
                }
                Err(e) => {
                    self.message = Some(AppMessage::error(format!("Pull failed: {}", e)));
                }
            }
        }
        self.is_pulling = false;
        self.state = AppState::List;
    }

    fn push_worktree(&mut self) {
        let wt_info = self.selected_worktree().map(|wt| (wt.is_bare, wt.display_name()));

        if let Some((is_bare, name)) = wt_info {
            if is_bare {
                self.message = Some(AppMessage::error("Cannot push bare repository"));
                return;
            }
            self.is_pushing = true;
            self.state = AppState::Pushing;
            self.message = Some(AppMessage::info(format!("Pushing: {}...", name)));
        }
    }

    fn do_push(&mut self) {
        if let Some(wt) = self.selected_worktree().cloned() {
            match git::push_worktree(&wt.path) {
                Ok(msg) => {
                    let display = if msg.is_empty() || msg.contains("Everything up-to-date") {
                        "Push completed: Everything up-to-date".to_string()
                    } else {
                        format!("Push completed: {}", wt.display_name())
                    };
                    self.message = Some(AppMessage::info(display));
                    self.refresh_worktrees();
                }
                Err(e) => {
                    self.message = Some(AppMessage::error(format!("Push failed: {}", e)));
                }
            }
        }
        self.is_pushing = false;
        self.state = AppState::List;
    }

    fn merge_upstream(&mut self) {
        let wt_info = self.selected_worktree().map(|wt| (wt.is_bare, wt.display_name(), wt.status.clone()));

        if let Some((is_bare, name, status)) = wt_info {
            if is_bare {
                self.message = Some(AppMessage::error("Cannot merge into bare repository"));
                return;
            }
            if status != WorktreeStatus::Clean {
                self.message = Some(AppMessage::error("Cannot merge: worktree has uncommitted changes"));
                return;
            }
            self.merge_source_branch = None; // upstream merge
            self.is_merging = true;
            self.state = AppState::Merging;
            self.message = Some(AppMessage::info(format!("Merging upstream into {}...", name)));
        }
    }

    fn open_merge_branch_select(&mut self) {
        let wt_info = self.selected_worktree().map(|wt| (wt.is_bare, wt.status.clone()));

        if let Some((is_bare, status)) = wt_info {
            if is_bare {
                self.message = Some(AppMessage::error("Cannot merge into bare repository"));
                return;
            }
            if status != WorktreeStatus::Clean {
                self.message = Some(AppMessage::error("Cannot merge: worktree has uncommitted changes"));
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
                    self.message = Some(AppMessage::error(format!("Failed to list branches: {}", e)));
                }
            }
        }
    }

    fn handle_merge_branch_select_input(&mut self, code: KeyCode, branches: Vec<String>, selected: usize) {
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
                    self.merge_source_branch = Some(branch.clone());
                    self.is_merging = true;
                    self.state = AppState::Merging;
                    self.message = Some(AppMessage::info(format!("Merging {}...", branch)));
                }
            }
            _ => {}
        }
    }

    fn do_merge(&mut self) {
        if let Some(wt) = self.selected_worktree().cloned() {
            let result = if let Some(ref source_branch) = self.merge_source_branch {
                git::merge_branch(&wt.path, source_branch)
            } else {
                git::merge_upstream(&wt.path)
            };

            match result {
                Ok(msg) => {
                    let display = if msg.is_empty() {
                        "Merge completed".to_string()
                    } else {
                        format!("Merge completed: {}", msg)
                    };
                    self.message = Some(AppMessage::info(display));
                    self.refresh_worktrees();
                }
                Err(e) => {
                    self.message = Some(AppMessage::error(format!("Merge failed: {}", e)));
                }
            }
        }
        self.is_merging = false;
        self.merge_source_branch = None;
        self.state = AppState::List;
    }

}
