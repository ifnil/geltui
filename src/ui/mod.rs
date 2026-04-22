mod breadcrumb;
pub mod browser;
mod details;
mod footer;
pub mod menu;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
};

use crate::{state::Navigator, theme::Theme};

const HOTKEY_HINTS: &str = "Enter open  h back  m menu  ? help  q quit";

pub struct Areas {
    pub breadcrumb: Rect,
    pub list: Rect,
    pub details: Rect,
    pub footer: Rect,
}

pub fn layout(area: Rect) -> Areas {
    let [breadcrumb, _spacer, body, footer] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(area);

    let [list, _gutter, details] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(42),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .areas(body);

    Areas {
        breadcrumb,
        list,
        details,
        footer,
    }
}

pub fn render(frame: &mut Frame, navigator: &Navigator, theme: &Theme, status: &str) -> Areas {
    let areas = layout(frame.area());

    breadcrumb::render(frame, areas.breadcrumb, navigator.trail(), theme);
    details::render(
        frame,
        areas.details,
        navigator.current().selected_item(),
        theme,
    );
    footer::render(frame, areas.footer, status, HOTKEY_HINTS, theme);
    // NOTE: the list pane is rendered directly to the terminal after
    // `terminal.draw` returns (see `browser::render_direct`), bypassing
    // ratatui's buffer/diff pipeline for that rectangle.

    areas
}
