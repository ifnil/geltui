use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::{
    state::BrowserState,
    theme::{BREADCRUMB_SEP, Theme},
};

const MIN_SEG: usize = 3;

pub fn render(frame: &mut Frame, area: Rect, trail: &[BrowserState], theme: &Theme) {
    if area.width == 0 || trail.is_empty() {
        return;
    }

    let segments: Vec<&str> = trail.iter().map(|s| s.title.as_str()).collect();
    let spans = build_spans(&segments, area.width as usize, theme);
    let paragraph = Paragraph::new(Line::from(spans));
    frame.render_widget(paragraph, area);
}

fn build_spans(segments: &[&str], max_width: usize, theme: &Theme) -> Vec<Span<'static>> {
    let shortened = fit_segments(segments, max_width);
    let last_idx = shortened.len().saturating_sub(1);

    let mut spans: Vec<Span<'static>> = Vec::with_capacity(shortened.len() * 2);
    for (i, seg) in shortened.into_iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(BREADCRUMB_SEP.to_string(), theme.muted));
        }
        let style = if i == last_idx {
            theme.breadcrumb_current
        } else {
            Style::default()
        };
        spans.push(Span::styled(seg, style));
    }
    spans
}

/// Shorten segments to fit inside `max_width` characters when joined with the
/// breadcrumb separator. Shortens the longest segment first, down to a
/// per-segment minimum. If all segments are at their minimum and it still
/// overflows, falls back to left-truncating the whole joined line with a
/// leading ellipsis so the deepest (rightmost) part stays visible.
fn fit_segments(segments: &[&str], max_width: usize) -> Vec<String> {
    if segments.is_empty() {
        return Vec::new();
    }

    let sep_chars = BREADCRUMB_SEP.chars().count();
    let sep_total = sep_chars * segments.len().saturating_sub(1);

    let mut budgets: Vec<usize> = segments.iter().map(|s| s.chars().count()).collect();

    loop {
        let sum: usize = budgets.iter().sum();
        if sum + sep_total <= max_width {
            break;
        }
        let longest = budgets
            .iter()
            .enumerate()
            .filter(|&(_, &b)| b > MIN_SEG)
            .max_by_key(|&(_, &b)| b)
            .map(|(i, _)| i);
        match longest {
            Some(i) => budgets[i] -= 1,
            None => break,
        }
    }

    let shortened: Vec<String> = segments
        .iter()
        .zip(budgets.iter())
        .map(|(seg, &budget)| shorten_to(seg, budget))
        .collect();

    let joined = join_with_sep(&shortened);
    let joined_len = joined.chars().count();
    if joined_len <= max_width {
        return shortened;
    }

    // Fallback: return a single segment containing the left-truncated full line.
    let drop = joined_len - max_width + 3; // +3 for the leading "..."
    let mut out = String::from("...");
    out.extend(joined.chars().skip(drop));
    vec![out]
}

fn shorten_to(s: &str, budget: usize) -> String {
    let len = s.chars().count();
    if len <= budget {
        return s.to_string();
    }
    if budget == 0 {
        return String::new();
    }
    if budget <= 3 {
        return ".".repeat(budget);
    }
    let keep = budget - 3;
    let mut out: String = s.chars().take(keep).collect();
    out.push_str("...");
    out
}

fn join_with_sep(segments: &[String]) -> String {
    segments.join(BREADCRUMB_SEP)
}
