use std::borrow::Cow;

use ratatui::style::{Color, Modifier, Style};

use crate::config::Config;

#[allow(dead_code)] // used in Task 4 (ui::render)
pub const FOLDER_ICON: &str = "\u{25B8}"; // ▸
#[allow(dead_code)] // used in Task 4 (ui::render)
pub const PLAYABLE_ICON: &str = "\u{25B6}"; // ▶
#[allow(dead_code)] // used in Task 4 (ui::render)
pub const OTHER_ICON: &str = "\u{00B7}"; // ·
#[allow(dead_code)] // used in Task 4 (ui::render)
pub const BREADCRUMB_SEP: &str = " \u{203A} "; // " › "

#[allow(dead_code)] // fields read in Task 4 (ui::render)
#[derive(Debug, Clone)]
pub struct Theme {
    pub accent: Color,
    pub selection_bg: Color,
    pub dim: Style,
    pub bold: Style,
    pub accent_bold: Style,
    pub selection: Style,
}

impl Theme {
    pub fn from_config(config: &Config) -> Self {
        let accent = config
            .accent_color
            .as_deref()
            .and_then(parse_color)
            .unwrap_or_else(|| {
                if let Some(raw) = config.accent_color.as_deref() {
                    eprintln!(
                        "geltui: unrecognized accent_color `{raw}`, using default (cyan)"
                    );
                }
                Color::Cyan
            });

        let selection_bg = Color::Indexed(237); // dark grey

        Self {
            accent,
            selection_bg,
            dim: Style::default().add_modifier(Modifier::DIM),
            bold: Style::default().add_modifier(Modifier::BOLD),
            accent_bold: Style::default().fg(accent).add_modifier(Modifier::BOLD),
            selection: Style::default().bg(selection_bg),
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
#[allow(dead_code)] // used in Task 4 (ui::render)
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
