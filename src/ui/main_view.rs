use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

use crate::app::App;
use crate::types::WorktreeStatus;

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
        Span::styled("v0.1.0", Style::default().fg(Color::DarkGray)),
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

    let rows: Vec<Row> = app
        .worktrees
        .iter()
        .enumerate()
        .map(|(i, wt)| {
            let is_selected = i == app.selected_index;
            let cursor = if is_selected { ">" } else { " " };

            let status_color = match wt.status {
                WorktreeStatus::Clean => Color::Green,
                WorktreeStatus::Staged => Color::Yellow,
                WorktreeStatus::Unstaged => Color::Red,
                WorktreeStatus::Conflict => Color::Magenta,
                WorktreeStatus::Mixed => Color::Yellow,
            };

            let status_text = format!("{} {}", wt.status.symbol(), wt.status.label());

            let row_style = if is_selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(cursor).style(Style::default().fg(Color::Cyan)),
                Cell::from(wt.display_name()).style(if wt.is_bare {
                    Style::default().fg(Color::DarkGray).italic()
                } else {
                    Style::default().fg(Color::White)
                }),
                Cell::from(wt.branch_display()).style(Style::default().fg(Color::Cyan)),
                Cell::from(status_text).style(Style::default().fg(status_color)),
                Cell::from(
                    wt.last_commit_time
                        .clone()
                        .unwrap_or_else(|| "-".to_string()),
                )
                .style(Style::default().fg(Color::DarkGray)),
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
    let keybindings = vec![
        ("j/k", "navigate"),
        ("a", "add"),
        ("d", "delete"),
        ("o", "editor"),
        ("t", "terminal"),
        ("f", "fetch"),
        ("r", "refresh"),
        ("q", "quit"),
    ];

    let binding_spans: Vec<Span> = keybindings
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
