use std::fmt::Write;

use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
};

use crate::{jellyfin::MediaItem, theme::Theme};

pub fn render(frame: &mut Frame, area: Rect, item: Option<&MediaItem>, theme: &Theme) {
    let text = match item {
        Some(item) => build_text(item, theme),
        None => Text::from(vec![
            Line::from(Span::styled("No items found.", theme.dim)),
            Line::from(""),
            Line::from(Span::styled(
                "Check the Jellyfin credentials and library visibility.",
                theme.dim,
            )),
        ]),
    };

    let paragraph = Paragraph::new(text).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn build_text(item: &MediaItem, theme: &Theme) -> Text<'static> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    // Title
    lines.push(Line::from(Span::styled(item.name.clone(), theme.bold)));

    // Series/season/episode
    if let Some(series) = &item.series_name {
        let season_ep = match (item.parent_index_number, item.index_number) {
            (Some(s), Some(e)) => format!("{series} \u{2014} Season {s}, Episode {e}"),
            (None, Some(e)) => format!("{series} \u{2014} Episode {e}"),
            _ => series.clone(),
        };
        lines.push(Line::from(Span::styled(season_ep, theme.dim)));
    }

    lines.push(Line::from(""));

    // Metadata line
    let mut meta = String::new();
    if let Some(rating) = &item.official_rating {
        meta.push_str(rating);
    }
    if let Some(score) = item.community_rating {
        if !meta.is_empty() {
            meta.push_str("  |  ");
        }
        write!(meta, "\u{2605} {score:.1}").unwrap();
    }
    if let Some(yr) = item.production_year {
        if !meta.is_empty() {
            meta.push_str("  |  ");
        }
        write!(meta, "{yr}").unwrap();
    }
    if let Some(count) = item.child_count {
        if !meta.is_empty() {
            meta.push_str("  |  ");
        }
        write!(meta, "{count} items").unwrap();
    }
    if let Some(rt) = item.runtime_ticks {
        if !meta.is_empty() {
            meta.push_str("  |  ");
        }
        meta.push_str(&crate::jellyfin::format_runtime(rt));
    }
    if !meta.is_empty() {
        lines.push(Line::from(meta));
    }

    // Collection type
    if let Some(ct) = &item.collection_type {
        lines.push(Line::from(Span::styled(ct.clone(), theme.dim)));
    }

    // Genres
    if !item.genres.is_empty() {
        lines.push(Line::from(Span::styled(item.genres.join(", "), theme.dim)));
    }

    lines.push(Line::from(""));

    // Overview
    match item.overview.as_deref() {
        Some(overview) if !overview.trim().is_empty() => {
            lines.push(Line::from(overview.to_string()));
        }
        _ => {
            lines.push(Line::from(Span::styled(
                "No overview available.",
                theme.dim,
            )));
        }
    }

    Text::from(lines)
}
