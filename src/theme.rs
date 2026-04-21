use std::borrow::Cow;

use ratatui::style::{Color, Modifier, Style};

use crate::config::Config;

pub const FOLDER_ICON: &str = "\u{25B8}"; // ▸
pub const PLAYABLE_ICON: &str = "\u{25B6}"; // ▶
pub const OTHER_ICON: &str = "\u{00B7}"; // ·
pub const BREADCRUMB_SEP: &str = " \u{203A} "; // " › "

#[derive(Debug, Clone)]
pub struct Theme {
    pub dim: Style,
    pub bold: Style,
    pub accent_bold: Style,
    pub selection: Style,
}

impl Theme {
    pub fn from_config(config: &Config) -> Self {
        let theme = &config.theme;
        let accent = resolve_color(theme.accent.as_deref(), "theme.accent").unwrap_or(Color::Cyan);

        // Terminal-colors mode uses reverse-video so selection inherits the
        // terminal's fg/bg and stays legible on both light and dark themes.
        // Explicit selection_bg/selection_fg always win over the default bg
        // but don't override the reverse-video flag.
        let mut selection = Style::default().remove_modifier(Modifier::DIM);
        if theme.terminal_colors {
            selection = selection.add_modifier(Modifier::REVERSED);
        } else if let Some(bg) = resolve_color(theme.selection_bg.as_deref(), "theme.selection_bg")
        {
            selection = selection.bg(bg);
        } else {
            selection = selection.bg(Color::Indexed(237));
        }
        if let Some(fg) = resolve_color(theme.selection_fg.as_deref(), "theme.selection_fg") {
            selection = selection.fg(fg);
        }

        let mut dim = Style::default().add_modifier(Modifier::DIM);
        if let Some(fg) = resolve_color(theme.dim_fg.as_deref(), "theme.dim_fg") {
            dim = dim.fg(fg);
        }

        Self {
            dim,
            bold: Style::default().add_modifier(Modifier::BOLD),
            accent_bold: Style::default().fg(accent).add_modifier(Modifier::BOLD),
            selection,
        }
    }
}

fn resolve_color(value: Option<&str>, field: &str) -> Option<Color> {
    let raw = value?;
    match parse_color(raw) {
        Some(color) => Some(color),
        None => {
            eprintln!("geltui: unrecognized {field} `{raw}`, using default");
            None
        }
    }
}

/// Parse a color string. Accepts ANSI names (with optional "bright " prefix)
/// and `#rrggbb` hex. Case-insensitive. Returns `None` on unrecognized input.
pub fn parse_color(input: &str) -> Option<Color> {
    let s = input.trim().to_lowercase();

    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        return Some(Color::Rgb(r, g, b));
    }

    let (bright, name) = match s.strip_prefix("bright ") {
        Some(rest) => (true, rest),
        None => (false, s.as_str()),
    };

    let color = match name {
        "black" => {
            if bright {
                Color::DarkGray
            } else {
                Color::Black
            }
        }
        "red" => {
            if bright {
                Color::LightRed
            } else {
                Color::Red
            }
        }
        "green" => {
            if bright {
                Color::LightGreen
            } else {
                Color::Green
            }
        }
        "yellow" => {
            if bright {
                Color::LightYellow
            } else {
                Color::Yellow
            }
        }
        "blue" => {
            if bright {
                Color::LightBlue
            } else {
                Color::Blue
            }
        }
        "magenta" => {
            if bright {
                Color::LightMagenta
            } else {
                Color::Magenta
            }
        }
        "cyan" => {
            if bright {
                Color::LightCyan
            } else {
                Color::Cyan
            }
        }
        "white" => {
            if bright {
                Color::White
            } else {
                Color::Gray
            }
        }
        _ => return None,
    };

    Some(color)
}

/// Truncate a string to `max_chars` Unicode scalar values, appending `…` if
/// shortened. `max_chars == 0` returns an empty string.
pub fn truncate(s: &str, max_chars: usize) -> Cow<'_, str> {
    let len = s.chars().count();
    if len <= max_chars {
        return Cow::Borrowed(s);
    }
    if max_chars == 0 {
        return Cow::Owned(String::new());
    }
    let keep = max_chars - 1;
    let mut out: String = s.chars().take(keep).collect();
    out.push('\u{2026}'); // …
    Cow::Owned(out)
}
