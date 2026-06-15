use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::App;
use crate::config::Config;
use crate::types::AppState;
use crate::ui::theme::{centered_rect, Theme};
use std::path::Path;

pub const CONFIG_ITEM_COUNT: usize = 7;

pub fn render(frame: &mut Frame, app: &App) {
    let t = &app.theme;
    let (selected_index, editing) = match app.state {
        AppState::ConfigModal {
            selected_index,
            editing,
        } => (selected_index, editing),
        _ => (0, false),
    };

    let area = centered_rect(60, 60, frame.area());

    // Clear the background
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Config Settings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(1), // Spacing
        Constraint::Length(1), // Config path header
        Constraint::Length(1), // Config path value
        Constraint::Length(1), // Spacing
        Constraint::Length(1), // Settings header
        Constraint::Length(1), // Editor
        Constraint::Length(1), // Terminal
        Constraint::Length(1), // Worktree root
        Constraint::Length(1), // Copy files
        Constraint::Length(1),
        Constraint::Length(1), // Run post-add in tmux
        Constraint::Length(1), // Post-add script
        Constraint::Min(1),    // Spacing
        Constraint::Length(1), // Help
    ])
    .split(inner);

    // Config path header
    let path_header = Paragraph::new(Line::from(vec![Span::styled(
        "Config File:",
        Style::default()
            .fg(t.text_primary)
            .add_modifier(Modifier::BOLD),
    )]));
    frame.render_widget(path_header, chunks[1]);

    // Config path value
    let config_path = get_config_path(app);
    let path_value = Paragraph::new(Line::from(vec![Span::styled(
        config_path,
        Style::default().fg(t.text_muted),
    )]));
    frame.render_widget(path_value, chunks[2]);

    // Settings header
    let settings_header = Paragraph::new(Line::from(vec![Span::styled(
        "Settings:",
        Style::default()
            .fg(t.text_primary)
            .add_modifier(Modifier::BOLD),
    )]));
    frame.render_widget(settings_header, chunks[4]);

    // Render each config item
    render_config_item(
        frame,
        chunks[5],
        "editor",
        &get_editor_display(app),
        selected_index == 0,
        editing && selected_index == 0,
        &app.input_buffer,
        t,
    );
    render_config_item(
        frame,
        chunks[6],
        "terminal",
        &get_terminal_display(app),
        selected_index == 1,
        editing && selected_index == 1,
        &app.input_buffer,
        t,
    );
    render_config_item(
        frame,
        chunks[7],
        "worktree_root",
        &get_worktree_root_display(app),
        selected_index == 2,
        editing && selected_index == 2,
        &app.input_buffer,
        t,
    );
    render_config_item(
        frame,
        chunks[8],
        "copy_files",
        &get_copy_files_display(app),
        selected_index == 3,
        editing && selected_index == 3,
        &app.input_buffer,
        t,
    );
    render_config_item(
        frame,
        chunks[9],
        "tmux_worktree_mode",
        &get_tmux_worktree_display(app),
        selected_index == 4,
        false,
        &app.input_buffer,
        t,
    );
    render_config_item(
        frame,
        chunks[10],
        "run_post_add_script_in_tmux",
        &get_tmux_script_display(app),
        selected_index == 5,
        false,
        &app.input_buffer,
        t,
    );
    render_config_item(
        frame,
        chunks[11],
        "post_add_script",
        &get_script_display(app),
        selected_index == 6,
        false,
        &app.input_buffer,
        t,
    );

    // Help text
    let help_text = if editing {
        vec![
            Span::styled("Enter", Style::default().fg(t.cyan)),
            Span::raw(" save  "),
            Span::styled("Esc", Style::default().fg(t.cyan)),
            Span::raw(" cancel"),
        ]
    } else {
        vec![
            Span::styled("j/k", Style::default().fg(t.cyan)),
            Span::raw(" nav  "),
            Span::styled("Enter", Style::default().fg(t.cyan)),
            Span::raw(" edit/toggle  "),
            Span::styled("s", Style::default().fg(t.cyan)),
            Span::raw(" save  "),
            Span::styled("Esc", Style::default().fg(t.cyan)),
            Span::raw(" close"),
        ]
    };
    let help = Paragraph::new(Line::from(help_text)).style(Style::default().fg(t.text_muted));
    frame.render_widget(help, chunks[13]);
}

fn render_config_item(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    is_selected: bool,
    is_editing: bool,
    input_buffer: &str,
    t: &Theme,
) {
    let cursor = if is_selected { "> " } else { "  " };
    let label_style = if is_selected {
        Style::default().fg(t.cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(t.cyan)
    };

    let spans = if is_editing {
        // Show input buffer with cursor indicator
        let display_value = format!("[{}█]", input_buffer);
        vec![
            Span::styled(cursor, label_style),
            Span::styled(format!("{}: ", label), label_style),
            Span::styled(display_value, Style::default().fg(t.amber)),
        ]
    } else if let Some(hint) = selected_config_hint(label).filter(|_| is_selected) {
        vec![
            Span::styled(cursor, label_style),
            Span::styled(format!("{}: ", label), label_style),
            Span::styled(value, Style::default().fg(t.text_primary)),
            Span::styled(format!(" {}", hint), Style::default().fg(t.text_muted)),
        ]
    } else {
        let value_style = if is_selected {
            Style::default()
                .fg(t.text_primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.text_primary)
        };
        vec![
            Span::styled(cursor, label_style),
            Span::styled(format!("{}: ", label), label_style),
            Span::styled(value, value_style),
        ]
    };

    let line = Paragraph::new(Line::from(spans));
    frame.render_widget(line, area);
}

fn get_editor_display(app: &App) -> String {
    app.config
        .editor
        .as_deref()
        .unwrap_or("(not set)")
        .to_string()
}

fn get_terminal_display(app: &App) -> String {
    app.config
        .terminal
        .as_deref()
        .unwrap_or("(not set)")
        .to_string()
}

fn get_copy_files_display(app: &App) -> String {
    if app.config.copy_files.is_empty() {
        "(none)".to_string()
    } else {
        app.config.copy_files.join(", ")
    }
}

fn get_worktree_root_display(app: &App) -> String {
    if app.repo_is_bare {
        return "bare sibling layout".to_string();
    }

    app.config
        .worktree_root
        .clone()
        .unwrap_or_else(|| format!("{} (default)", Config::default_worktree_root().display()))
}

fn get_script_display(app: &App) -> String {
    script_display(&app.config, &app.project_root_path)
}

fn get_tmux_worktree_display(app: &App) -> String {
    if app.config.tmux_worktree_mode {
        "on".to_string()
    } else {
        "off".to_string()
    }
}

fn script_display(config: &Config, project_root_path: &Path) -> String {
    let script_path = config.resolved_post_add_script_path(project_root_path);
    if script_path.exists() {
        script_path.display().to_string()
    } else {
        format!("{} (not found)", script_path.display())
    }
}

fn get_tmux_script_display(app: &App) -> String {
    tmux_script_display(app.config.run_post_add_script_in_tmux)
}

fn tmux_script_display(run_in_tmux: bool) -> String {
    if run_in_tmux {
        "on global-only".to_string()
    } else {
        "off global-only".to_string()
    }
}

fn selected_config_hint(label: &str) -> Option<&'static str> {
    match label {
        "post_add_script" => Some("(Enter to edit with $EDITOR)"),
        "tmux_worktree_mode" => Some("(Enter to toggle)"),
        "run_post_add_script_in_tmux" => Some("(global config only)"),
        _ => None,
    }
}

fn get_config_path(app: &App) -> String {
    Config::project_config_path(&app.project_root_path)
        .display()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn config_modal_configured_post_add_script_displays_effective_project_path() {
        let project_root = std::env::temp_dir().join(format!(
            "owt_config_modal_script_display_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut config = Config::default();
        config.post_add_script = Some("setup.sh".to_string());

        let display = script_display(&config, &project_root);
        let expected_path = project_root.join("setup.sh").display().to_string();

        assert!(
            display.contains(&expected_path),
            "configured script path should display as effective project-root path: {display}"
        );
        assert!(
            display.contains("(not found)"),
            "missing configured script should still show the effective path: {display}"
        );
        assert!(
            !display.contains(".owt/post-add.sh"),
            "configured script display must not fall back to fixed default path: {display}"
        );
    }

    #[test]
    fn config_modal_tmux_text_communicates_global_only_trust_boundary() {
        let enabled_display = tmux_script_display(true);
        let disabled_display = tmux_script_display(false);
        let selected_hint = selected_config_hint("run_post_add_script_in_tmux").unwrap();
        let selected_text = format!("{disabled_display} {selected_hint}");

        assert_eq!(enabled_display, "on global-only");
        assert_eq!(disabled_display, "off global-only");
        assert_eq!(selected_hint, "(global config only)");
        assert!(selected_text.contains("global"));
        assert!(
            !selected_text.to_ascii_lowercase().contains("project"),
            "tmux row/hint must not imply project config can enable auto-run: {selected_text}"
        );
    }

    #[test]
    fn config_modal_default_post_add_script_display_uses_default_helper_path() {
        let project_root = PathBuf::from("/tmp/owt-config-modal-default-display");
        let config = Config::default();

        let display = script_display(&config, &project_root);
        let expected_path = Config::post_add_script_path(&project_root)
            .display()
            .to_string();

        assert!(display.contains(&expected_path));
    }
}
