use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::app::App;
use crate::types::AppState;

pub fn render(frame: &mut Frame, app: &App) {
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
        .border_style(Style::default().fg(Color::Cyan));

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
            Span::styled("Merge into: ", Style::default().fg(Color::DarkGray)),
            Span::styled(wt.branch_display(), Style::default().fg(Color::Yellow)),
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
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(Span::styled(format!("  {}", branch), style)))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, chunks[2]);

    // Help text
    let help = Paragraph::new(Line::from(vec![
        Span::styled("j/k", Style::default().fg(Color::Cyan)),
        Span::raw(" navigate  "),
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::raw(" merge  "),
        Span::styled("Esc", Style::default().fg(Color::Cyan)),
        Span::raw(" cancel"),
    ]))
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[3]);
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
