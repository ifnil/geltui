use ratatui::{
    Frame,
    layout::Rect,
    text::Line,
    widgets::{Clear, List, ListItem, ListState},
};

use crate::{
    jellyfin::MediaItem,
    state::BrowserState,
    theme::{FOLDER_ICON, OTHER_ICON, PLAYABLE_ICON, Theme},
};

pub fn render(
    frame: &mut Frame,
    area: Rect,
    state: &BrowserState,
    theme: &Theme,
    list_state: &mut ListState,
) {
    // Wipe the area so stale cells from a prior frame can't leak through. The
    // `List` widget only writes the cells its items cover, so shorter new
    // labels leave leftover chars from longer previous ones.
    frame.render_widget(Clear, area);

    let is_season = state.is_season_view();

    let items: Vec<ListItem> = state
        .items
        .iter()
        .map(|item| {
            let icon = icon_for(item);
            let label = format_label(item, is_season);
            let line = Line::from(format!("{icon} {label}"));
            let list_item = ListItem::new(line);
            if !item.is_folder && !item.is_playable() {
                list_item.style(theme.muted)
            } else {
                list_item
            }
        })
        .collect();

    let list = List::new(items).highlight_style(theme.selection);

    if state.items.is_empty() {
        list_state.select(None);
        *list_state.offset_mut() = 0;
    } else {
        list_state.select(Some(state.selected));
        *list_state.offset_mut() = center_offset(state.selected, state.items.len(), area.height as usize);
    }
    frame.render_stateful_widget(list, area, list_state);
}

/// Pick a scroll offset that keeps the selection roughly centered in the
/// viewport. Clamps to the list's bounds so short lists (or selections near
/// the ends) don't leave blank space above/below unnecessarily.
fn center_offset(selected: usize, total: usize, visible: usize) -> usize {
    if visible == 0 || total <= visible {
        return 0;
    }
    let half = visible / 2;
    let desired = selected.saturating_sub(half);
    let max_offset = total - visible;
    desired.min(max_offset)
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
        format!("{ep:02} \u{2014} {}", item.name)
    } else {
        item.name.clone()
    };

    let mut marks = String::new();
    if item.user_data.is_favorite {
        marks.push(' ');
        marks.push('\u{2605}'); // ★
    }
    if item.user_data.played {
        marks.push(' ');
        marks.push('\u{2713}'); // ✓
    }
    format!("{base}{marks}")
}
