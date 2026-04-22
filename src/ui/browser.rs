use std::io::{self, Write};

use crossterm::{
    cursor::MoveTo,
    queue,
    style::{
        Attribute, Color as CColor, Print, ResetColor, SetAttribute, SetBackgroundColor,
        SetForegroundColor,
    },
};
use ratatui::{
    layout::Rect,
    style::{Color as RColor, Modifier, Style},
};

use crate::{
    jellyfin::MediaItem,
    state::BrowserState,
    theme::{FOLDER_ICON, OTHER_ICON, PLAYABLE_ICON, Theme},
};

/// Render the list pane by writing directly to the terminal backend, bypassing
/// ratatui's buffer/diff pipeline. Emits an explicit MoveTo + content for every
/// visible row, so stale cells from a prior frame cannot persist — something we
/// hit in ratatui's diff when rows shifted from centered-offset scrolling.
pub fn render_direct<W: Write>(
    out: &mut W,
    area: Rect,
    state: &BrowserState,
    theme: &Theme,
) -> io::Result<()> {
    if area.width == 0 || area.height == 0 {
        return Ok(());
    }
    let visible = area.height as usize;
    let total = state.items.len();
    let offset = if total == 0 {
        0
    } else {
        compute_offset(state.selected, total, visible)
    };
    let is_season = state.is_season_view();
    let width = area.width as usize;

    for row in 0..visible {
        let y = area.y + row as u16;
        queue!(
            out,
            MoveTo(area.x, y),
            SetAttribute(Attribute::Reset),
            ResetColor,
        )?;

        let idx = offset + row;
        if total == 0 || idx >= total {
            queue!(out, Print(" ".repeat(width)))?;
            continue;
        }

        let item = &state.items[idx];
        let label = build_row(item, is_season, width);

        let base_style = if !item.is_folder && !item.is_playable() {
            theme.muted
        } else {
            Style::default()
        };
        let row_style = if idx == state.selected {
            base_style.patch(theme.selection)
        } else {
            base_style
        };

        apply_style(out, row_style)?;
        queue!(out, Print(label))?;
        queue!(out, SetAttribute(Attribute::Reset), ResetColor)?;
    }
    out.flush()
}

/// Centered-ish offset that always shows the selection.
pub fn compute_offset(selected: usize, total: usize, visible: usize) -> usize {
    if visible == 0 || total <= visible {
        return 0;
    }
    let half = visible / 2;
    let desired = selected.saturating_sub(half);
    let max_offset = total - visible;
    desired.min(max_offset)
}

fn apply_style<W: Write>(out: &mut W, style: Style) -> io::Result<()> {
    if let Some(fg) = style.fg {
        queue!(out, SetForegroundColor(r_to_c(fg)))?;
    }
    if let Some(bg) = style.bg {
        queue!(out, SetBackgroundColor(r_to_c(bg)))?;
    }
    let m = style.add_modifier;
    if m.contains(Modifier::BOLD) {
        queue!(out, SetAttribute(Attribute::Bold))?;
    }
    if m.contains(Modifier::DIM) {
        queue!(out, SetAttribute(Attribute::Dim))?;
    }
    if m.contains(Modifier::REVERSED) {
        queue!(out, SetAttribute(Attribute::Reverse))?;
    }
    if m.contains(Modifier::ITALIC) {
        queue!(out, SetAttribute(Attribute::Italic))?;
    }
    if m.contains(Modifier::UNDERLINED) {
        queue!(out, SetAttribute(Attribute::Underlined))?;
    }
    Ok(())
}

fn r_to_c(c: RColor) -> CColor {
    match c {
        RColor::Reset => CColor::Reset,
        RColor::Black => CColor::Black,
        RColor::Red => CColor::DarkRed,
        RColor::Green => CColor::DarkGreen,
        RColor::Yellow => CColor::DarkYellow,
        RColor::Blue => CColor::DarkBlue,
        RColor::Magenta => CColor::DarkMagenta,
        RColor::Cyan => CColor::DarkCyan,
        RColor::Gray => CColor::Grey,
        RColor::DarkGray => CColor::DarkGrey,
        RColor::LightRed => CColor::Red,
        RColor::LightGreen => CColor::Green,
        RColor::LightYellow => CColor::Yellow,
        RColor::LightBlue => CColor::Blue,
        RColor::LightMagenta => CColor::Magenta,
        RColor::LightCyan => CColor::Cyan,
        RColor::White => CColor::White,
        RColor::Rgb(r, g, b) => CColor::Rgb { r, g, b },
        RColor::Indexed(i) => CColor::AnsiValue(i),
    }
}

/// Build a single row's text, padded/truncated to exactly `width` ASCII cells.
fn build_row(item: &MediaItem, is_season: bool, width: usize) -> String {
    let icon = icon_for(item);
    let label = format_label(item, is_season);
    let raw = format!("{icon} {label}");

    let char_count = raw.chars().count();
    if char_count >= width {
        raw.chars().take(width).collect()
    } else {
        let mut padded = String::with_capacity(raw.len() + (width - char_count));
        padded.push_str(&raw);
        for _ in 0..(width - char_count) {
            padded.push(' ');
        }
        padded
    }
}

fn icon_for(item: &MediaItem) -> &'static str {
    if item.is_folder {
        FOLDER_ICON
    } else if item.is_playable() {
        PLAYABLE_ICON
    } else {
        OTHER_ICON
    }
}

fn format_label(item: &MediaItem, is_season: bool) -> String {
    let base = if is_season
        && let Some(ep) = item.index_number
    {
        format!("{ep:02} - {}", item.name)
    } else {
        item.name.clone()
    };

    let mut marks = String::new();
    if item.user_data.is_favorite {
        marks.push_str(" *");
    }
    if item.user_data.played {
        marks.push_str(" v");
    }
    format!("{base}{marks}")
}
