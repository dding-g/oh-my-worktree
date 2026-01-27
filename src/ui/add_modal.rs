use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::App;
use crate::git;
use crate::types::BaseSource;

/// Render the original simple add modal (for backwards compatibility)
pub fn render(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 28, frame.area());

    // Clear the background
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Add Worktree ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(1), // Spacing
        Constraint::Length(1), // Label + Input
        Constraint::Length(1), // Hint
        Constraint::Min(1),    // Spacing
        Constraint::Length(1), // Help
    ])
    .split(inner);

    // Branch name label + input (inline like config_modal)
    let input_display = format!("[{}█]", app.input_buffer);
    let label_input = Paragraph::new(Line::from(vec![
        Span::styled("Branch name: ", Style::default().fg(Color::White)),
        Span::styled(input_display, Style::default().fg(Color::Yellow)),
    ]));
    frame.render_widget(label_input, chunks[1]);

    // Hint for name format
    let hint = Paragraph::new(Line::from(vec![Span::styled(
        "  e.g. TASK-123-feature-description",
        Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
    )]));
    frame.render_widget(hint, chunks[2]);

    // Help text
    let help = Paragraph::new(Line::from(vec![
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::raw(" confirm  "),
        Span::styled("Esc", Style::default().fg(Color::Cyan)),
        Span::raw(" cancel"),
    ]))
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[4]);
}

/// Render the branch type selection screen
pub fn render_type_select(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 50, frame.area());

    // Clear the background
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Add Worktree ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Calculate dynamic layout based on number of branch types
    let bt_count = app.config.branch_types.len();
    let mut constraints = vec![
        Constraint::Length(1), // Spacing
        Constraint::Length(1), // Title
        Constraint::Length(1), // Spacing
    ];

    // Add constraints for each branch type
    for _ in 0..bt_count {
        constraints.push(Constraint::Length(1));
    }

    constraints.extend([
        Constraint::Length(1), // Separator
        Constraint::Length(1), // Custom option
        Constraint::Min(1),    // Flexible spacing
        Constraint::Length(1), // Help
    ]);

    let chunks = Layout::vertical(constraints).split(inner);

    // Title
    let title = Paragraph::new(Line::from(vec![
        Span::styled("Select branch type:", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
    ]));
    frame.render_widget(title, chunks[1]);

    // Branch types
    for (i, bt) in app.config.branch_types.iter().enumerate() {
        let line = Paragraph::new(Line::from(vec![
            Span::styled("  [", Style::default().fg(Color::DarkGray)),
            Span::styled(bt.shortcut.to_string(), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled("] ", Style::default().fg(Color::DarkGray)),
            Span::styled(&bt.name, Style::default().fg(Color::White)),
            Span::styled(format!("  → {}", bt.base), Style::default().fg(Color::DarkGray)),
        ]));
        frame.render_widget(line, chunks[3 + i]);
    }

    // Separator
    let sep_idx = 3 + bt_count;
    let separator = Paragraph::new(Line::from(vec![
        Span::styled("  ─────────────────────────", Style::default().fg(Color::DarkGray)),
    ]));
    frame.render_widget(separator, chunks[sep_idx]);

    // Custom option
    let custom = Paragraph::new(Line::from(vec![
        Span::styled("  [", Style::default().fg(Color::DarkGray)),
        Span::styled("c", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("] ", Style::default().fg(Color::DarkGray)),
        Span::styled("custom", Style::default().fg(Color::White)),
        Span::styled("  (select base manually)", Style::default().fg(Color::DarkGray)),
    ]));
    frame.render_widget(custom, chunks[sep_idx + 1]);

    // Help text
    let help_idx = chunks.len() - 1;
    let help = Paragraph::new(Line::from(vec![
        Span::styled("Type shortcut", Style::default().fg(Color::Cyan)),
        Span::raw(" to select  "),
        Span::styled("Esc", Style::default().fg(Color::Cyan)),
        Span::raw(" cancel"),
    ]))
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[help_idx]);
}

/// Render the branch name input screen with base branch comparison
pub fn render_branch_input(frame: &mut Frame, app: &App) {
    let area = centered_rect(70, 60, frame.area());

    // Clear the background
    frame.render_widget(Clear, area);

    // Determine title based on branch type
    let title = if let Some(ref bt) = app.add_worktree_state.branch_type {
        format!(" Add {} worktree ", bt.name)
    } else {
        " Add worktree (custom) ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(1), // Spacing
        Constraint::Length(1), // Name label
        Constraint::Length(1), // Name input
        Constraint::Length(1), // Spacing
        Constraint::Length(1), // Separator
        Constraint::Length(1), // Base label
        Constraint::Length(1), // Spacing
        Constraint::Length(1), // Local info
        Constraint::Length(1), // Remote info
        Constraint::Length(1), // Behind count
        Constraint::Length(1), // Spacing
        Constraint::Length(1), // Actions
        Constraint::Length(1), // Separator
        Constraint::Length(1), // Spacing
        Constraint::Length(1), // Will create from
        Constraint::Min(1),    // Flexible spacing
        Constraint::Length(1), // Help
    ])
    .split(inner);

    // Name label
    let name_label = Paragraph::new(Line::from(vec![
        Span::styled("Name:", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
    ]));
    frame.render_widget(name_label, chunks[1]);

    // Name input with prefix
    let input_display = format!("[{}█]", app.input_buffer);
    let name_input = Paragraph::new(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(input_display, Style::default().fg(Color::Yellow)),
    ]));
    frame.render_widget(name_input, chunks[2]);

    // Separator
    let separator = Line::from(vec![
        Span::styled("───────────────────────────────────────────────────────", Style::default().fg(Color::DarkGray)),
    ]);
    frame.render_widget(Paragraph::new(separator.clone()), chunks[4]);

    // Base label
    let base_label = Paragraph::new(Line::from(vec![
        Span::styled("Base: ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(&app.add_worktree_state.base_branch, Style::default().fg(Color::Cyan)),
    ]));
    frame.render_widget(base_label, chunks[5]);

    // Get branch comparison info
    let comparison = git::compare_local_remote(&app.bare_repo_path, &app.add_worktree_state.base_branch)
        .unwrap_or_default();

    // Local info
    let local_info = if let Some(ref info) = comparison.local {
        Line::from(vec![
            Span::styled("  local   ", Style::default().fg(if app.add_worktree_state.base_source == BaseSource::Local { Color::Green } else { Color::DarkGray })),
            Span::styled(&info.hash, Style::default().fg(Color::Yellow)),
            Span::styled(format!("  \"{}\"", truncate_str(&info.message, 30)), Style::default().fg(Color::White)),
            Span::styled(format!(" ({})", info.time_ago), Style::default().fg(Color::DarkGray)),
        ])
    } else {
        Line::from(vec![
            Span::styled("  local   ", Style::default().fg(Color::DarkGray)),
            Span::styled("(not found)", Style::default().fg(Color::DarkGray)),
        ])
    };
    frame.render_widget(Paragraph::new(local_info), chunks[7]);

    // Remote info
    let remote_info = if let Some(ref info) = comparison.remote {
        Line::from(vec![
            Span::styled("  remote  ", Style::default().fg(if app.add_worktree_state.base_source == BaseSource::Remote { Color::Green } else { Color::DarkGray })),
            Span::styled(&info.hash, Style::default().fg(Color::Yellow)),
            Span::styled(format!("  \"{}\"", truncate_str(&info.message, 30)), Style::default().fg(Color::White)),
            Span::styled(format!(" ({})", info.time_ago), Style::default().fg(Color::DarkGray)),
        ])
    } else {
        Line::from(vec![
            Span::styled("  remote  ", Style::default().fg(Color::DarkGray)),
            Span::styled("(not fetched)", Style::default().fg(Color::DarkGray)),
        ])
    };
    frame.render_widget(Paragraph::new(remote_info), chunks[8]);

    // Behind count
    let behind_info = if comparison.behind_count > 0 {
        Line::from(vec![
            Span::styled(format!("          ↓{} commits behind", comparison.behind_count), Style::default().fg(Color::Yellow)),
        ])
    } else if comparison.local.is_some() && comparison.remote.is_some() {
        Line::from(vec![
            Span::styled("          ✓ up to date", Style::default().fg(Color::Green)),
        ])
    } else {
        Line::from(vec![])
    };
    frame.render_widget(Paragraph::new(behind_info), chunks[9]);

    // Actions
    let actions = Paragraph::new(Line::from(vec![
        Span::styled("  [", Style::default().fg(Color::DarkGray)),
        Span::styled("F", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("] Fetch  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[", Style::default().fg(Color::DarkGray)),
        Span::styled("U", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("] Use remote  ", Style::default().fg(Color::DarkGray)),
        Span::styled("[", Style::default().fg(Color::DarkGray)),
        Span::styled("L", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("] Use local", Style::default().fg(Color::DarkGray)),
    ]));
    frame.render_widget(actions, chunks[11]);

    // Separator
    frame.render_widget(Paragraph::new(separator.clone()), chunks[12]);

    // Will create from
    let source = match app.add_worktree_state.base_source {
        BaseSource::Local => format!("local/{}", app.add_worktree_state.base_branch),
        BaseSource::Remote => format!("origin/{}", app.add_worktree_state.base_branch),
    };
    let hash = match app.add_worktree_state.base_source {
        BaseSource::Local => comparison.local.as_ref().map(|i| i.hash.clone()),
        BaseSource::Remote => comparison.remote.as_ref().map(|i| i.hash.clone()),
    };
    let create_from = Paragraph::new(Line::from(vec![
        Span::styled("Will create from: ", Style::default().fg(Color::White)),
        Span::styled(&source, Style::default().fg(Color::Cyan)),
        if let Some(h) = hash {
            Span::styled(format!(" ({})", h), Style::default().fg(Color::DarkGray))
        } else {
            Span::styled("", Style::default())
        },
    ]));
    frame.render_widget(create_from, chunks[14]);

    // Help text
    let help = Paragraph::new(Line::from(vec![
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::raw(" create  "),
        Span::styled("Esc", Style::default().fg(Color::Cyan)),
        Span::raw(" back"),
    ]))
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[16]);
}

/// Truncate a string to max length, adding "..." if truncated
fn truncate_str(s: &str, max_len: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
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
