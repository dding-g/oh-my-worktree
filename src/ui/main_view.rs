use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{Modifier, Style, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::app::App;
use crate::types::{OpKind, ScriptStatus, SortMode, WorktreeStatus};
use crate::ui::theme::Theme;

// Spinner frames for loading animation
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let repo_path = app.bare_repo_path.to_string_lossy().to_string();
    let t = &app.theme;

    // Main container with rounded border
    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_set(border::ROUNDED)
        .border_style(Style::default().fg(t.border))
        .title(Line::from(vec![
            Span::styled(" ◆ ", Style::default().fg(t.accent)),
            Span::styled("owt ", Style::default().fg(t.text_primary).bold()),
            Span::styled(env!("CARGO_PKG_VERSION"), Style::default().fg(t.text_muted)),
            Span::raw(" "),
        ]))
        .title_bottom(Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(repo_path, Style::default().fg(t.text_muted)),
            Span::styled(" ", Style::default()),
        ]));

    frame.render_widget(main_block, area);

    let inner = area.inner(Margin::new(1, 1));

    let chunks = Layout::vertical([
        Constraint::Length(2),  // Header
        Constraint::Min(5),     // Table
        Constraint::Length(10), // Selected worktree details
        Constraint::Length(3),  // Footer
    ])
    .split(inner);

    render_header(frame, chunks[0], app);
    render_table(frame, chunks[1], app);
    render_details(frame, chunks[2], app);
    render_footer(frame, chunks[3], app);
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let worktree_count = app.worktrees.iter().filter(|w| !w.is_bare).count();

    let header_text = vec![Line::from(vec![
        Span::styled("Worktrees", Style::default().fg(t.text_primary).bold()),
        Span::raw("  "),
        Span::styled(
            format!("{} total", worktree_count),
            Style::default().fg(t.text_muted),
        ),
    ])];

    let header = Paragraph::new(header_text);
    frame.render_widget(header, area);
}

fn render_table(frame: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;

    // Store viewport height for half-page navigation (subtract 1 for header row)
    app.viewport_height.set(area.height.saturating_sub(1));

    let header = Row::new(vec![
        Cell::from(""),
        Cell::from("Name").style(Style::default().fg(t.text_muted)),
        Cell::from("Branch").style(Style::default().fg(t.text_muted)),
        Cell::from("Status").style(Style::default().fg(t.text_muted)),
        Cell::from("PR").style(Style::default().fg(t.text_muted)),
        Cell::from("Commit").style(Style::default().fg(t.text_muted)),
    ])
    .height(1);

    // Check if filter matches a worktree
    let filter_lower = app.filter_text.to_lowercase();
    let has_filter = !app.filter_text.is_empty();

    // Get current spinner frame
    let spinner = SPINNER_FRAMES[app.spinner_tick % SPINNER_FRAMES.len()];

    let rows: Vec<Row> = app
        .worktrees
        .iter()
        .enumerate()
        .map(|(i, wt)| {
            let is_selected = i == app.selected_index;
            let is_current = app
                .current_worktree_path
                .as_ref()
                .map(|cp| cp == &wt.path)
                .unwrap_or(false);

            // Check if this row matches filter
            let matches_filter = if has_filter {
                wt.display_name().to_lowercase().contains(&filter_lower)
                    || wt.branch_display().to_lowercase().contains(&filter_lower)
            } else {
                true
            };

            // Modern indicator: dot for selection, filled dot for current
            let cursor = if is_selected && is_current {
                "● "
            } else if is_selected {
                "› "
            } else if is_current {
                "◦ "
            } else {
                "  "
            };

            let cursor_color = if is_selected { t.accent } else { t.text_muted };

            let status_color = match wt.status {
                WorktreeStatus::Clean => t.accent,
                WorktreeStatus::Staged => t.amber,
                WorktreeStatus::Unstaged => t.amber,
                WorktreeStatus::Conflict => t.red,
                WorktreeStatus::Mixed => t.amber,
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

            // Dim non-matching rows during filter (selected highlight handled by StatefulWidget)
            let row_style = if has_filter && !matches_filter {
                Style::default().fg(t.text_muted)
            } else {
                Style::default()
            };

            // Show operation status in last commit column with spinner
            let is_op_target = app
                .active_op_info
                .as_ref()
                .map(|op| op.worktree_path == wt.path)
                .unwrap_or(false);

            let (last_commit, last_commit_style) = if is_op_target {
                let op = app.active_op_info.as_ref().unwrap();
                let label = match &op.kind {
                    OpKind::Fetch => "Fetching...",
                    OpKind::Pull => "Pulling...",
                    OpKind::Push => "Pushing...",
                    OpKind::Add => "Adding...",
                    OpKind::Delete => "Deleting...",
                    OpKind::Merge => "Merging...",
                };
                let color = if op.kind == OpKind::Delete {
                    t.red
                } else {
                    t.amber
                };
                (format!("{} {}", spinner, label), Style::default().fg(color))
            } else {
                (
                    wt.last_commit_time
                        .clone()
                        .unwrap_or_else(|| "-".to_string()),
                    Style::default().fg(t.text_muted),
                )
            };

            let name_style = if has_filter && !matches_filter {
                Style::default().fg(t.text_muted)
            } else if wt.is_bare {
                Style::default().fg(t.text_muted).italic()
            } else if is_current {
                Style::default().fg(t.accent)
            } else {
                Style::default().fg(t.text_primary)
            };

            let branch_style = if has_filter && !matches_filter {
                Style::default().fg(t.text_muted)
            } else {
                Style::default().fg(t.cyan)
            };

            let status_style = if has_filter && !matches_filter {
                Style::default().fg(t.text_muted)
            } else {
                Style::default().fg(status_color)
            };

            Row::new(vec![
                Cell::from(cursor).style(Style::default().fg(cursor_color)),
                Cell::from(wt.display_name()).style(name_style),
                Cell::from(wt.branch_display()).style(branch_style),
                Cell::from(status_text).style(status_style),
                Cell::from(wt.github_pr_display()).style(Style::default().fg(t.text_muted)),
                Cell::from(last_commit).style(last_commit_style),
            ])
            .style(row_style)
        })
        .collect();

    let widths = [
        Constraint::Length(2),
        Constraint::Percentage(20),
        Constraint::Percentage(26),
        Constraint::Percentage(20),
        Constraint::Length(8),
        Constraint::Percentage(26),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::NONE))
        .row_highlight_style(
            Style::default()
                .bg(t.accent_dim)
                .add_modifier(Modifier::BOLD),
        );

    // Use StatefulWidget so ratatui handles scroll offset automatically
    let selected = Some(app.selected_index);
    let mut table_state = TableState::new().with_selected(selected);
    frame.render_stateful_widget(table, area, &mut table_state);
}

fn render_details(frame: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let mut lines = Vec::new();

    if let Some(wt) = app.selected_worktree() {
        if wt.is_bare {
            lines.push(Line::from(vec![Span::styled(
                "Bare repository",
                Style::default().fg(t.text_muted),
            )]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("Name ", Style::default().fg(t.text_muted)),
                Span::styled(
                    wt.display_name(),
                    Style::default().fg(t.text_primary).bold(),
                ),
                Span::styled("  Branch ", Style::default().fg(t.text_muted)),
                Span::styled(wt.branch_display(), Style::default().fg(t.cyan)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("Status ", Style::default().fg(t.text_muted).bold()),
                Span::styled(
                    app.selected_details
                        .as_ref()
                        .map(|details| details.status_summary.as_str())
                        .unwrap_or("unavailable"),
                    Style::default().fg(t.text_primary).bold(),
                ),
            ]));
            lines.push(Line::from(vec![Span::styled(
                "Recent commits",
                Style::default().fg(t.text_muted),
            )]));

            if let Some(details) = app.selected_details.as_ref() {
                for commit in details.recent_commits.iter().take(6) {
                    lines.push(render_commit_line(commit, t));
                }
            }
        }
    }

    let details = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::TOP)
            .border_set(border::ROUNDED)
            .border_style(Style::default().fg(t.border))
            .title(" Details "),
    );
    frame.render_widget(details, area);
}

fn render_commit_line(commit: &str, t: &Theme) -> Line<'static> {
    let Some((graph, hash, rest)) = split_commit_line(commit) else {
        return Line::from(Span::styled(
            commit.to_string(),
            Style::default().fg(t.text_primary),
        ));
    };

    let Some((date, rest)) = split_commit_date(rest) else {
        return Line::from(Span::styled(
            commit.to_string(),
            Style::default().fg(t.text_primary),
        ));
    };

    let (decoration, message) = split_decoration(rest);
    let mut spans = vec![
        Span::styled(graph, Style::default().fg(t.text_muted)),
        Span::styled(hash, Style::default().fg(t.amber).bold()),
        Span::raw(" "),
        Span::styled(date, Style::default().fg(t.text_muted)),
    ];

    if let Some(decoration) = decoration {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(decoration, Style::default().fg(t.cyan)));
    }

    if !message.is_empty() {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(message, Style::default().fg(t.text_primary)));
    }

    Line::from(spans)
}

fn split_commit_line(commit: &str) -> Option<(String, String, String)> {
    let mut search_from = 0;

    for token in commit.split_whitespace() {
        let token_start = search_from + commit[search_from..].find(token)?;
        let token_end = token_start + token.len();

        if is_commit_hash(token) {
            return Some((
                commit[..token_start].to_string(),
                token.to_string(),
                commit[token_end..].trim_start().to_string(),
            ));
        }

        search_from = token_end;
    }

    None
}

fn is_commit_hash(token: &str) -> bool {
    (4..=40).contains(&token.len()) && token.chars().all(|c| c.is_ascii_hexdigit())
}

fn split_commit_date(rest: String) -> Option<(String, String)> {
    let (date, rest) = rest.split_once(' ')?;
    Some((date.to_string(), rest.trim_start().to_string()))
}

fn split_decoration(rest: String) -> (Option<String>, String) {
    if !rest.starts_with('(') {
        return (None, rest);
    }

    let Some(end) = rest.find(')') else {
        return (None, rest);
    };

    let decoration = rest[..=end].to_string();
    let message = rest[end + 1..].trim_start().to_string();
    (Some(decoration), message)
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;

    // Show filter input if filtering
    if app.is_filtering {
        let filter_line = Line::from(vec![
            Span::styled("/", Style::default().fg(t.accent)),
            Span::styled(&app.filter_text, Style::default().fg(t.text_primary)),
            Span::styled(
                "▋",
                Style::default()
                    .fg(t.accent)
                    .add_modifier(Modifier::SLOW_BLINK),
            ),
            Span::raw("  "),
            Span::styled(
                "Enter to apply · Esc to cancel",
                Style::default().fg(t.text_muted),
            ),
        ]);

        let footer = Paragraph::new(vec![filter_line]).block(
            Block::default()
                .borders(Borders::TOP)
                .border_set(border::ROUNDED)
                .border_style(Style::default().fg(t.border)),
        );

        frame.render_widget(footer, area);
        return;
    }

    let keybindings = vec![
        ("↵", "enter"),
        ("j/k", "nav"),
        ("a", "add"),
        ("d", "del"),
        ("f", "fetch"),
        ("p", "pull"),
        ("/", "search"),
        ("?", "help"),
    ];

    let mut binding_spans: Vec<Span> = keybindings
        .iter()
        .flat_map(|(key, action)| {
            vec![
                Span::styled(*key, Style::default().fg(t.accent).bold()),
                Span::styled(format!(" {} ", action), Style::default().fg(t.text_muted)),
            ]
        })
        .collect();

    // Show current sort mode if not default
    if app.sort_mode != SortMode::Name {
        binding_spans.push(Span::styled("│ ", Style::default().fg(t.border)));
        binding_spans.push(Span::styled(
            app.sort_mode.label(),
            Style::default().fg(t.amber),
        ));
    }

    // Add shell integration warning if needed
    let integration_warning = if !app.has_shell_integration {
        Some(Span::styled(
            " │ run 'owt setup' for shell integration",
            Style::default().fg(t.amber),
        ))
    } else {
        None
    };

    let footer_content = if let Some(ref op) = app.active_op_info {
        let spinner = SPINNER_FRAMES[app.spinner_tick % SPINNER_FRAMES.len()];
        let label = match &op.kind {
            OpKind::Fetch => "Fetching",
            OpKind::Pull => "Pulling",
            OpKind::Push => "Pushing",
            OpKind::Add => "Creating",
            OpKind::Delete => "Deleting",
            OpKind::Merge => "Merging",
        };
        vec![
            Line::from(binding_spans),
            Line::from(vec![
                Span::styled(spinner, Style::default().fg(t.amber)),
                Span::styled(
                    format!(" {} {}...", label, op.display_name),
                    Style::default().fg(t.amber),
                ),
            ]),
        ]
    } else if let ScriptStatus::Running { ref worktree_name } = app.script_status {
        let spinner = SPINNER_FRAMES[app.spinner_tick % SPINNER_FRAMES.len()];
        let script_line = if let Some(ref msg) = app.message {
            // Show both message and script status
            Line::from(vec![
                Span::styled(&msg.text, Style::default().fg(t.accent)),
                Span::styled(
                    format!("  {} running setup...", spinner),
                    Style::default().fg(t.amber),
                ),
            ])
        } else {
            Line::from(vec![
                Span::styled(spinner, Style::default().fg(t.amber)),
                Span::styled(
                    format!(" Running setup script for {}...", worktree_name),
                    Style::default().fg(t.amber),
                ),
            ])
        };
        vec![Line::from(binding_spans), script_line]
    } else if let Some(ref msg) = app.message {
        let msg_style = if msg.is_error {
            Style::default().fg(t.red)
        } else {
            Style::default().fg(t.accent)
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
                Span::styled("Filter: ", Style::default().fg(t.text_muted)),
                Span::styled(&app.filter_text, Style::default().fg(t.amber)),
                Span::styled(" (Esc to clear)", Style::default().fg(t.text_muted)),
            ]),
        ]
    } else if let Some(ref op) = app.active_op_info {
        let spinner = SPINNER_FRAMES[app.spinner_tick % SPINNER_FRAMES.len()];
        let label = match &op.kind {
            OpKind::Fetch => "Fetching",
            OpKind::Pull => "Pulling",
            OpKind::Push => "Pushing",
            OpKind::Add => "Adding",
            OpKind::Delete => "Deleting",
            OpKind::Merge => "Merging",
        };
        vec![
            Line::from(binding_spans),
            Line::from(vec![
                Span::styled(spinner, Style::default().fg(t.amber)),
                Span::styled(
                    format!(" {} {}...", label, op.display_name),
                    Style::default().fg(t.amber),
                ),
            ]),
        ]
    } else if let Some(warning) = integration_warning {
        vec![Line::from(binding_spans), Line::from(warning)]
    } else {
        vec![Line::from(binding_spans)]
    };

    let footer = Paragraph::new(footer_content).block(
        Block::default()
            .borders(Borders::TOP)
            .border_set(border::ROUNDED)
            .border_style(Style::default().fg(t.border)),
    );

    frame.render_widget(footer, area);
}
