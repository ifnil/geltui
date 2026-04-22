use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Clear, Paragraph},
};

use crate::theme::Theme;

#[derive(Debug, Clone)]
pub struct Menu {
    pub title: String,
    pub entries: Vec<String>,
    pub selected: usize,
}

impl Menu {
    pub fn new(title: impl Into<String>, entries: Vec<String>) -> Self {
        Self {
            title: title.into(),
            entries,
            selected: 0,
        }
    }

    pub fn next(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.entries.len();
    }

    pub fn previous(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        if self.selected == 0 {
            self.selected = self.entries.len() - 1;
        } else {
            self.selected -= 1;
        }
    }
}

/// Render a centered interactive menu overlay.
pub fn render_menu(frame: &mut Frame, menu: &Menu, theme: &Theme) {
    let entry_width = menu.entries.iter().map(|e| e.chars().count()).max().unwrap_or(0);
    let title_width = menu.title.chars().count();
    let inner_width = entry_width.max(title_width);
    let width = (inner_width + 4).min(frame.area().width as usize).max(16) as u16;
    let height = (menu.entries.len() + 3).min(frame.area().height as usize) as u16;

    let area = centered_rect(frame.area(), width, height);
    frame.render_widget(Clear, area);

    let mut lines: Vec<Line<'static>> = Vec::with_capacity(menu.entries.len() + 2);
    lines.push(Line::from(Span::styled(menu.title.clone(), theme.title)));
    lines.push(Line::from(""));
    for (i, entry) in menu.entries.iter().enumerate() {
        let marker = if i == menu.selected { "> " } else { "  " };
        let text = format!("{marker}{entry}");
        let style = if i == menu.selected {
            theme.selection
        } else {
            theme.muted
        };
        lines.push(Line::from(Span::styled(text, style)));
    }

    frame.render_widget(Paragraph::new(lines), pad(area));
}

/// Render a static help overlay (non-interactive list of shortcuts).
pub fn render_help(frame: &mut Frame, lines: &[(&str, &str)], theme: &Theme) {
    let key_w = lines.iter().map(|(k, _)| k.chars().count()).max().unwrap_or(0);
    let desc_w = lines.iter().map(|(_, d)| d.chars().count()).max().unwrap_or(0);
    let title = "Keybindings";
    let inner_width = (key_w + 2 + desc_w).max(title.len());
    let width = (inner_width + 4).min(frame.area().width as usize).max(20) as u16;
    let height = (lines.len() + 4).min(frame.area().height as usize) as u16;

    let area = centered_rect(frame.area(), width, height);
    frame.render_widget(Clear, area);

    let mut out: Vec<Line<'static>> = Vec::with_capacity(lines.len() + 3);
    out.push(Line::from(Span::styled(title.to_string(), theme.title)));
    out.push(Line::from(""));
    for (key, desc) in lines {
        let padded = format!("{:<width$}  ", key, width = key_w);
        out.push(Line::from(vec![
            Span::styled(padded, theme.breadcrumb_current),
            Span::styled((*desc).to_string(), theme.description),
        ]));
    }
    out.push(Line::from(""));
    out.push(Line::from(Span::styled("Press Esc or ? to close", theme.hint)));

    frame.render_widget(Paragraph::new(out), pad(area));
}

fn centered_rect(parent: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(parent.width);
    let height = height.min(parent.height);
    let x = parent.x + (parent.width.saturating_sub(width)) / 2;
    let y = parent.y + (parent.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

fn pad(area: Rect) -> Rect {
    Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    )
}
