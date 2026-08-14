use ratatui::{
    layout::{Constraint, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::theme::centered_rect_with_min;
use crate::{app::App, types::AddModalField};

pub fn render(frame: &mut Frame, app: &App) {
    let t = &app.theme;
    // min: 9 inner rows + 2 border = 11
    let area = centered_rect_with_min(72, 42, 11, frame.area());

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
        Constraint::Length(1), // Base branch
        Constraint::Length(1), // Explicit path
        Constraint::Length(1), // tmux override
        Constraint::Min(1),    // Spacing
        Constraint::Length(1), // Help
    ])
    .split(inner);

    // Branch name label + input (inline like config_modal)
    let input_display = if app.add_modal_field == AddModalField::Branch {
        format!("[{}█]", app.input_buffer)
    } else {
        format!("[{}]", app.input_buffer)
    };
    let label_input = Paragraph::new(Line::from(vec![
        Span::styled("Branch name: ", Style::default().fg(t.text_primary)),
        Span::styled(input_display, Style::default().fg(t.amber)),
    ]));
    frame.render_widget(label_input, chunks[1]);

    // Hint for name format
    let hint = Paragraph::new(Line::from(vec![Span::styled(
        "  e.g. feature/login, hotfix/bug-123",
        Style::default()
            .fg(t.text_muted)
            .add_modifier(Modifier::ITALIC),
    )]));
    frame.render_widget(hint, chunks[2]);

    let base_branch = Paragraph::new(Line::from(vec![Span::styled(
        format!("  {}  (Tab to change)", app.add_modal_base_label()),
        Style::default().fg(t.text_muted),
    )]));
    frame.render_widget(base_branch, chunks[3]);

    let path_display = if app.add_modal_field == AddModalField::WorktreePath {
        format!("[{}█]", app.add_worktree_path)
    } else if app.add_worktree_path.is_empty() {
        "[default path]".to_string()
    } else {
        format!("[{}]", app.add_worktree_path)
    };
    let path = Paragraph::new(Line::from(vec![
        Span::styled("Path: ", Style::default().fg(t.text_primary)),
        Span::styled(path_display, Style::default().fg(t.amber)),
        Span::styled("  (Ctrl+P to edit)", Style::default().fg(t.text_muted)),
    ]));
    frame.render_widget(path, chunks[4]);

    let tmux = Paragraph::new(Line::from(vec![Span::styled(
        format!(
            "  tmux: {}  (Ctrl+T to cycle default/on/off)",
            app.add_modal_tmux_label()
        ),
        Style::default().fg(t.text_muted),
    )]));
    frame.render_widget(tmux, chunks[5]);

    // Help text
    let help = Paragraph::new(Line::from(vec![
        Span::styled("Enter", Style::default().fg(t.cyan)),
        Span::raw(" confirm  "),
        Span::styled("Tab", Style::default().fg(t.cyan)),
        Span::raw(" base  "),
        Span::styled("Ctrl+P", Style::default().fg(t.cyan)),
        Span::raw(" path  "),
        Span::styled("Ctrl+T", Style::default().fg(t.cyan)),
        Span::raw(" tmux  "),
        Span::styled("Esc", Style::default().fg(t.cyan)),
        Span::raw(" cancel"),
    ]))
    .style(Style::default().fg(t.text_muted));
    frame.render_widget(help, chunks[7]);
}
