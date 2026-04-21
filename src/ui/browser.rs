use ratatui::{
    Frame,
    layout::Rect,
    text::Line,
    widgets::{List, ListItem, ListState},
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
    } else {
        list_state.select(Some(state.selected));
    }
    frame.render_stateful_widget(list, area, list_state);
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
