use ratatui::{
    layout::{Constraint, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::theme::centered_rect;
use crate::app::App;
use crate::worktree_prune::PruneWorktreeAction;

pub fn render(frame: &mut Frame, app: &App) {
    let t = &app.theme;
    let area = centered_rect(78, 62, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Cleanup Preview ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.amber));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .split(inner);

    let Some(report) = app.cleanup_preview.as_ref() else {
        return;
    };

    let candidate_count = report.candidate_count();
    let title = if candidate_count == 0 {
        "No completed-PR worktrees are eligible for cleanup.".to_string()
    } else {
        format!(
            "{} completed-PR worktree(s) are eligible. Live status is checked again before removal.",
            candidate_count
        )
    };
    frame.render_widget(
        Paragraph::new(Span::styled(title, Style::default().fg(t.text_primary))),
        chunks[0],
    );

    let mut lines = Vec::new();
    for log in report.logs.iter().take(12) {
        let branch = log.branch.as_deref().unwrap_or("-");
        let (label, color) = match &log.action {
            PruneWorktreeAction::WouldRemove => ("REMOVE", t.red),
            PruneWorktreeAction::Removed => ("REMOVED", t.accent),
            PruneWorktreeAction::Kept(reason) => (reason.as_str(), t.text_muted),
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{:<14}", label), Style::default().fg(color)),
            Span::styled(branch.to_string(), Style::default().fg(t.cyan)),
            Span::raw("  "),
            Span::styled(
                log.path.display().to_string(),
                Style::default().fg(t.text_muted),
            ),
        ]));
    }
    if report.logs.len() > 12 {
        lines.push(Line::from(Span::styled(
            format!("… {} more worktree(s)", report.logs.len() - 12),
            Style::default()
                .fg(t.text_muted)
                .add_modifier(Modifier::ITALIC),
        )));
    }
    frame.render_widget(Paragraph::new(lines), chunks[2]);

    let help = if candidate_count == 0 {
        Line::from(vec![
            Span::styled("Esc", Style::default().fg(t.cyan)),
            Span::raw(" close"),
        ])
    } else {
        Line::from(vec![
            Span::styled("Enter/y", Style::default().fg(t.red)),
            Span::raw(" remove eligible worktrees  "),
            Span::styled("Esc/n", Style::default().fg(t.cyan)),
            Span::raw(" cancel"),
        ])
    };
    frame.render_widget(
        Paragraph::new(help).style(Style::default().fg(t.text_muted)),
        chunks[3],
    );
}
