use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::App;
use crate::config::Config;
use crate::types::AppState;

pub const CONFIG_ITEM_COUNT: usize = 5; // Added branch_types

pub fn render(frame: &mut Frame, app: &App) {
    let (selected_index, editing) = match app.state {
        AppState::ConfigModal { selected_index, editing } => (selected_index, editing),
        _ => (0, false),
    };

    let area = centered_rect(60, 70, frame.area());

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
        Constraint::Length(1), // Spacing
        Constraint::Length(1), // Branch Types header
        Constraint::Length(1), // Branch Types summary
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
        "Settings:",
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
    )]));
    frame.render_widget(settings_header, chunks[4]);

    // Render each config item
    render_config_item(frame, chunks[5], "editor", &get_editor_display(app, editing), selected_index == 0, editing && selected_index == 0, &app.input_buffer);
    render_config_item(frame, chunks[6], "terminal", &get_terminal_display(app, editing), selected_index == 1, editing && selected_index == 1, &app.input_buffer);
    render_config_item(frame, chunks[7], "copy_files", &get_copy_files_display(app, editing), selected_index == 2, editing && selected_index == 2, &app.input_buffer);
    render_config_item(frame, chunks[8], "post_add_script", &get_script_display(app), selected_index == 3, false, &app.input_buffer);

    // Branch Types header
    let bt_header = Paragraph::new(Line::from(vec![Span::styled(
        "Branch Types:",
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
    )]));
    frame.render_widget(bt_header, chunks[10]);

    // Branch Types summary with selection
    render_branch_types_summary(frame, chunks[11], app, selected_index == 4);

    // Help text
    let help_text = if editing {
        vec![
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::raw(" save  "),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::raw(" cancel"),
        ]
    } else {
        vec![
            Span::styled("j/k", Style::default().fg(Color::Cyan)),
            Span::raw(" nav  "),
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::raw(" edit  "),
            Span::styled("s", Style::default().fg(Color::Cyan)),
            Span::raw(" save  "),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::raw(" close"),
        ]
    };
    let help = Paragraph::new(Line::from(help_text))
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[13]);
}

fn render_branch_types_summary(frame: &mut Frame, area: Rect, app: &App, is_selected: bool) {
    let cursor = if is_selected { "> " } else { "  " };
    let label_style = if is_selected {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    };

    // Create summary of branch types
    let summary: Vec<String> = app.config.branch_types.iter()
        .map(|bt| format!("[{}]{} → {}", bt.shortcut, bt.name, bt.base))
        .collect();
    let summary_str = summary.join("  ");

    let hint = if is_selected {
        " (Enter to edit)"
    } else {
        ""
    };

    let line = Paragraph::new(Line::from(vec![
        Span::styled(cursor, label_style),
        Span::styled(&summary_str, Style::default().fg(Color::White)),
        Span::styled(hint, Style::default().fg(Color::DarkGray)),
    ]));
    frame.render_widget(line, area);
}

fn render_config_item(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    is_selected: bool,
    is_editing: bool,
    input_buffer: &str,
) {
    let cursor = if is_selected { "> " } else { "  " };
    let label_style = if is_selected {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    };

    let spans = if is_editing {
        // Show input buffer with cursor indicator
        let display_value = format!("[{}█]", input_buffer);
        vec![
            Span::styled(cursor, label_style),
            Span::styled(format!("{}: ", label), label_style),
            Span::styled(display_value, Style::default().fg(Color::Yellow)),
        ]
    } else if label == "post_add_script" && is_selected {
        // Special hint for post_add_script
        vec![
            Span::styled(cursor, label_style),
            Span::styled(format!("{}: ", label), label_style),
            Span::styled(value, Style::default().fg(Color::White)),
            Span::styled(" (Enter to edit with $EDITOR)", Style::default().fg(Color::DarkGray)),
        ]
    } else {
        let value_style = if is_selected {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        vec![
            Span::styled(cursor, label_style),
            Span::styled(format!("{}: ", label), label_style),
            Span::styled(value, value_style),
        ]
    };

    let line = Paragraph::new(Line::from(spans));
    frame.render_widget(line, area);
}

fn get_editor_display(app: &App, _editing: bool) -> String {
    app.config.editor.as_deref().unwrap_or("(not set)").to_string()
}

fn get_terminal_display(app: &App, _editing: bool) -> String {
    app.config.terminal.as_deref().unwrap_or("(not set)").to_string()
}

fn get_copy_files_display(app: &App, _editing: bool) -> String {
    if app.config.copy_files.is_empty() {
        "(none)".to_string()
    } else {
        app.config.copy_files.join(", ")
    }
}

fn get_script_display(app: &App) -> String {
    let script_path = Config::post_add_script_path(&app.bare_repo_path);
    if script_path.exists() {
        format!("{}", script_path.display())
    } else {
        "(not found)".to_string()
    }
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

/// Render the branch types editing modal
pub fn render_branch_types(frame: &mut Frame, app: &App) {
    let (selected_index, editing_field) = match app.state {
        AppState::BranchTypesModal { selected_index, editing_field } => (selected_index, editing_field),
        _ => (0, None),
    };

    let area = centered_rect(60, 60, frame.area());

    // Clear the background
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Branch Types ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Calculate dynamic layout
    let bt_count = app.config.branch_types.len();
    let mut constraints = vec![
        Constraint::Length(1), // Spacing
        Constraint::Length(1), // Header row
        Constraint::Length(1), // Separator
    ];

    // Add constraint for each branch type
    for _ in 0..bt_count {
        constraints.push(Constraint::Length(1));
    }

    constraints.extend([
        Constraint::Min(1),    // Flexible spacing
        Constraint::Length(1), // Help
    ]);

    let chunks = Layout::vertical(constraints).split(inner);

    // Header
    let header = Paragraph::new(Line::from(vec![
        Span::styled("  Key   Name       Base Branch", Style::default().fg(Color::DarkGray)),
    ]));
    frame.render_widget(header, chunks[1]);

    // Separator
    let separator = Paragraph::new(Line::from(vec![
        Span::styled("  ─────────────────────────────────────", Style::default().fg(Color::DarkGray)),
    ]));
    frame.render_widget(separator, chunks[2]);

    // Branch types
    for (i, bt) in app.config.branch_types.iter().enumerate() {
        let is_selected = i == selected_index;
        let cursor = if is_selected { "> " } else { "  " };

        let base_display = if is_selected && editing_field == Some(0) {
            format!("[{}█]", app.input_buffer)
        } else {
            bt.base.clone()
        };

        let name_style = if is_selected {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let base_style = if is_selected && editing_field == Some(0) {
            Style::default().fg(Color::Yellow)
        } else if is_selected {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let line = Paragraph::new(Line::from(vec![
            Span::styled(cursor, Style::default().fg(Color::Cyan)),
            Span::styled(format!("[{}]  ", bt.shortcut), Style::default().fg(Color::Cyan)),
            Span::styled(format!("{:<10} ", bt.name), name_style),
            Span::styled("→ ", Style::default().fg(Color::DarkGray)),
            Span::styled(base_display, base_style),
        ]));
        frame.render_widget(line, chunks[3 + i]);
    }

    // Help text
    let help_idx = chunks.len() - 1;
    let help_text = if editing_field.is_some() {
        vec![
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::raw(" save  "),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::raw(" cancel"),
        ]
    } else {
        vec![
            Span::styled("j/k", Style::default().fg(Color::Cyan)),
            Span::raw(" nav  "),
            Span::styled("b", Style::default().fg(Color::Cyan)),
            Span::raw(" edit base  "),
            Span::styled("s", Style::default().fg(Color::Cyan)),
            Span::raw(" save  "),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::raw(" back"),
        ]
    };
    let help = Paragraph::new(Line::from(help_text))
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[help_idx]);
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
