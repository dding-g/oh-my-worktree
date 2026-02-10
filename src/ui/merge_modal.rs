use ratatui::{
    layout::{Constraint, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::app::App;
use crate::types::AppState;
use super::theme::centered_rect;

pub fn render(frame: &mut Frame, app: &App) {
    let t = &app.theme;
    let (branches, selected) = match &app.state {
        AppState::MergeBranchSelect { branches, selected } => (branches, *selected),
        _ => return,
    };

    let area = centered_rect(50, 60, frame.area());

    // Clear the background
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Select Branch to Merge ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(1), // Current worktree info
        Constraint::Length(1), // Spacing
        Constraint::Min(1),    // Branch list
        Constraint::Length(1), // Help
    ])
    .split(inner);

    // Current worktree info
    if let Some(wt) = app.selected_worktree() {
        let info = Paragraph::new(Line::from(vec![
            Span::styled("Merge into: ", Style::default().fg(t.text_muted)),
            Span::styled(wt.branch_display(), Style::default().fg(t.amber)),
        ]));
        frame.render_widget(info, chunks[0]);
    }

    // Branch list
    let items: Vec<ListItem> = branches
        .iter()
        .enumerate()
        .map(|(i, branch)| {
            let style = if i == selected {
                Style::default()
                    .fg(t.selection_bg)
                    .bg(t.cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.text_primary)
            };
            ListItem::new(Line::from(Span::styled(format!("  {}", branch), style)))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, chunks[2]);

    // Help text
    let help = Paragraph::new(Line::from(vec![
        Span::styled("j/k", Style::default().fg(t.cyan)),
        Span::raw(" navigate  "),
        Span::styled("Enter", Style::default().fg(t.cyan)),
        Span::raw(" merge  "),
        Span::styled("Esc", Style::default().fg(t.cyan)),
        Span::raw(" cancel"),
    ]))
    .style(Style::default().fg(t.text_muted));
    frame.render_widget(help, chunks[3]);
}
