use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::App;
use crate::config::Config;

pub fn render(frame: &mut Frame, app: &App) {
    let area = centered_rect(70, 50, frame.area());

    // Clear the background
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Config Settings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(1), // Spacing
        Constraint::Length(1), // Config path header
        Constraint::Length(1), // Config path value
        Constraint::Length(1), // Spacing
        Constraint::Length(1), // Settings header
        Constraint::Length(1), // Editor
        Constraint::Length(1), // Terminal
        Constraint::Length(1), // Copy files
        Constraint::Length(1), // Post-add script
        Constraint::Min(1),    // Spacing
        Constraint::Length(1), // Help
    ])
    .split(inner);

    // Config path header
    let path_header = Paragraph::new(Line::from(vec![Span::styled(
        "Config File:",
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
    )]));
    frame.render_widget(path_header, chunks[1]);

    // Config path value
    let config_path = get_config_path();
    let path_value = Paragraph::new(Line::from(vec![Span::styled(
        config_path,
        Style::default().fg(Color::DarkGray),
    )]));
    frame.render_widget(path_value, chunks[2]);

    // Settings header
    let settings_header = Paragraph::new(Line::from(vec![Span::styled(
        "Current Settings:",
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
    )]));
    frame.render_widget(settings_header, chunks[4]);

    // Editor
    let editor_value = app.config.editor.as_deref().unwrap_or("(not set, using $EDITOR or vim)");
    let editor = Paragraph::new(Line::from(vec![
        Span::styled("  editor: ", Style::default().fg(Color::Cyan)),
        Span::styled(editor_value, Style::default().fg(Color::White)),
    ]));
    frame.render_widget(editor, chunks[5]);

    // Terminal
    let terminal_value = app.config.terminal.as_deref().unwrap_or("(not set, using $TERMINAL or default)");
    let terminal = Paragraph::new(Line::from(vec![
        Span::styled("  terminal: ", Style::default().fg(Color::Cyan)),
        Span::styled(terminal_value, Style::default().fg(Color::White)),
    ]));
    frame.render_widget(terminal, chunks[6]);

    // Copy files
    let copy_files_value = if app.config.copy_files.is_empty() {
        "(none)".to_string()
    } else {
        app.config.copy_files.join(", ")
    };
    let copy_files = Paragraph::new(Line::from(vec![
        Span::styled("  copy_files: ", Style::default().fg(Color::Cyan)),
        Span::styled(copy_files_value, Style::default().fg(Color::White)),
    ]));
    frame.render_widget(copy_files, chunks[7]);

    // Post-add script
    let script_path = Config::post_add_script_path(&app.bare_repo_path);
    let script_status = if script_path.exists() {
        format!("{}", script_path.display())
    } else {
        "(not found)".to_string()
    };
    let post_add = Paragraph::new(Line::from(vec![
        Span::styled("  post_add_script: ", Style::default().fg(Color::Cyan)),
        Span::styled(script_status, Style::default().fg(Color::White)),
    ]));
    frame.render_widget(post_add, chunks[8]);

    // Help text
    let help = Paragraph::new(Line::from(vec![
        Span::styled("Esc", Style::default().fg(Color::Cyan)),
        Span::raw(" close"),
    ]))
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[10]);
}

fn get_config_path() -> String {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return format!("{}/owt/config.toml", xdg);
    }

    if let Ok(home) = std::env::var("HOME") {
        return format!("{}/.config/owt/config.toml", home);
    }

    ".config/owt/config.toml".to_string()
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
