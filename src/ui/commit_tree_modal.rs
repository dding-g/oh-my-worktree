use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::theme::centered_rect;
use crate::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let theme = &app.theme;
    let area = centered_rect(88, 78, frame.area());
    frame.render_widget(Clear, area);
    let block = Block::default()
        .title(format!(" Commit Tree ({} commits) ", app.commit_tree_limit))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.cyan));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = vec![Line::from(Span::styled(
        "+/- change limit   r refresh   Esc close",
        Style::default().fg(theme.text_muted),
    ))];
    if app.commit_tree_receiver.is_some() {
        lines.push(Line::from(Span::styled(
            "Loading commit graph…",
            Style::default()
                .fg(theme.amber)
                .add_modifier(Modifier::ITALIC),
        )));
    } else if let Some(error) = &app.commit_tree_error {
        lines.push(Line::from(Span::styled(
            error,
            Style::default().fg(theme.red),
        )));
    } else {
        lines.extend(app.commit_tree_lines.iter().cloned().map(Line::from));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
