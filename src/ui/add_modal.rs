use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 30, frame.area());

    // Clear the background
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Add Worktree ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(1), // Spacing
        Constraint::Length(1), // Label
        Constraint::Length(3), // Input
        Constraint::Length(1), // Spacing
        Constraint::Length(1), // Path label
        Constraint::Length(1), // Path value
        Constraint::Min(1),    // Spacing
        Constraint::Length(1), // Help
    ])
    .split(inner);

    // Branch name label
    let label = Paragraph::new(Line::from(vec![Span::styled(
        "Branch name:",
        Style::default().fg(Color::White),
    )]));
    frame.render_widget(label, chunks[1]);

    // Input field
    let input_text = format!("{}_", app.input_buffer);
    let input = Paragraph::new(input_text)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    frame.render_widget(input, chunks[2]);

    // Generated path label
    let path_label = Paragraph::new(Line::from(vec![Span::styled(
        "Worktree path:",
        Style::default().fg(Color::DarkGray),
    )]));
    frame.render_widget(path_label, chunks[4]);

    // Generated path value (truncate from left if too long)
    let generated_path = app.generated_worktree_path();
    let path_str = generated_path.to_string_lossy().to_string();
    let max_width = chunks[5].width.saturating_sub(1) as usize;
    let display_path = if path_str.len() > max_width && max_width > 3 {
        format!("...{}", &path_str[path_str.len() - (max_width - 3)..])
    } else {
        path_str
    };
    let path_value = Paragraph::new(Line::from(vec![Span::styled(
        display_path,
        Style::default().fg(Color::Yellow),
    )]));
    frame.render_widget(path_value, chunks[5]);

    // Help text
    let help = Paragraph::new(Line::from(vec![
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::raw(" confirm  "),
        Span::styled("Esc", Style::default().fg(Color::Cyan)),
        Span::raw(" cancel"),
    ]))
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[7]);
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
