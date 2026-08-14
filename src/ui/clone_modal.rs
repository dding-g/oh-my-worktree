use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::theme::centered_rect;
use crate::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let theme = &app.theme;
    let area = centered_rect(72, 36, frame.area());
    frame.render_widget(Clear, area);
    let block = Block::default()
        .title(" Clone Workspace ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.cyan));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let lines = vec![
        Line::from("Runs CLI-equivalent clone after terminal restore."),
        Line::from(vec![
            Span::raw("URL:  "),
            Span::styled(
                format!(
                    "[{}{}]",
                    app.clone_url,
                    if app.clone_path_editing { "" } else { "█" }
                ),
                Style::default().fg(theme.amber),
            ),
        ]),
        Line::from(vec![
            Span::raw("Path: "),
            Span::styled(
                format!(
                    "[{}{}]",
                    app.clone_path,
                    if app.clone_path_editing {
                        "█"
                    } else {
                        " (default)"
                    }
                ),
                Style::default().fg(theme.amber),
            ),
        ]),
        Line::from("Enter clone   Tab field   Esc cancel"),
    ];
    frame.render_widget(Paragraph::new(lines), inner);
}
