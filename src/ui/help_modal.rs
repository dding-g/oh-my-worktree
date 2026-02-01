use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame) {
    let area = centered_rect(50, 70, frame.area());

    // Clear the background
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Keybindings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let keybindings = vec![
        ("Navigation", vec![
            ("j / ↓", "Move down"),
            ("k / ↑", "Move up"),
            ("gg / Home", "Go to top"),
            ("G / End", "Go to bottom"),
            ("Ctrl+d/u", "Half page down/up"),
            ("g", "Jump to current worktree"),
            ("/", "Search worktrees"),
            ("Enter", "Enter worktree (cd)"),
        ]),
        ("Worktree Actions", vec![
            ("a", "Add new worktree"),
            ("d", "Delete worktree"),
            ("r", "Refresh list"),
            ("s", "Sort (name/recent/status)"),
        ]),
        ("Git Operations", vec![
            ("f", "Fetch remotes"),
            ("p", "Pull from remote"),
            ("P", "Push to remote"),
            ("m", "Merge upstream"),
            ("M", "Merge branch (select)"),
        ]),
        ("External Apps", vec![
            ("o", "Open in editor"),
            ("t", "Open in terminal"),
        ]),
        ("Other", vec![
            ("y", "Copy path to clipboard"),
            ("v", "Toggle verbose mode"),
            ("c", "View config"),
            ("?", "Show this help"),
            ("q", "Quit"),
        ]),
    ];

    let mut lines = Vec::new();
    lines.push(Line::from(""));

    for (section, bindings) in keybindings {
        // Section header
        lines.push(Line::from(Span::styled(
            format!("  {}", section),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        // Keybindings
        for (key, desc) in bindings {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(format!("{:12}", key), Style::default().fg(Color::Cyan)),
                Span::styled(desc, Style::default().fg(Color::White)),
            ]));
        }
        lines.push(Line::from(""));
    }

    // Help text
    lines.push(Line::from(vec![
        Span::raw("  Press "),
        Span::styled("Esc", Style::default().fg(Color::Cyan)),
        Span::raw(" or "),
        Span::styled("?", Style::default().fg(Color::Cyan)),
        Span::raw(" to close"),
    ]));

    let help = Paragraph::new(lines)
        .style(Style::default().fg(Color::DarkGray));

    frame.render_widget(help, inner);
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
