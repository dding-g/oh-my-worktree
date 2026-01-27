use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{backend::Backend, Frame, Terminal};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use crate::config::Config;
use crate::git;
use crate::types::{AddWorktreeState, AppMessage, AppState, BaseSource, ExitAction, SortMode, Worktree, WorktreeStatus};
use crate::ui::{add_modal, config_modal, confirm_modal, help_modal, main_view};

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
    pub has_shell_integration: bool, // Whether OWT_OUTPUT_FILE is set
    pub filter_text: String,         // Search/filter text
    pub is_filtering: bool,          // Whether in filter mode
    pub last_key: Option<char>,      // For gg detection
    pub sort_mode: SortMode,         // Current sort mode
    pub add_worktree_state: AddWorktreeState,  // State for add worktree modal
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

        // Initialize add_worktree_state with default base branch
        let default_branch = git::get_default_branch(&bare_repo_path).unwrap_or_else(|_| "main".to_string());
        let mut add_worktree_state = AddWorktreeState::default();
        add_worktree_state.base_branch = default_branch;

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
            has_shell_integration,
            filter_text: String::new(),
            is_filtering: false,
            last_key: None,
            sort_mode: SortMode::default(),
            add_worktree_state,
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
                // Drain pending key events to prevent accidental actions
                while event::poll(Duration::from_millis(0))? {
                    let _ = event::read()?;
                }
                continue;
            }

            self.handle_events(terminal)?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        match self.state {
            AppState::List | AppState::Fetching | AppState::Adding | AppState::Deleting => {
                main_view::render(frame, self)
            }
            AppState::AddModal => {
                main_view::render(frame, self);
                add_modal::render(frame, self);
            }
            AppState::AddTypeSelect => {
                main_view::render(frame, self);
                add_modal::render_type_select(frame, self);
            }
            AppState::AddBranchInput => {
                main_view::render(frame, self);
                add_modal::render_branch_input(frame, self);
            }
            AppState::ConfirmDelete { .. } => {
                main_view::render(frame, self);
                confirm_modal::render(frame, self);
            }
            AppState::ConfigModal { .. } => {
                main_view::render(frame, self);
                config_modal::render(frame, self);
            }
            AppState::BranchTypesModal { .. } => {
                main_view::render(frame, self);
                config_modal::render_branch_types(frame, self);
            }
            AppState::SetupModal { .. } => {
                main_view::render(frame, self);
                // TODO: setup_modal::render(frame, self);
            }
            AppState::HelpModal => {
                main_view::render(frame, self);
                help_modal::render(frame);
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

                    match self.state.clone() {
                        AppState::List => self.handle_list_input(key.code, key.modifiers),
                        AppState::AddModal => self.handle_add_modal_input(key.code),
                        AppState::AddTypeSelect => self.handle_add_type_select_input(key.code),
                        AppState::AddBranchInput => self.handle_add_branch_input(key.code, key.modifiers),
                        AppState::ConfirmDelete { delete_branch } => {
                            self.handle_confirm_delete_input(key.code, delete_branch)
                        }
                        AppState::ConfigModal { selected_index, editing } => {
                            self.handle_config_modal_input(key.code, selected_index, editing)
                        }
                        AppState::BranchTypesModal { selected_index, editing_field } => {
                            self.handle_branch_types_modal_input(key.code, selected_index, editing_field)
                        }
                        AppState::SetupModal { selected_index } => {
                            self.handle_setup_modal_input(key.code, selected_index)
                        }
                        AppState::HelpModal => self.handle_help_modal_input(key.code),
                        AppState::Fetching | AppState::Adding | AppState::Deleting => {
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
                // Reset add worktree state and open type selection
                let default_branch = git::get_default_branch(&self.bare_repo_path)
                    .unwrap_or_else(|_| "main".to_string());
                self.add_worktree_state = AddWorktreeState::default();
                self.add_worktree_state.base_branch = default_branch;
                self.state = AppState::AddTypeSelect;
                self.input_buffer.clear();
                self.last_key = None;
            }
            KeyCode::Char('d') => {
                if let Some(wt) = self.selected_worktree() {
                    if wt.is_bare {
                        self.message = Some(AppMessage::error("Cannot delete bare repository"));
                    } else if wt.status != WorktreeStatus::Clean {
                        self.message = Some(AppMessage::error(
                            "Cannot delete: worktree has uncommitted changes. Commit or stash changes first."
                        ));
                    } else {
                        self.state = AppState::ConfirmDelete { delete_branch: false };
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
            KeyCode::Char('r') => {
                self.refresh_worktrees();
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
            KeyCode::Char('?') => {
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

    fn handle_confirm_delete_input(&mut self, code: KeyCode, delete_branch: bool) {
        match code {
            KeyCode::Esc | KeyCode::Char('n') => {
                self.state = AppState::List;
            }
            KeyCode::Char('y') | KeyCode::Enter => {
                self.delete_selected_worktree(delete_branch);
            }
            KeyCode::Char('b') => {
                // Toggle delete branch option
                self.state = AppState::ConfirmDelete { delete_branch: !delete_branch };
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
                    } else if selected == 4 {
                        // branch_types - open BranchTypesModal
                        self.state = AppState::BranchTypesModal {
                            selected_index: 0,
                            editing_field: None,
                        };
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
            _ => {}
        }
    }

    /// Handle input for branch type selection screen
    fn handle_add_type_select_input(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => {
                self.state = AppState::List;
            }
            // Shortcut keys for branch types
            KeyCode::Char(c) => {
                if c == 'c' {
                    // Custom mode - go directly to branch input
                    let default_branch = git::get_default_branch(&self.bare_repo_path)
                        .unwrap_or_else(|_| "main".to_string());
                    self.add_worktree_state = AddWorktreeState::custom(default_branch);
                    self.input_buffer.clear();
                    self.state = AppState::AddBranchInput;
                } else if let Some(bt) = self.config.find_branch_type_by_shortcut(c).cloned() {
                    // Found a matching branch type
                    self.add_worktree_state = AddWorktreeState::with_branch_type(bt);
                    self.input_buffer = self.add_worktree_state.branch_name.clone();
                    self.state = AppState::AddBranchInput;
                }
            }
            _ => {}
        }
    }

    /// Handle input for branch name input screen
    fn handle_add_branch_input(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        match code {
            KeyCode::Esc => {
                // Go back to type selection
                self.state = AppState::AddTypeSelect;
                self.input_buffer.clear();
            }
            KeyCode::Enter => {
                if !self.input_buffer.trim().is_empty() {
                    self.add_worktree_state.branch_name = self.input_buffer.trim().to_string();
                    self.add_worktree_with_state();
                }
            }
            KeyCode::Backspace => {
                // Don't allow backspace past the prefix if using branch type
                if let Some(ref bt) = self.add_worktree_state.branch_type {
                    if self.input_buffer.len() > bt.prefix.len() {
                        self.input_buffer.pop();
                    }
                } else {
                    self.input_buffer.pop();
                }
            }
            KeyCode::Char('f') if modifiers.contains(KeyModifiers::SHIFT) => {
                // Fetch remote base branch
                self.fetch_base_branch();
            }
            KeyCode::Char('u') if modifiers.contains(KeyModifiers::SHIFT) => {
                // Use remote as base
                self.add_worktree_state.base_source = BaseSource::Remote;
                self.message = Some(AppMessage::info("Using remote as base"));
            }
            KeyCode::Char('l') if modifiers.contains(KeyModifiers::SHIFT) => {
                // Use local as base
                self.add_worktree_state.base_source = BaseSource::Local;
                self.message = Some(AppMessage::info("Using local as base"));
            }
            KeyCode::Char('b') if modifiers.contains(KeyModifiers::SHIFT) => {
                // Change base branch (show list)
                self.message = Some(AppMessage::info("Base branch selection not yet implemented"));
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
            }
            _ => {}
        }
    }

    /// Fetch the base branch from remote
    fn fetch_base_branch(&mut self) {
        let base = self.add_worktree_state.base_branch.clone();
        self.message = Some(AppMessage::info(format!("Fetching {}...", base)));

        match git::fetch_branch(&self.bare_repo_path, &base) {
            Ok(()) => {
                self.message = Some(AppMessage::info(format!("Fetched {}", base)));
            }
            Err(e) => {
                self.message = Some(AppMessage::error(format!("Failed to fetch: {}", e)));
            }
        }
    }

    /// Add worktree using the current add_worktree_state
    fn add_worktree_with_state(&mut self) {
        let branch = self.add_worktree_state.branch_name.trim().to_string();
        if branch.is_empty() {
            self.message = Some(AppMessage::error("Branch name cannot be empty"));
            return;
        }

        // Determine base branch reference
        let base_ref = match self.add_worktree_state.base_source {
            BaseSource::Local => self.add_worktree_state.base_branch.clone(),
            BaseSource::Remote => format!("origin/{}", self.add_worktree_state.base_branch),
        };

        // Store branch name for do_add_worktree
        self.input_buffer = branch.clone();

        self.is_adding = true;
        self.state = AppState::Adding;
        self.message = Some(AppMessage::info(format!("Creating worktree: {} from {}...", branch, base_ref)));
    }

    /// Handle input for branch types modal
    fn handle_branch_types_modal_input(&mut self, code: KeyCode, selected_index: usize, editing_field: Option<usize>) {
        let branch_type_count = self.config.branch_types.len();

        if let Some(field) = editing_field {
            // Editing mode
            match code {
                KeyCode::Esc => {
                    self.input_buffer.clear();
                    self.state = AppState::BranchTypesModal {
                        selected_index,
                        editing_field: None,
                    };
                }
                KeyCode::Enter => {
                    // Save the edited value
                    self.apply_branch_type_edit(selected_index, field);
                    self.input_buffer.clear();
                    self.state = AppState::BranchTypesModal {
                        selected_index,
                        editing_field: None,
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
            // Navigation mode
            match code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.state = AppState::ConfigModal {
                        selected_index: 4, // branch_types item
                        editing: false,
                    };
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if selected_index > 0 {
                        self.state = AppState::BranchTypesModal {
                            selected_index: selected_index - 1,
                            editing_field: None,
                        };
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if selected_index < branch_type_count.saturating_sub(1) {
                        self.state = AppState::BranchTypesModal {
                            selected_index: selected_index + 1,
                            editing_field: None,
                        };
                    }
                }
                KeyCode::Char('b') => {
                    // Edit base branch
                    if selected_index < branch_type_count {
                        self.input_buffer = self.config.branch_types[selected_index].base.clone();
                        self.state = AppState::BranchTypesModal {
                            selected_index,
                            editing_field: Some(0),
                        };
                    }
                }
                KeyCode::Char('s') => {
                    // Save config
                    self.save_config();
                }
                _ => {}
            }
        }
    }

    /// Apply edit to branch type
    fn apply_branch_type_edit(&mut self, index: usize, field: usize) {
        if index >= self.config.branch_types.len() {
            return;
        }
        let value = self.input_buffer.trim().to_string();
        match field {
            0 => self.config.branch_types[index].base = value,
            _ => {}
        }
        self.message = Some(AppMessage::info("Branch type updated (press 's' to save)"));
    }

    /// Handle input for setup modal
    fn handle_setup_modal_input(&mut self, code: KeyCode, _selected_index: usize) {
        match code {
            KeyCode::Esc => {
                self.state = AppState::List;
            }
            KeyCode::Enter => {
                // Save configuration and close
                match self.config.save_to_project(&self.bare_repo_path) {
                    Ok(()) => {
                        self.message = Some(AppMessage::info("Configuration saved"));
                    }
                    Err(e) => {
                        self.message = Some(AppMessage::error(format!("Failed to save: {}", e)));
                    }
                }
                self.state = AppState::List;
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
        let half_page = 10; // Approximate half page
        let max_index = self.worktrees.len().saturating_sub(1);
        self.selected_index = (self.selected_index + half_page).min(max_index);
    }

    fn move_selection_half_page_up(&mut self) {
        let half_page = 10; // Approximate half page
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

        // Determine base branch reference from add_worktree_state
        let base_ref = match self.add_worktree_state.base_source {
            BaseSource::Local => Some(self.add_worktree_state.base_branch.clone()),
            BaseSource::Remote => Some(format!("origin/{}", self.add_worktree_state.base_branch)),
        };

        match git::add_worktree(&self.bare_repo_path, &branch, &worktree_path, base_ref.as_deref()) {
            Ok(()) => {
                // Copy files if configured
                self.copy_configured_files(&worktree_path);

                // Run post-add script if exists
                self.run_post_add_script(&worktree_path);

                self.message = Some(AppMessage::info(format!("Created worktree: {}", branch)));
                self.refresh_worktrees();
            }
            Err(e) => {
                self.message = Some(AppMessage::error(format!("Failed to create: {}", e)));
            }
        }

        self.is_adding = false;
        self.state = AppState::List;
        self.input_buffer.clear();
        // Reset add_worktree_state
        self.add_worktree_state = AddWorktreeState::default();
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

    fn run_post_add_script(&self, worktree_path: &PathBuf) {
        let script_path = Config::post_add_script_path(&self.bare_repo_path);

        if !script_path.exists() {
            return;
        }

        // Run the script in the worktree directory
        let _ = Command::new("sh")
            .arg(&script_path)
            .current_dir(worktree_path)
            .output();
    }

    fn delete_selected_worktree(&mut self, delete_branch: bool) {
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
            // Store delete_branch flag in input_buffer (hacky but works)
            self.input_buffer = if delete_branch { "delete_branch".to_string() } else { String::new() };
        }
    }

    fn do_delete_worktree(&mut self) {
        let delete_branch = self.input_buffer == "delete_branch";

        if let Some(wt) = self.selected_worktree().cloned() {
            let branch_name = wt.branch.clone();

            match git::remove_worktree(&self.bare_repo_path, &wt.path, false) {
                Ok(()) => {
                    let mut msg = format!("Deleted worktree: {}", wt.display_name());

                    // Delete branch if requested
                    if delete_branch {
                        if let Some(ref branch) = branch_name {
                            match git::delete_branch(&self.bare_repo_path, branch, false) {
                                Ok(()) => {
                                    msg.push_str(&format!(" (branch '{}' deleted)", branch));
                                }
                                Err(e) => {
                                    msg.push_str(&format!(" (branch delete failed: {})", e));
                                }
                            }
                        }
                    }

                    self.message = Some(AppMessage::info(msg));
                    self.refresh_worktrees();
                }
                Err(e) => {
                    self.message = Some(AppMessage::error(format!("Failed to delete: {}", e)));
                }
            }
        }

        self.is_deleting = false;
        self.state = AppState::List;
        self.input_buffer.clear();
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
            match git::fetch_worktree(&wt.path) {
                Ok(()) => {
                    self.message = Some(AppMessage::info(format!("Fetch completed: {}", wt.display_name())));
                    self.refresh_worktrees();
                }
                Err(e) => {
                    self.message = Some(AppMessage::error(format!("Fetch failed: {}", e)));
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

}
