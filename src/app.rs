use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{backend::Backend, Frame, Terminal};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use crate::config::Config;
use crate::git;
use crate::types::{AppMessage, AppState, ExitAction, Worktree, WorktreeStatus};
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
}

impl App {
    pub fn new(bare_repo_path: PathBuf, launch_path: Option<PathBuf>) -> Result<Self> {
        let worktrees = git::list_worktrees(&bare_repo_path)?;
        let config = Config::load().unwrap_or_default();

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

        // Set initial selection to current worktree if found
        let selected_index = current_worktree_path.as_ref()
            .and_then(|cp| worktrees.iter().position(|wt| wt.path == *cp))
            .unwrap_or(0);

        Ok(Self {
            worktrees,
            selected_index,
            state: AppState::List,
            message: None,
            bare_repo_path,
            input_buffer: String::new(),
            should_quit: false,
            config,
            exit_action: ExitAction::Quit,
            is_fetching: false,
            current_worktree_path,
            is_adding: false,
            is_deleting: false,
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
                continue;
            }

            if self.is_deleting {
                self.do_delete_worktree();
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

                    match self.state {
                        AppState::List => self.handle_list_input(key.code, key.modifiers),
                        AppState::AddModal => self.handle_add_modal_input(key.code),
                        AppState::ConfirmDelete { delete_branch } => {
                            self.handle_confirm_delete_input(key.code, delete_branch)
                        }
                        AppState::ConfigModal { selected_index, editing } => {
                            self.handle_config_modal_input(key.code, selected_index, editing)
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
        match code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Up | KeyCode::Char('k') => self.move_selection_up(),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection_down(),
            KeyCode::Enter => self.enter_worktree(),
            KeyCode::Char('a') => {
                self.state = AppState::AddModal;
                self.input_buffer.clear();
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
            }
            KeyCode::Char('o') => self.open_editor(),
            KeyCode::Char('t') => self.open_terminal(),
            KeyCode::Char('f') => self.fetch_all(),
            KeyCode::Char('r') => self.refresh_worktrees(),
            KeyCode::Char('c') => {
                self.state = AppState::ConfigModal {
                    selected_index: 0,
                    editing: false,
                };
            }
            KeyCode::Char('?') => {
                self.state = AppState::HelpModal;
            }
            _ => {}
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

    pub fn selected_worktree(&self) -> Option<&Worktree> {
        self.worktrees.get(self.selected_index)
    }

    fn refresh_worktrees(&mut self) {
        match git::list_worktrees(&self.bare_repo_path) {
            Ok(worktrees) => {
                self.worktrees = worktrees;
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

        match git::add_worktree(&self.bare_repo_path, &branch, &worktree_path, None) {
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
        if let Some(wt) = self.selected_worktree() {
            if wt.is_bare {
                self.message = Some(AppMessage::error("Cannot enter bare repository"));
                return;
            }
            self.exit_action = ExitAction::ChangeDirectory(wt.path.clone());
            self.should_quit = true;
        }
    }

}
