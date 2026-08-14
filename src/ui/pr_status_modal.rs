use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::theme::centered_rect;
use crate::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let theme = &app.theme;
    let area = centered_rect(72, 52, frame.area());
    frame.render_widget(Clear, area);
    let block = Block::default()
        .title(" PR Status ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.cyan));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = vec![Line::from(Span::styled(
        "s/Enter selected   a all worktrees   b arbitrary branch   Esc close",
        Style::default().fg(theme.text_muted),
    ))];
    if app.pr_query_editing {
        lines.push(Line::from(vec![
            Span::styled("Branch: ", Style::default().fg(theme.text_primary)),
            Span::styled(
                format!("[{}█]", app.pr_query_input),
                Style::default().fg(theme.amber),
            ),
        ]));
    } else if app.pr_query_receiver.is_some() {
        lines.push(Line::from(Span::styled(
            "Checking GitHub PR status…",
            Style::default()
                .fg(theme.amber)
                .add_modifier(Modifier::ITALIC),
        )));
    } else if app.pr_query_results.is_empty() {
        lines.push(Line::from(Span::styled(
            "No query result. Select a worktree or choose all/arbitrary branch.",
            Style::default().fg(theme.text_muted),
        )));
    } else {
        for (branch, path, status) in &app.pr_query_results {
            lines.push(Line::from(vec![
                Span::styled(format!("{:<24}", branch), Style::default().fg(theme.cyan)),
                Span::styled(
                    format!("{:<8}", status.map(|value| value.label()).unwrap_or("-")),
                    Style::default().fg(theme.text_primary),
                ),
                Span::styled(
                    path.display().to_string(),
                    Style::default().fg(theme.text_muted),
                ),
            ]));
        }
    }
    frame.render_widget(Paragraph::new(lines), inner);
}
