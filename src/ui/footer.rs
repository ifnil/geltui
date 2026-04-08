use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::theme::{Theme, truncate};

pub fn render(frame: &mut Frame, area: Rect, status: &str, hints: &str, theme: &Theme) {
    if area.width == 0 {
        return;
    }

    let width = area.width as usize;
    let status_len = status.chars().count();
    let hints_len = hints.chars().count();

    // Reserve at least 2 spaces of padding between status and hints.
    let padding = 2usize;

    let (status_final, hints_final) =
        if status_len + padding + hints_len <= width {
            (status.to_string(), hints.to_string())
        } else if status_len + padding + MIN_HINT_CHARS <= width {
            // Truncate hints first.
            let available = width - status_len - padding;
            (status.to_string(), truncate(hints, available).into_owned())
        } else {
            // Status too long — truncate it and drop hints.
            (truncate(status, width).into_owned(), String::new())
        };

    let gap = width
        .saturating_sub(status_final.chars().count() + hints_final.chars().count());
    let gap_str = " ".repeat(gap);

    let spans = vec![
        Span::raw(status_final),
        Span::raw(gap_str),
        Span::styled(hints_final, theme.dim),
    ];

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

const MIN_HINT_CHARS: usize = 8;
