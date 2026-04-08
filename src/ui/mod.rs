mod breadcrumb;
mod browser;
mod details;
mod footer;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

use crate::{state::Navigator, theme::Theme};

const HOTKEY_HINTS: &str = "Enter open  h back  r reload  q quit";

pub fn render(frame: &mut Frame, navigator: &Navigator, theme: &Theme, status: &str) {
    let area = frame.area();

    let [breadcrumb_area, _spacer, body, footer_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(area);

    let [list_area, _gutter, details_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(42),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .areas(body);

    breadcrumb::render(frame, breadcrumb_area, navigator.trail(), theme);
    browser::render(frame, list_area, navigator.current(), theme);
    details::render(
        frame,
        details_area,
        navigator.current().selected_item(),
        theme,
    );
    footer::render(frame, footer_area, status, HOTKEY_HINTS, theme);
}
