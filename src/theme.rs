use std::borrow::Cow;

use ratatui::style::{Color, Modifier, Style};

use crate::config::Config;

// ASCII-only so unicode-width agrees with every terminal. The fancier
// ▸/▶/· glyphs are "ambiguous width" (U+25B8, U+25B6, U+00B7): some
// terminals draw them 2 cells wide, and the cursor-advance mismatch
// leaves stale cells at the start of list rows between frames.
pub const FOLDER_ICON: &str = ">";
pub const PLAYABLE_ICON: &str = "*";
pub const OTHER_ICON: &str = "-";
pub const BREADCRUMB_SEP: &str = " > ";

#[derive(Debug, Clone)]
pub struct Theme {
    /// Secondary/muted text: breadcrumb separators, non-playable rows,
    /// metadata subtitles, placeholders.
    pub muted: Style,
    /// Item title (e.g. details panel heading).
    pub title: Style,
    /// Footer help text.
    pub hint: Style,
    /// Description body text (overview paragraphs).
    pub description: Style,
    /// Current breadcrumb segment.
    pub breadcrumb_current: Style,
    /// Selected list row.
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

        // Roles default to DIM (for muted-ish roles) or BOLD (for titles). Any
        // configured fg overrides the terminal's default foreground.
        let muted = style_with(Modifier::DIM, theme.muted_fg.as_deref(), "theme.muted_fg");
        let title = style_with(Modifier::BOLD, theme.title_fg.as_deref(), "theme.title_fg");
        let hint = style_with(Modifier::DIM, theme.hint_fg.as_deref(), "theme.hint_fg");
        let description = style_with(
            Modifier::empty(),
            theme.description_fg.as_deref(),
            "theme.description_fg",
        );
        let breadcrumb_accent = theme
            .breadcrumb_current_fg
            .as_deref()
            .and_then(|raw| resolve_color(Some(raw), "theme.breadcrumb_current_fg"))
            .unwrap_or(accent);
        let breadcrumb_current = Style::default()
            .fg(breadcrumb_accent)
            .add_modifier(Modifier::BOLD);

        Self {
            muted,
            title,
            hint,
            description,
            breadcrumb_current,
            selection,
        }
    }
}

fn style_with(modifier: Modifier, fg_raw: Option<&str>, field: &str) -> Style {
    let mut style = Style::default();
    if !modifier.is_empty() {
        style = style.add_modifier(modifier);
    }
    if let Some(fg) = resolve_color(fg_raw, field) {
        style = style.fg(fg);
    }
    style
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

/// Truncate a string to `max_chars` Unicode scalar values, appending `...` if
/// shortened. `max_chars == 0` returns an empty string.
pub fn truncate(s: &str, max_chars: usize) -> Cow<'_, str> {
    let len = s.chars().count();
    if len <= max_chars {
        return Cow::Borrowed(s);
    }
    if max_chars <= 3 {
        return Cow::Owned(".".repeat(max_chars));
    }
    let keep = max_chars - 3;
    let mut out: String = s.chars().take(keep).collect();
    out.push_str("...");
    Cow::Owned(out)
}
