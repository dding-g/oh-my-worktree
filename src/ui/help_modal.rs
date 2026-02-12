use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::App;
use super::theme::centered_rect;

pub fn render(frame: &mut Frame, app: &App) {
    let t = &app.theme;
    let area = centered_rect(50, 70, frame.area());

    // Clear the background
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Keybindings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.cyan));

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
            ("d", "Delete worktree (f: force)"),
            ("x", "Prune stale worktrees"),
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
            Style::default().fg(t.amber).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        // Keybindings
        for (key, desc) in bindings {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(format!("{:12}", key), Style::default().fg(t.cyan)),
                Span::styled(desc, Style::default().fg(t.text_primary)),
            ]));
        }
        lines.push(Line::from(""));
    }

    // Help text
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("j/k", Style::default().fg(t.cyan)),
        Span::raw(" scroll  "),
        Span::styled("Esc", Style::default().fg(t.cyan)),
        Span::raw("/"),
        Span::styled("?", Style::default().fg(t.cyan)),
        Span::raw(" close"),
    ]));

    let help = Paragraph::new(lines)
        .style(Style::default().fg(t.text_muted))
        .scroll((app.help_scroll_offset, 0));

    frame.render_widget(help, inner);
}
