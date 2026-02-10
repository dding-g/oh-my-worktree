use ratatui::{
    layout::{Constraint, Layout},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::App;
use crate::types::{AppState, WorktreeStatus};
use super::theme::centered_rect;

pub fn render(frame: &mut Frame, app: &App) {
    let t = &app.theme;
    let delete_branch = match app.state {
        AppState::ConfirmDelete { delete_branch } => delete_branch,
        _ => false,
    };

    let area = centered_rect(55, 35, frame.area());

    // Clear the background
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Delete Worktree ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.red));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(1), // Spacing
        Constraint::Length(1), // Question
        Constraint::Length(1), // Spacing
        Constraint::Length(1), // Worktree name
        Constraint::Length(1), // Branch
        Constraint::Length(1), // Delete branch option
        Constraint::Length(1), // Status warning
        Constraint::Min(1),    // Spacing
        Constraint::Length(1), // Help
    ])
    .split(inner);

    // Question
    let question = Paragraph::new(Line::from(vec![Span::styled(
        "Are you sure you want to delete this worktree?",
        Style::default().fg(t.text_primary),
    )]));
    frame.render_widget(question, chunks[1]);

    if let Some(wt) = app.selected_worktree() {
        // Worktree name
        let name = Paragraph::new(Line::from(vec![
            Span::styled("Name: ", Style::default().fg(t.text_muted)),
            Span::styled(wt.display_name(), Style::default().fg(t.text_primary)),
        ]));
        frame.render_widget(name, chunks[3]);

        // Branch
        let branch = Paragraph::new(Line::from(vec![
            Span::styled("Branch: ", Style::default().fg(t.text_muted)),
            Span::styled(wt.branch_display(), Style::default().fg(t.cyan)),
        ]));
        frame.render_widget(branch, chunks[4]);

        // Delete branch option
        let checkbox = if delete_branch { "[x]" } else { "[ ]" };
        let checkbox_color = if delete_branch { t.red } else { t.text_muted };
        let delete_branch_opt = Paragraph::new(Line::from(vec![
            Span::styled(checkbox, Style::default().fg(checkbox_color)),
            Span::raw(" Also delete local branch"),
        ]));
        frame.render_widget(delete_branch_opt, chunks[5]);

        // Status warning
        if wt.status != WorktreeStatus::Clean {
            let warning = Paragraph::new(Line::from(vec![Span::styled(
                "Warning: Worktree has uncommitted changes!",
                Style::default().fg(t.amber),
            )]));
            frame.render_widget(warning, chunks[6]);
        }
    }

    // Help text
    let help = Paragraph::new(Line::from(vec![
        Span::styled("y", Style::default().fg(t.red)),
        Span::raw(" yes  "),
        Span::styled("n", Style::default().fg(t.cyan)),
        Span::raw(" no  "),
        Span::styled("b", Style::default().fg(t.amber)),
        Span::raw(" toggle branch  "),
        Span::styled("Esc", Style::default().fg(t.cyan)),
        Span::raw(" cancel"),
    ]))
    .style(Style::default().fg(t.text_muted));
    frame.render_widget(help, chunks[8]);
}
