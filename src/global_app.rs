use std::path::PathBuf;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::Backend,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame, Terminal,
};

use crate::types::ExitAction;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Home,
    Clone,
    InitGuide,
    Help,
    About,
}

pub(crate) struct GlobalApp {
    screen: Screen,
    url: String,
    path: String,
    path_editing: bool,
    pub(crate) exit_action: ExitAction,
    should_quit: bool,
}

impl GlobalApp {
    pub(crate) fn new(_launch_path: PathBuf) -> Self {
        Self {
            screen: Screen::Home,
            url: String::new(),
            path: String::new(),
            path_editing: false,
            exit_action: ExitAction::Quit,
            should_quit: false,
        }
    }

    pub(crate) fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| self.draw(frame))?;
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle(key.code);
                    }
                }
            }
        }
        Ok(())
    }

    fn handle(&mut self, code: KeyCode) {
        match self.screen {
            Screen::Home => match code {
                KeyCode::Char('c') => self.screen = Screen::Clone,
                KeyCode::Char('i') => self.screen = Screen::InitGuide,
                KeyCode::Char('s') => {
                    self.exit_action = ExitAction::InstallShellSetup;
                    self.should_quit = true;
                }
                KeyCode::Char('?') | KeyCode::Char('h') => self.screen = Screen::Help,
                KeyCode::Char('v') => self.screen = Screen::About,
                KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                _ => {}
            },
            Screen::Clone => match code {
                KeyCode::Esc => self.screen = Screen::Home,
                KeyCode::Tab => self.path_editing = !self.path_editing,
                KeyCode::Enter if !self.url.trim().is_empty() => {
                    self.exit_action = ExitAction::CloneWorkspace {
                        url: self.url.trim().to_string(),
                        path: (!self.path.trim().is_empty())
                            .then(|| PathBuf::from(self.path.trim())),
                    };
                    self.should_quit = true;
                }
                KeyCode::Backspace => {
                    if self.path_editing {
                        self.path.pop();
                    } else {
                        self.url.pop();
                    }
                }
                KeyCode::Char(value) => {
                    if self.path_editing {
                        self.path.push(value);
                    } else {
                        self.url.push(value);
                    }
                }
                _ => {}
            },
            _ => {
                if matches!(code, KeyCode::Esc | KeyCode::Char('q')) {
                    self.screen = Screen::Home;
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        frame.render_widget(Clear, area);
        let block = Block::default()
            .title(" owt Global Home ")
            .borders(Borders::ALL);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let lines = match self.screen {
            Screen::Home => vec![
                Line::from("Use owt without opening a repository."),
                Line::from(""),
                Line::from("c  Clone .bare workspace"),
                Line::from("i  Show .bare conversion guide"),
                Line::from("s  Install shell integration"),
                Line::from("?  CLI help"),
                Line::from("v  About/version"),
                Line::from("q  Quit"),
            ],
            Screen::Clone => vec![
                Line::from("Clone workspace (Enter executes after terminal restore)"),
                Line::from(vec![
                    Span::raw("URL:  "),
                    Span::styled(
                        format!(
                            "[{}{}]",
                            self.url,
                            if !self.path_editing { "█" } else { "" }
                        ),
                        Style::default(),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("Path: "),
                    Span::styled(
                        format!(
                            "[{}{}]",
                            self.path,
                            if self.path_editing {
                                "█"
                            } else {
                                " (default)"
                            }
                        ),
                        Style::default(),
                    ),
                ]),
                Line::from("Tab field  Esc cancel"),
            ],
            Screen::InitGuide => vec![
                Line::from("owt init conversion guide"),
                Line::from(
                    "Run `owt init` inside a regular Git repository to print the exact safe guide.",
                ),
                Line::from(
                    "This TUI is outside a repository, so it cannot infer a repository name.",
                ),
                Line::from("Esc close"),
            ],
            Screen::Help => vec![
                Line::from("CLI: clone, init, setup, worktree, pr, commit, search"),
                Line::from("Run `owt <command> --help` for command-specific options."),
                Line::from("Esc close"),
            ],
            Screen::About => vec![
                Line::from(format!("owt v{}", env!("CARGO_PKG_VERSION"))),
                Line::from("Interactive wrapper for owt CLI worktree capabilities."),
                Line::from("Esc close"),
            ],
        };
        frame.render_widget(Paragraph::new(lines), inner);
    }
}
