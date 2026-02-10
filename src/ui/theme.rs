use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Color,
};

/// All UI colors used throughout the application.
#[derive(Debug, Clone)]
pub struct Theme {
    pub accent: Color,
    pub accent_dim: Color,
    pub amber: Color,
    pub red: Color,
    pub cyan: Color,
    pub green: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub bg_elevated: Color,
    pub border: Color,
    pub selection_bg: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            accent: Color::Rgb(16, 185, 129),       // Emerald green
            accent_dim: Color::Rgb(6, 95, 70),      // Darker emerald
            amber: Color::Rgb(245, 158, 11),        // Amber/yellow
            red: Color::Rgb(239, 68, 68),           // Red
            cyan: Color::Rgb(34, 211, 238),         // Cyan
            green: Color::Rgb(16, 185, 129),        // Same as accent
            text_primary: Color::Rgb(250, 250, 250),
            text_secondary: Color::Rgb(161, 161, 170),
            text_muted: Color::Rgb(113, 113, 122),
            bg_elevated: Color::Rgb(39, 39, 42),
            border: Color::Rgb(63, 63, 70),
            selection_bg: Color::Rgb(6, 95, 70),    // Same as accent_dim
        }
    }

    pub fn light() -> Self {
        Self {
            accent: Color::Rgb(5, 150, 105),        // Darker emerald for contrast
            accent_dim: Color::Rgb(209, 250, 229),  // Light emerald bg
            amber: Color::Rgb(180, 83, 9),          // Darker amber
            red: Color::Rgb(185, 28, 28),           // Darker red
            cyan: Color::Rgb(14, 116, 144),         // Darker cyan
            green: Color::Rgb(5, 150, 105),         // Same as accent
            text_primary: Color::Rgb(24, 24, 27),   // Near black
            text_secondary: Color::Rgb(82, 82, 91),
            text_muted: Color::Rgb(161, 161, 170),
            bg_elevated: Color::Rgb(244, 244, 245),
            border: Color::Rgb(212, 212, 216),
            selection_bg: Color::Rgb(209, 250, 229), // Same as accent_dim
        }
    }
}

/// Detect terminal theme from environment.
/// Checks COLORFGBG env var (format "fg;bg", bg >= 7 means light).
/// Falls back to dark theme.
pub fn detect_theme() -> Theme {
    if let Ok(colorfgbg) = std::env::var("COLORFGBG") {
        if let Some(bg_str) = colorfgbg.rsplit(';').next() {
            if let Ok(bg) = bg_str.parse::<u8>() {
                if bg >= 7 {
                    return Theme::light();
                }
            }
        }
    }
    Theme::dark()
}

/// Centered rectangle helper used by all modals.
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
