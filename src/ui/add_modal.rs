use ratatui::{
    layout::{Constraint, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::App;
use super::theme::centered_rect;

pub fn render(frame: &mut Frame, app: &App) {
    let t = &app.theme;
    let area = centered_rect(60, 28, frame.area());

    // Clear the background
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Add Worktree ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.cyan));

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
        Span::styled("Branch name: ", Style::default().fg(t.text_primary)),
        Span::styled(input_display, Style::default().fg(t.amber)),
    ]));
    frame.render_widget(label_input, chunks[1]);

    // Hint for name format
    let hint = Paragraph::new(Line::from(vec![Span::styled(
        "  e.g. feature/login, hotfix/bug-123",
        Style::default().fg(t.text_muted).add_modifier(Modifier::ITALIC),
    )]));
    frame.render_widget(hint, chunks[2]);

    // Help text
    let help = Paragraph::new(Line::from(vec![
        Span::styled("Enter", Style::default().fg(t.cyan)),
        Span::raw(" confirm  "),
        Span::styled("Esc", Style::default().fg(t.cyan)),
        Span::raw(" cancel"),
    ]))
    .style(Style::default().fg(t.text_muted));
    frame.render_widget(help, chunks[4]);
}
