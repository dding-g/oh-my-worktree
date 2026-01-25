use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::app::App;
use crate::types::{SortMode, WorktreeStatus};

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // Header
        Constraint::Min(5),    // Table
        Constraint::Length(3), // Footer
    ])
    .split(frame.area());

    render_header(frame, chunks[0], app);
    render_table(frame, chunks[1], app);
    render_footer(frame, chunks[2], app);
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let header_text = vec![Line::from(vec![
        Span::styled("owt ", Style::default().fg(Color::Cyan).bold()),
        Span::styled(env!("CARGO_PKG_VERSION"), Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled(
            app.bare_repo_path.to_string_lossy().to_string(),
            Style::default().fg(Color::Yellow),
        ),
    ])];

    let header = Paragraph::new(header_text).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(header, area);
}

fn render_table(frame: &mut Frame, area: Rect, app: &App) {
    let header = Row::new(vec![
        Cell::from("  "),
        Cell::from("Name").style(Style::default().bold()),
        Cell::from("Branch").style(Style::default().bold()),
        Cell::from("Status").style(Style::default().bold()),
        Cell::from("Last Commit").style(Style::default().bold()),
    ])
    .style(Style::default().fg(Color::DarkGray))
    .height(1);

    // Check if filter matches a worktree
    let filter_lower = app.filter_text.to_lowercase();
    let has_filter = !app.filter_text.is_empty();

    let rows: Vec<Row> = app
        .worktrees
        .iter()
        .enumerate()
        .map(|(i, wt)| {
            let is_selected = i == app.selected_index;
            let is_current = app.current_worktree_path.as_ref()
                .map(|cp| cp == &wt.path)
                .unwrap_or(false);

            // Check if this row matches filter
            let matches_filter = if has_filter {
                wt.display_name().to_lowercase().contains(&filter_lower)
                    || wt.branch_display().to_lowercase().contains(&filter_lower)
            } else {
                true
            };

            // Show cursor and current indicator
            let cursor = if is_selected && is_current {
                ">●"
            } else if is_selected {
                "> "
            } else if is_current {
                " ●"
            } else {
                "  "
            };

            let status_color = match wt.status {
                WorktreeStatus::Clean => Color::Green,
                WorktreeStatus::Staged => Color::Yellow,
                WorktreeStatus::Unstaged => Color::Red,
                WorktreeStatus::Conflict => Color::Magenta,
                WorktreeStatus::Mixed => Color::Yellow,
            };

            // Build status text with ahead/behind info
            let status_base = format!("{} {}", wt.status.symbol(), wt.status.label());
            let status_text = if let Some(ref ab) = wt.ahead_behind {
                if let Some(ab_display) = ab.display() {
                    format!("{} {}", status_base, ab_display)
                } else {
                    status_base
                }
            } else {
                status_base
            };

            let row_style = if is_selected {
                Style::default().bg(Color::DarkGray)
            } else if has_filter && !matches_filter {
                // Dim non-matching rows when filtering
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };

            // Show operation status in last commit column
            let (last_commit, last_commit_style) = if app.is_fetching && is_selected {
                ("Fetching...".to_string(), Style::default().fg(Color::Yellow))
            } else if app.is_adding {
                ("Adding...".to_string(), Style::default().fg(Color::Yellow))
            } else if app.is_deleting && is_selected {
                ("Deleting...".to_string(), Style::default().fg(Color::Red))
            } else {
                (
                    wt.last_commit_time.clone().unwrap_or_else(|| "-".to_string()),
                    Style::default().fg(Color::DarkGray),
                )
            };

            let name_style = if has_filter && !matches_filter {
                Style::default().fg(Color::DarkGray)
            } else if wt.is_bare {
                Style::default().fg(Color::DarkGray).italic()
            } else if is_current {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };

            let branch_style = if has_filter && !matches_filter {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::Cyan)
            };

            let status_style = if has_filter && !matches_filter {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(status_color)
            };

            Row::new(vec![
                Cell::from(cursor).style(Style::default().fg(Color::Cyan)),
                Cell::from(wt.display_name()).style(name_style),
                Cell::from(wt.branch_display()).style(branch_style),
                Cell::from(status_text).style(status_style),
                Cell::from(last_commit).style(last_commit_style),
            ])
            .style(row_style)
        })
        .collect();

    let widths = [
        Constraint::Length(2),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(20),
        Constraint::Percentage(30),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::NONE))
        .row_highlight_style(Style::default().add_modifier(Modifier::BOLD));

    frame.render_widget(table, area);
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    // Show filter input if filtering
    if app.is_filtering {
        let filter_line = Line::from(vec![
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::styled(&app.filter_text, Style::default().fg(Color::White)),
            Span::styled("_", Style::default().fg(Color::Yellow).add_modifier(Modifier::SLOW_BLINK)),
            Span::raw("  "),
            Span::styled("(Enter to apply, Esc to cancel)", Style::default().fg(Color::DarkGray)),
        ]);

        let footer = Paragraph::new(vec![filter_line]).block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

        frame.render_widget(footer, area);
        return;
    }

    let keybindings = vec![
        ("↵", "enter"),
        ("j/k", "nav"),
        ("/", "search"),
        ("s", "sort"),
        ("a", "add"),
        ("d", "del"),
        ("?", "help"),
        ("q", "quit"),
    ];

    let mut binding_spans: Vec<Span> = keybindings
        .iter()
        .flat_map(|(key, action)| {
            vec![
                Span::styled(*key, Style::default().fg(Color::Cyan).bold()),
                Span::raw(" "),
                Span::styled(*action, Style::default().fg(Color::DarkGray)),
                Span::raw("  "),
            ]
        })
        .collect();

    // Show current sort mode if not default
    if app.sort_mode != SortMode::Name {
        binding_spans.push(Span::styled("[", Style::default().fg(Color::DarkGray)));
        binding_spans.push(Span::styled(app.sort_mode.label(), Style::default().fg(Color::Yellow)));
        binding_spans.push(Span::styled("]", Style::default().fg(Color::DarkGray)));
    }

    // Add shell integration warning if needed
    let integration_warning = if !app.has_shell_integration {
        Some(Span::styled(
            " (run 'owt setup' for Enter key navigation)",
            Style::default().fg(Color::Yellow),
        ))
    } else {
        None
    };

    let footer_content = if let Some(ref msg) = app.message {
        let msg_style = if msg.is_error {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::Green)
        };
        vec![
            Line::from(binding_spans),
            Line::from(Span::styled(&msg.text, msg_style)),
        ]
    } else if !app.filter_text.is_empty() {
        // Show active filter
        vec![
            Line::from(binding_spans),
            Line::from(vec![
                Span::styled("Filter: ", Style::default().fg(Color::DarkGray)),
                Span::styled(&app.filter_text, Style::default().fg(Color::Yellow)),
                Span::styled(" (Esc to clear)", Style::default().fg(Color::DarkGray)),
            ]),
        ]
    } else if let Some(warning) = integration_warning {
        vec![
            Line::from(binding_spans),
            Line::from(warning),
        ]
    } else {
        vec![Line::from(binding_spans)]
    };

    let footer = Paragraph::new(footer_content).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(footer, area);
}
