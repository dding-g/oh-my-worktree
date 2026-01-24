use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{DefaultTerminal, Frame};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use crate::git;
use crate::types::{AppMessage, AppState, Worktree};
use crate::ui::{add_modal, confirm_modal, main_view};

pub struct App {
    pub worktrees: Vec<Worktree>,
    pub selected_index: usize,
    pub state: AppState,
    pub message: Option<AppMessage>,
    pub bare_repo_path: PathBuf,
    pub input_buffer: String,
    pub should_quit: bool,
}

impl App {
    pub fn new(bare_repo_path: PathBuf) -> Result<Self> {
        let worktrees = git::list_worktrees(&bare_repo_path)?;
        Ok(Self {
            worktrees,
            selected_index: 0,
            state: AppState::List,
            message: None,
            bare_repo_path,
            input_buffer: String::new(),
            should_quit: false,
        })
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events(terminal)?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        match self.state {
            AppState::List => main_view::render(frame, self),
            AppState::AddModal => {
                main_view::render(frame, self);
                add_modal::render(frame, self);
            }
            AppState::ConfirmDelete => {
                main_view::render(frame, self);
                confirm_modal::render(frame, self);
            }
        }
    }

    fn handle_events(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
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
                        AppState::ConfirmDelete => self.handle_confirm_delete_input(key.code),
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
            KeyCode::Char('a') => {
                self.state = AppState::AddModal;
                self.input_buffer.clear();
            }
            KeyCode::Char('d') => {
                if let Some(wt) = self.selected_worktree() {
                    if wt.is_bare {
                        self.message = Some(AppMessage::error("Cannot delete bare repository"));
                    } else {
                        self.state = AppState::ConfirmDelete;
                    }
                }
            }
            KeyCode::Char('o') => self.open_editor(),
            KeyCode::Char('t') => self.open_terminal(),
            KeyCode::Char('f') => self.fetch_all(),
            KeyCode::Char('r') => self.refresh_worktrees(),
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

    fn handle_confirm_delete_input(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc | KeyCode::Char('n') => {
                self.state = AppState::List;
            }
            KeyCode::Char('y') | KeyCode::Enter => {
                self.delete_selected_worktree();
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

        // Generate worktree path: sibling to bare repo with branch name
        let worktree_path = self
            .bare_repo_path
            .parent()
            .map(|p| p.join(&branch))
            .unwrap_or_else(|| PathBuf::from(&branch));

        match git::add_worktree(&self.bare_repo_path, &branch, &worktree_path, None) {
            Ok(()) => {
                self.message = Some(AppMessage::info(format!("Created worktree: {}", branch)));
                self.refresh_worktrees();
                self.state = AppState::List;
                self.input_buffer.clear();
            }
            Err(e) => {
                self.message = Some(AppMessage::error(format!("Failed to create: {}", e)));
            }
        }
    }

    fn delete_selected_worktree(&mut self) {
        if let Some(wt) = self.selected_worktree().cloned() {
            if wt.is_bare {
                self.message = Some(AppMessage::error("Cannot delete bare repository"));
                self.state = AppState::List;
                return;
            }

            let force = wt.status != crate::types::WorktreeStatus::Clean;
            match git::remove_worktree(&self.bare_repo_path, &wt.path, force) {
                Ok(()) => {
                    self.message = Some(AppMessage::info(format!(
                        "Deleted worktree: {}",
                        wt.display_name()
                    )));
                    self.refresh_worktrees();
                }
                Err(e) => {
                    self.message = Some(AppMessage::error(format!("Failed to delete: {}", e)));
                }
            }
        }
        self.state = AppState::List;
    }

    fn open_editor(&mut self) {
        if let Some(wt) = self.selected_worktree() {
            if wt.is_bare {
                self.message = Some(AppMessage::error("Cannot open bare repository in editor"));
                return;
            }

            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
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
            let terminal = std::env::var("TERMINAL").ok();

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
        self.message = Some(AppMessage::info("Fetching..."));
        match git::fetch_all(&self.bare_repo_path) {
            Ok(()) => {
                self.message = Some(AppMessage::info("Fetch completed"));
                self.refresh_worktrees();
            }
            Err(e) => {
                self.message = Some(AppMessage::error(format!("Fetch failed: {}", e)));
            }
        }
    }

    pub fn generated_worktree_path(&self) -> PathBuf {
        let branch = self.input_buffer.trim();
        self.bare_repo_path
            .parent()
            .map(|p| p.join(branch))
            .unwrap_or_else(|| PathBuf::from(branch))
    }
}
