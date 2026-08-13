use ratatui::{
    style::Style,
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::theme::centered_rect;
use crate::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let area = centered_rect(55, 30, frame.area());
    frame.render_widget(Clear, area);
    let block = Block::default()
        .title(" About owt ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.cyan));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(format!("owt v{}", env!("CARGO_PKG_VERSION"))),
            Line::from("Interactive wrapper for owt CLI worktree capabilities."),
            Line::from("Esc close"),
        ]),
        inner,
    );
}
