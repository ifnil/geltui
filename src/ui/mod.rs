mod breadcrumb;
mod browser;
mod details;
mod footer;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::ListState,
};

use crate::{state::Navigator, theme::Theme};

const HOTKEY_HINTS: &str = "Enter open  h back  s shuffle  r reload  q quit";

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

pub fn render(
    frame: &mut Frame,
    navigator: &Navigator,
    theme: &Theme,
    status: &str,
    list_state: &mut ListState,
) -> Areas {
    let areas = layout(frame.area());

    breadcrumb::render(frame, areas.breadcrumb, navigator.trail(), theme);
    browser::render(frame, areas.list, navigator.current(), theme, list_state);
    details::render(
        frame,
        areas.details,
        navigator.current().selected_item(),
        theme,
    );
    footer::render(frame, areas.footer, status, HOTKEY_HINTS, theme);

    areas
}
