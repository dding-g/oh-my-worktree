use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::app::App;
use crate::types::{SortMode, WorktreeStatus};

// Modern color palette inspired by the landing page
const ACCENT: Color = Color::Rgb(16, 185, 129);      // Emerald green
const ACCENT_DIM: Color = Color::Rgb(6, 95, 70);     // Darker emerald
const AMBER: Color = Color::Rgb(245, 158, 11);       // Amber/yellow
const RED: Color = Color::Rgb(239, 68, 68);          // Red
const CYAN: Color = Color::Rgb(34, 211, 238);        // Cyan
const TEXT_PRIMARY: Color = Color::Rgb(250, 250, 250);
const TEXT_SECONDARY: Color = Color::Rgb(161, 161, 170);
const TEXT_MUTED: Color = Color::Rgb(113, 113, 122);
const BG_ELEVATED: Color = Color::Rgb(39, 39, 42);
const BORDER: Color = Color::Rgb(63, 63, 70);

// Spinner frames for loading animation
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let repo_path = app.bare_repo_path.to_string_lossy().to_string();

    // Main container with rounded border
    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_set(border::ROUNDED)
        .border_style(Style::default().fg(BORDER))
        .title(Line::from(vec![
            Span::styled(" ◆ ", Style::default().fg(ACCENT)),
            Span::styled("owt ", Style::default().fg(TEXT_PRIMARY).bold()),
            Span::styled(env!("CARGO_PKG_VERSION"), Style::default().fg(TEXT_MUTED)),
            Span::raw(" "),
        ]))
        .title_bottom(Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(repo_path, Style::default().fg(TEXT_MUTED)),
            Span::styled(" ", Style::default()),
        ]));

    frame.render_widget(main_block, area);

    let inner = area.inner(Margin::new(1, 1));

    let chunks = Layout::vertical([
        Constraint::Length(2), // Header
        Constraint::Min(5),    // Table
        Constraint::Length(2), // Footer
    ])
    .split(inner);

    render_header(frame, chunks[0], app);
    render_table(frame, chunks[1], app);
    render_footer(frame, chunks[2], app);
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let worktree_count = app.worktrees.iter().filter(|w| !w.is_bare).count();

    let header_text = vec![Line::from(vec![
        Span::styled("Worktrees", Style::default().fg(TEXT_PRIMARY).bold()),
        Span::raw("  "),
        Span::styled(
            format!("{} total", worktree_count),
            Style::default().fg(TEXT_MUTED),
        ),
    ])];

    let header = Paragraph::new(header_text);
    frame.render_widget(header, area);
}

fn render_table(frame: &mut Frame, area: Rect, app: &App) {
    let header = Row::new(vec![
        Cell::from(""),
        Cell::from("Name").style(Style::default().fg(TEXT_MUTED)),
        Cell::from("Branch").style(Style::default().fg(TEXT_MUTED)),
        Cell::from("Status").style(Style::default().fg(TEXT_MUTED)),
        Cell::from("Commit").style(Style::default().fg(TEXT_MUTED)),
    ])
    .height(1);

    // Check if filter matches a worktree
    let filter_lower = app.filter_text.to_lowercase();
    let has_filter = !app.filter_text.is_empty();

    // Check if any loading operation is in progress
    let is_loading = app.is_adding || app.is_deleting || app.is_fetching
        || app.is_pulling || app.is_pushing || app.is_merging;

    // Get current spinner frame
    let spinner = SPINNER_FRAMES[app.spinner_tick % SPINNER_FRAMES.len()];

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

            // Modern indicator: dot for selection, filled dot for current
            // Hide selection cursor during loading
            let cursor = if is_loading {
                if is_current {
                    "◦ "
                } else {
                    "  "
                }
            } else if is_selected && is_current {
                "● "
            } else if is_selected {
                "› "
            } else if is_current {
                "◦ "
            } else {
                "  "
            };

            let cursor_color = if is_loading {
                TEXT_MUTED
            } else if is_selected {
                ACCENT
            } else {
                TEXT_MUTED
            };

            let status_color = match wt.status {
                WorktreeStatus::Clean => ACCENT,
                WorktreeStatus::Staged => AMBER,
                WorktreeStatus::Unstaged => AMBER,
                WorktreeStatus::Conflict => RED,
                WorktreeStatus::Mixed => AMBER,
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

            // Hide highlight during loading operations
            let row_style = if is_loading {
                // No highlight during loading
                if has_filter && !matches_filter {
                    Style::default().fg(TEXT_MUTED)
                } else {
                    Style::default()
                }
            } else if is_selected {
                Style::default().bg(ACCENT_DIM)
            } else if has_filter && !matches_filter {
                Style::default().fg(TEXT_MUTED)
            } else {
                Style::default()
            };

            // Show operation status in last commit column with spinner
            let (last_commit, last_commit_style) = if app.is_fetching && is_selected {
                (format!("{} Fetching...", spinner), Style::default().fg(AMBER))
            } else if app.is_adding && is_selected {
                (format!("{} Adding...", spinner), Style::default().fg(AMBER))
            } else if app.is_deleting && is_selected {
                (format!("{} Deleting...", spinner), Style::default().fg(RED))
            } else if app.is_pulling && is_selected {
                (format!("{} Pulling...", spinner), Style::default().fg(AMBER))
            } else if app.is_pushing && is_selected {
                (format!("{} Pushing...", spinner), Style::default().fg(AMBER))
            } else if app.is_merging && is_selected {
                (format!("{} Merging...", spinner), Style::default().fg(AMBER))
            } else {
                (
                    wt.last_commit_time.clone().unwrap_or_else(|| "-".to_string()),
                    Style::default().fg(TEXT_MUTED),
                )
            };

            let name_style = if has_filter && !matches_filter {
                Style::default().fg(TEXT_MUTED)
            } else if wt.is_bare {
                Style::default().fg(TEXT_MUTED).italic()
            } else if is_current {
                Style::default().fg(ACCENT)
            } else {
                Style::default().fg(TEXT_PRIMARY)
            };

            let branch_style = if has_filter && !matches_filter {
                Style::default().fg(TEXT_MUTED)
            } else {
                Style::default().fg(CYAN)
            };

            let status_style = if has_filter && !matches_filter {
                Style::default().fg(TEXT_MUTED)
            } else {
                Style::default().fg(status_color)
            };

            Row::new(vec![
                Cell::from(cursor).style(Style::default().fg(cursor_color)),
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
        Constraint::Percentage(22),
        Constraint::Percentage(28),
        Constraint::Percentage(22),
        Constraint::Percentage(28),
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
            Span::styled("/", Style::default().fg(ACCENT)),
            Span::styled(&app.filter_text, Style::default().fg(TEXT_PRIMARY)),
            Span::styled("▋", Style::default().fg(ACCENT).add_modifier(Modifier::SLOW_BLINK)),
            Span::raw("  "),
            Span::styled("Enter to apply · Esc to cancel", Style::default().fg(TEXT_MUTED)),
        ]);

        let footer = Paragraph::new(vec![filter_line]).block(
            Block::default()
                .borders(Borders::TOP)
                .border_set(border::ROUNDED)
                .border_style(Style::default().fg(BORDER)),
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
                Span::styled(*key, Style::default().fg(ACCENT).bold()),
                Span::styled(format!(" {} ", action), Style::default().fg(TEXT_MUTED)),
            ]
        })
        .collect();

    // Show current sort mode if not default
    if app.sort_mode != SortMode::Name {
        binding_spans.push(Span::styled("│ ", Style::default().fg(BORDER)));
        binding_spans.push(Span::styled(app.sort_mode.label(), Style::default().fg(AMBER)));
    }

    // Add shell integration warning if needed
    let integration_warning = if !app.has_shell_integration {
        Some(Span::styled(
            " │ run 'owt setup' for shell integration",
            Style::default().fg(AMBER),
        ))
    } else {
        None
    };

    let footer_content = if let Some(ref msg) = app.message {
        let msg_style = if msg.is_error {
            Style::default().fg(RED)
        } else {
            Style::default().fg(ACCENT)
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
                Span::styled("Filter: ", Style::default().fg(TEXT_MUTED)),
                Span::styled(&app.filter_text, Style::default().fg(AMBER)),
                Span::styled(" (Esc to clear)", Style::default().fg(TEXT_MUTED)),
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
            .border_set(border::ROUNDED)
            .border_style(Style::default().fg(BORDER)),
    );

    frame.render_widget(footer, area);
}
