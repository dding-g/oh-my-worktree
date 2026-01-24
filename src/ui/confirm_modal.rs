use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::App;
use crate::types::{AppState, WorktreeStatus};

pub fn render(frame: &mut Frame, app: &App) {
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
        .border_style(Style::default().fg(Color::Red));

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
        Style::default().fg(Color::White),
    )]));
    frame.render_widget(question, chunks[1]);

    if let Some(wt) = app.selected_worktree() {
        // Worktree name
        let name = Paragraph::new(Line::from(vec![
            Span::styled("Name: ", Style::default().fg(Color::DarkGray)),
            Span::styled(wt.display_name(), Style::default().fg(Color::White)),
        ]));
        frame.render_widget(name, chunks[3]);

        // Branch
        let branch = Paragraph::new(Line::from(vec![
            Span::styled("Branch: ", Style::default().fg(Color::DarkGray)),
            Span::styled(wt.branch_display(), Style::default().fg(Color::Cyan)),
        ]));
        frame.render_widget(branch, chunks[4]);

        // Delete branch option
        let checkbox = if delete_branch { "[x]" } else { "[ ]" };
        let checkbox_color = if delete_branch { Color::Red } else { Color::DarkGray };
        let delete_branch_opt = Paragraph::new(Line::from(vec![
            Span::styled(checkbox, Style::default().fg(checkbox_color)),
            Span::raw(" Also delete local branch"),
        ]));
        frame.render_widget(delete_branch_opt, chunks[5]);

        // Status warning
        if wt.status != WorktreeStatus::Clean {
            let warning = Paragraph::new(Line::from(vec![Span::styled(
                "Warning: Worktree has uncommitted changes!",
                Style::default().fg(Color::Yellow),
            )]));
            frame.render_widget(warning, chunks[6]);
        }
    }

    // Help text
    let help = Paragraph::new(Line::from(vec![
        Span::styled("y", Style::default().fg(Color::Red)),
        Span::raw(" yes  "),
        Span::styled("n", Style::default().fg(Color::Cyan)),
        Span::raw(" no  "),
        Span::styled("b", Style::default().fg(Color::Yellow)),
        Span::raw(" toggle branch  "),
        Span::styled("Esc", Style::default().fg(Color::Cyan)),
        Span::raw(" cancel"),
    ]))
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[8]);
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
