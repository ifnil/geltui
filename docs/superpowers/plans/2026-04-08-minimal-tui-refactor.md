# Minimal TUI Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor geltui's TUI into a borderless minimal layout and split the monolithic `app.rs` into focused modules (`state`, `theme`, `ui/`).

**Architecture:** Introduce a `Navigator` type owning the browser stack in `state.rs`, a `Theme` value in `theme.rs`, and a `ui/` directory with one file per rendered region. `app.rs` shrinks to a controller; each pane module receives a minimal slice of state plus `&Theme`, never `&App`.

**Tech Stack:** Rust 2024 edition, ratatui 0.29, crossterm 0.29, serde, anyhow.

**Spec:** `docs/superpowers/specs/2026-04-08-minimal-tui-refactor-design.md`

---

## Note on Testing

This repository currently has no automated tests and no test harness. The spec explicitly keeps it that way for this change. Every task uses `cargo check` and `cargo clippy` as the verification gate — both must pass cleanly (no warnings, since clippy is treated as must-pass in `CLAUDE.md`). A final manual smoke test step is in Task 5.

Each task must leave the code compiling and clippy-clean. Do not stage half-finished migrations across commits.

---

## File Structure

After this plan is complete, `src/` will look like:

```
src/
├── main.rs            # + mod declarations for state, theme, ui
├── config.rs          # + accent_color field
├── jellyfin.rs        # unchanged
├── app.rs             # shrunk: controller only
├── state.rs           # NEW: BrowserState + Navigator
├── theme.rs           # NEW: Theme, icons, color parsing, truncate
└── ui/
    ├── mod.rs         # NEW: top-level render + layout
    ├── breadcrumb.rs  # NEW
    ├── browser.rs     # NEW
    ├── details.rs     # NEW
    └── footer.rs      # NEW
```

---

## Task 1: Add `accent_color` field to Config

**Files:**
- Modify: `src/config.rs`

- [ ] **Step 1: Add the field to the `Config` struct**

In `src/config.rs`, replace the struct definition:

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub server_url: String,
    pub api_key: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub user_id: Option<String>,
    pub mpv_bin: Option<String>,
    pub mpv_args: Option<Vec<String>>,
}
```

with:

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub server_url: String,
    pub api_key: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub user_id: Option<String>,
    pub mpv_bin: Option<String>,
    pub mpv_args: Option<Vec<String>>,
    #[serde(default)]
    pub accent_color: Option<String>,
}
```

- [ ] **Step 2: Verify compile**

Run: `cargo check`
Expected: clean, no errors.

- [ ] **Step 3: Verify clippy**

Run: `cargo clippy -- -D warnings`
Expected: clean, no warnings.

- [ ] **Step 4: Commit**

```bash
git add src/config.rs
git commit -m "config: add optional accent_color field"
```

---

## Task 2: Create `theme.rs` with Theme, icons, color parsing, and truncate helper

**Files:**
- Create: `src/theme.rs`
- Modify: `src/main.rs` (register the module)
- Modify: `src/app.rs` (hold a `Theme` field on `App`)

- [ ] **Step 1: Create `src/theme.rs`**

```rust
use std::borrow::Cow;

use ratatui::style::{Color, Modifier, Style};

use crate::config::Config;

pub const FOLDER_ICON: &str = "\u{25B8}"; // ▸
pub const PLAYABLE_ICON: &str = "\u{25B6}"; // ▶
pub const OTHER_ICON: &str = "\u{00B7}"; // ·
pub const BREADCRUMB_SEP: &str = " \u{203A} "; // " › "

#[derive(Debug, Clone)]
pub struct Theme {
    pub accent: Color,
    pub selection_bg: Color,
    pub dim: Style,
    pub bold: Style,
    pub accent_bold: Style,
    pub selection: Style,
}

impl Theme {
    pub fn from_config(config: &Config) -> Self {
        let accent = config
            .accent_color
            .as_deref()
            .and_then(parse_color)
            .unwrap_or_else(|| {
                if let Some(raw) = config.accent_color.as_deref() {
                    eprintln!(
                        "geltui: unrecognized accent_color `{raw}`, using default (cyan)"
                    );
                }
                Color::Cyan
            });

        let selection_bg = Color::Indexed(237); // dark grey

        Self {
            accent,
            selection_bg,
            dim: Style::default().add_modifier(Modifier::DIM),
            bold: Style::default().add_modifier(Modifier::BOLD),
            accent_bold: Style::default().fg(accent).add_modifier(Modifier::BOLD),
            selection: Style::default().bg(selection_bg),
        }
    }
}

/// Parse a color string. Accepts ANSI names (with optional "bright " prefix)
/// and `#rrggbb` hex. Case-insensitive. Returns `None` on unrecognized input.
pub fn parse_color(input: &str) -> Option<Color> {
    let s = input.trim().to_lowercase();

    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        return Some(Color::Rgb(r, g, b));
    }

    let (bright, name) = match s.strip_prefix("bright ") {
        Some(rest) => (true, rest),
        None => (false, s.as_str()),
    };

    let color = match name {
        "black" => {
            if bright {
                Color::DarkGray
            } else {
                Color::Black
            }
        }
        "red" => {
            if bright {
                Color::LightRed
            } else {
                Color::Red
            }
        }
        "green" => {
            if bright {
                Color::LightGreen
            } else {
                Color::Green
            }
        }
        "yellow" => {
            if bright {
                Color::LightYellow
            } else {
                Color::Yellow
            }
        }
        "blue" => {
            if bright {
                Color::LightBlue
            } else {
                Color::Blue
            }
        }
        "magenta" => {
            if bright {
                Color::LightMagenta
            } else {
                Color::Magenta
            }
        }
        "cyan" => {
            if bright {
                Color::LightCyan
            } else {
                Color::Cyan
            }
        }
        "white" => {
            if bright {
                Color::White
            } else {
                Color::Gray
            }
        }
        _ => return None,
    };

    Some(color)
}

/// Truncate a string to `max_chars` Unicode scalar values, appending `…` if
/// shortened. `max_chars == 0` returns an empty string.
pub fn truncate(s: &str, max_chars: usize) -> Cow<'_, str> {
    let len = s.chars().count();
    if len <= max_chars {
        return Cow::Borrowed(s);
    }
    if max_chars == 0 {
        return Cow::Owned(String::new());
    }
    let keep = max_chars - 1;
    let mut out: String = s.chars().take(keep).collect();
    out.push('\u{2026}'); // …
    Cow::Owned(out)
}
```

- [ ] **Step 2: Register the module in `main.rs`**

In `src/main.rs`, add `mod theme;` after the existing `mod jellyfin;` line. The file should read:

```rust
mod app;
mod config;
mod jellyfin;
mod theme;

use anyhow::Result;
use app::App;
use config::Config;
use jellyfin::Session;

fn main() -> Result<()> {
    let config = Config::load()?;
    let session = Session::connect(&config)?;
    App::new(config, session)?.run()
}
```

- [ ] **Step 3: Give `App` a `Theme` field**

In `src/app.rs`, update the `App` struct and constructor so the theme is built once at startup.

Replace:

```rust
pub struct App {
    config: Config,
    session: Session,
    stack: Vec<BrowserState>,
    status: String,
}
```

with:

```rust
pub struct App {
    config: Config,
    session: Session,
    #[allow(dead_code)] // used in Task 4 when rendering delegates to ui::
    theme: crate::theme::Theme,
    stack: Vec<BrowserState>,
    status: String,
}
```

The `#[allow(dead_code)]` is necessary because rustc's `dead_code` lint fires on private struct fields that are written but never read. Task 4 will actually read the field and the attribute will be removed then.

Replace the body of `App::new` (the `Ok(Self { ... })` expression). The current body:

```rust
    pub fn new(config: Config, session: Session) -> Result<Self> {
        let root = session.fetch_root()?;
        Ok(Self {
            config,
            session,
            stack: vec![BrowserState::new(None, None, "Libraries".to_string(), root)],
            status: "Connected to Jellyfin. Press Enter to open or play.".to_string(),
        })
    }
```

becomes:

```rust
    pub fn new(config: Config, session: Session) -> Result<Self> {
        let root = session.fetch_root()?;
        let theme = crate::theme::Theme::from_config(&config);
        Ok(Self {
            config,
            session,
            theme,
            stack: vec![BrowserState::new(None, None, "Libraries".to_string(), root)],
            status: "Connected to Jellyfin. Press Enter to open or play.".to_string(),
        })
    }
```

- [ ] **Step 4: Verify compile**

Run: `cargo check`
Expected: clean. The `#[allow(dead_code)]` on the `theme` field suppresses the "field never read" warning until Task 4 wires it up.

- [ ] **Step 5: Verify clippy**

Run: `cargo clippy -- -D warnings`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add src/theme.rs src/main.rs src/app.rs
git commit -m "theme: add Theme module with icons, color parsing, truncate"
```

---

## Task 3: Create `state.rs` with `BrowserState` and `Navigator`

**Files:**
- Create: `src/state.rs`
- Modify: `src/main.rs` (register module)
- Modify: `src/app.rs` (use `Navigator`, delete local `BrowserState`)

- [ ] **Step 1: Create `src/state.rs`**

```rust
use crate::jellyfin::MediaItem;

#[derive(Debug, Clone)]
pub struct BrowserState {
    pub parent_id: Option<String>,
    pub parent_kind: Option<String>,
    pub title: String,
    pub items: Vec<MediaItem>,
    pub selected: usize,
}

impl BrowserState {
    pub fn new(
        parent_id: Option<String>,
        parent_kind: Option<String>,
        title: String,
        items: Vec<MediaItem>,
    ) -> Self {
        Self::with_selection(parent_id, parent_kind, title, items, 0)
    }

    pub fn with_selection(
        parent_id: Option<String>,
        parent_kind: Option<String>,
        title: String,
        items: Vec<MediaItem>,
        selected: usize,
    ) -> Self {
        let selected = if items.is_empty() {
            0
        } else {
            selected.min(items.len().saturating_sub(1))
        };

        Self {
            parent_id,
            parent_kind,
            title,
            items,
            selected,
        }
    }

    pub fn is_season_view(&self) -> bool {
        self.parent_kind.as_deref() == Some("Season")
    }

    pub fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = (self.selected + 1).min(self.items.len() - 1);
    }

    pub fn previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn selected_item(&self) -> Option<&MediaItem> {
        self.items.get(self.selected)
    }
}

/// Stack-backed navigator. Invariant: `stack` is never empty.
#[derive(Debug)]
pub struct Navigator {
    stack: Vec<BrowserState>,
}

impl Navigator {
    pub fn new(root: BrowserState) -> Self {
        Self { stack: vec![root] }
    }

    pub fn current(&self) -> &BrowserState {
        self.stack
            .last()
            .expect("navigator stack invariant: never empty")
    }

    pub fn current_mut(&mut self) -> &mut BrowserState {
        self.stack
            .last_mut()
            .expect("navigator stack invariant: never empty")
    }

    pub fn push(&mut self, state: BrowserState) {
        self.stack.push(state);
    }

    /// Pop the current state. Returns `false` if already at the root (stack
    /// unchanged), `true` otherwise.
    pub fn pop(&mut self) -> bool {
        if self.stack.len() > 1 {
            self.stack.pop();
            true
        } else {
            false
        }
    }

    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// The full navigation trail, root first, current last.
    pub fn trail(&self) -> &[BrowserState] {
        &self.stack
    }

    /// Replace the current state in place (for reload).
    pub fn replace_current(&mut self, state: BrowserState) {
        let last_idx = self.stack.len() - 1;
        self.stack[last_idx] = state;
    }
}
```

- [ ] **Step 2: Register the module in `main.rs`**

Add `mod state;` to `src/main.rs`. The mod declarations should now read:

```rust
mod app;
mod config;
mod jellyfin;
mod state;
mod theme;
```

- [ ] **Step 3: Update `app.rs` to use `Navigator` and remove the local `BrowserState`**

In `src/app.rs`:

1. Replace the imports block at the top (the `use crate::{...}` block) with:

```rust
use crate::{
    config::Config,
    jellyfin::{MediaItem, Session},
    state::{BrowserState, Navigator},
};
```

2. Replace the `App` struct (keeping the `#[allow(dead_code)]` on `theme` — it will be removed in Task 4):

```rust
pub struct App {
    config: Config,
    session: Session,
    #[allow(dead_code)] // removed in Task 4
    theme: crate::theme::Theme,
    navigator: Navigator,
    status: String,
}
```

3. Replace `App::new`:

```rust
    pub fn new(config: Config, session: Session) -> Result<Self> {
        let root_items = session.fetch_root()?;
        let theme = crate::theme::Theme::from_config(&config);
        let root = BrowserState::new(None, None, "Libraries".to_string(), root_items);
        Ok(Self {
            config,
            session,
            theme,
            navigator: Navigator::new(root),
            status: "Connected to Jellyfin. Press Enter to open or play.".to_string(),
        })
    }
```

4. Delete the entire `struct BrowserState { ... }` declaration and its `impl BrowserState { ... }` block from `app.rs`. They now live in `state.rs`.

5. Replace the `current` and `current_mut` helpers on `App`:

```rust
    fn current(&self) -> &BrowserState {
        self.navigator.current()
    }

    fn current_mut(&mut self) -> &mut BrowserState {
        self.navigator.current_mut()
    }
```

(These stay as thin wrappers so `handle_key`, `open_selected`, etc., don't need to change yet.)

6. Replace `go_back`:

```rust
    fn go_back(&mut self) {
        if self.navigator.pop() {
            self.status = "Returned to previous view.".to_string();
        }
    }
```

7. Replace `open_selected`'s folder-push branch. The current code reads:

```rust
        if is_folder {
            let items = self.session.fetch_children(&id)?;
            self.stack
                .push(BrowserState::new(Some(id), Some(kind), name, items));
            self.status = "Loaded folder.".to_string();
            return Ok(());
        }
```

Change the `self.stack.push(...)` line to:

```rust
            self.navigator
                .push(BrowserState::new(Some(id), Some(kind), name, items));
```

8. Replace `reload_current`. The current function body reads:

```rust
    fn reload_current(&mut self) -> Result<()> {
        let refreshed = match self.current().parent_id.as_deref() {
            Some(parent_id) => self.session.fetch_children(parent_id)?,
            None => self.session.fetch_root()?,
        };

        let current = self.current_mut();
        let parent_id = current.parent_id.clone();
        let parent_kind = current.parent_kind.clone();
        let title = current.title.clone();
        let selected = current.selected;
        *current = BrowserState::with_selection(parent_id, parent_kind, title, refreshed, selected);
        self.status = "Reloaded view.".to_string();
        Ok(())
    }
```

Replace with:

```rust
    fn reload_current(&mut self) -> Result<()> {
        let refreshed = match self.navigator.current().parent_id.as_deref() {
            Some(parent_id) => self.session.fetch_children(parent_id)?,
            None => self.session.fetch_root()?,
        };

        let current = self.navigator.current();
        let parent_id = current.parent_id.clone();
        let parent_kind = current.parent_kind.clone();
        let title = current.title.clone();
        let selected = current.selected;
        self.navigator.replace_current(BrowserState::with_selection(
            parent_id,
            parent_kind,
            title,
            refreshed,
            selected,
        ));
        self.status = "Reloaded view.".to_string();
        Ok(())
    }
```

- [ ] **Step 4: Verify compile**

Run: `cargo check`
Expected: clean. The `render` function still references `self.current()` and helper functions in `app.rs` — it will work unchanged because we kept `App::current` as a wrapper.

If the compiler flags an unused import `MediaItem`, check whether `render_details` (still in `app.rs` at this task's point) still references it. It does. Leave the import.

- [ ] **Step 5: Verify clippy**

Run: `cargo clippy -- -D warnings`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add src/state.rs src/main.rs src/app.rs
git commit -m "state: extract BrowserState and add Navigator type"
```

---

## Task 4: Create `ui/` module with the new borderless design

This is the largest task. It creates five new files, migrates rendering out of `app.rs`, and replaces the visual design in one atomic change (so we don't land throwaway bordered-layout intermediate files).

**Files:**
- Create: `src/ui/mod.rs`
- Create: `src/ui/breadcrumb.rs`
- Create: `src/ui/browser.rs`
- Create: `src/ui/details.rs`
- Create: `src/ui/footer.rs`
- Modify: `src/main.rs` (register `ui` module)
- Modify: `src/app.rs` (delete old rendering, delegate to `ui::render`)

- [ ] **Step 1: Register the `ui` module in `main.rs`**

`src/main.rs` mod declarations:

```rust
mod app;
mod config;
mod jellyfin;
mod state;
mod theme;
mod ui;
```

- [ ] **Step 2: Create `src/ui/mod.rs`**

```rust
mod breadcrumb;
mod browser;
mod details;
mod footer;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
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
```

- [ ] **Step 3: Create `src/ui/breadcrumb.rs`**

```rust
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
            spans.push(Span::styled(BREADCRUMB_SEP.to_string(), theme.dim));
        }
        let style = if i == last_idx {
            theme.accent_bold
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
            .filter(|(_, &b)| b > MIN_SEG)
            .max_by_key(|(_, &b)| b)
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
    let drop = joined_len - max_width + 1; // +1 for the leading ellipsis
    let mut out = String::from("\u{2026}"); // …
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
    if budget == 1 {
        return "\u{2026}".to_string();
    }
    let keep = budget - 1;
    let mut out: String = s.chars().take(keep).collect();
    out.push('\u{2026}');
    out
}

fn join_with_sep(segments: &[String]) -> String {
    segments.join(BREADCRUMB_SEP)
}
```

- [ ] **Step 4: Create `src/ui/browser.rs`**

```rust
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

pub fn render(frame: &mut Frame, area: Rect, state: &BrowserState, theme: &Theme) {
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
                list_item.style(theme.dim)
            } else {
                list_item
            }
        })
        .collect();

    let list = List::new(items).highlight_style(theme.selection);

    let mut list_state = ListState::default();
    if !state.items.is_empty() {
        list_state.select(Some(state.selected));
    }
    frame.render_stateful_widget(list, area, &mut list_state);
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
    if is_season
        && let Some(ep) = item.index_number
    {
        return format!("{ep:02} \u{2014} {}", item.name);
    }
    item.name.clone()
}
```

- [ ] **Step 5: Create `src/ui/details.rs`**

```rust
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
```

- [ ] **Step 6: Create `src/ui/footer.rs`**

```rust
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
```

- [ ] **Step 7: Rewrite `app.rs` to delegate rendering**

In `src/app.rs`:

1. Replace the `use ratatui::{...}` import block. The current block imports a lot of rendering types that are no longer needed here:

```rust
use ratatui::{
    Frame,
    Terminal,
    backend::CrosstermBackend,
};
```

(Only `Frame`, `Terminal`, and `CrosstermBackend` remain — the layout/style/text/widgets imports all go away, since `app.rs` no longer renders.)

2. Remove the `#[allow(dead_code)]` attribute from the `theme` field on `App`. After this step, the field is read by `App::render`, so the attribute is no longer needed.

3. Delete the `fn render(&self, frame: &mut Frame)` method body and replace it with:

```rust
    fn render(&self, frame: &mut Frame) {
        crate::ui::render(frame, &self.navigator, &self.theme, &self.status);
    }
```

4. Delete the two free functions at the bottom of `app.rs`: `fn format_list_label(...)` and `fn render_details(...)`. They are now superseded by `ui::browser::format_label` and `ui::details::build_text`.

5. Delete the `current`/`current_mut` wrapper methods on `App` — with rendering gone, the only remaining callers are `go_back`, `open_selected`, and `reload_current`, which already use `self.navigator.current*()` directly after Task 3. Do a grep in `app.rs` for `self.current(` and `self.current_mut(` and replace any remaining calls inside `handle_key` with `self.navigator.current_mut()`.

   Specifically, `handle_key`:

```rust
    fn handle_key(&mut self, code: KeyCode) -> Result<bool> {
        match code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Down | KeyCode::Char('j') => self.navigator.current_mut().next(),
            KeyCode::Up | KeyCode::Char('k') => self.navigator.current_mut().previous(),
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => self.open_selected()?,
            KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => self.go_back(),
            KeyCode::Char('r') => self.reload_current()?,
            _ => {}
        }

        Ok(false)
    }
```

   And `open_selected`: change `self.current().selected_item()` to `self.navigator.current().selected_item()`.

6. Do NOT remove the `BrowserState` import from `app.rs` — it is still needed inside `open_selected` where the folder-push branch calls `BrowserState::new(...)`. Verify the import line `use crate::state::{BrowserState, Navigator};` is still present.

- [ ] **Step 8: Verify compile**

Run: `cargo check`
Expected: clean. If there are unresolved imports, they are likely leftovers from the deleted rendering code — trim them.

Common issues and fixes:
- `unused import: MediaItem` in `app.rs` → remove it from the `use crate::{...}` block.
- `unused import: ratatui::...` → trim the ratatui imports down to just `Frame`, `Terminal`, `backend::CrosstermBackend`.
- `private field theme never read` → it is now read by `App::render`, so this warning should not appear. If it does, the delegation wasn't wired up; fix step 7.

- [ ] **Step 9: Verify clippy**

Run: `cargo clippy -- -D warnings`
Expected: clean.

Common clippy nits and fixes:
- `needless_borrow` around arguments passed to render helpers → remove the `&`.
- `use_self` warnings inside `ui/*.rs` → these should not appear because the types are defined elsewhere, but if they do, apply as suggested.

- [ ] **Step 10: Commit**

```bash
git add src/ui src/app.rs src/main.rs
git commit -m "ui: split rendering into submodules and drop all borders"
```

---

## Task 5: Final verification

**Files:** None.

- [ ] **Step 1: Full cargo check**

Run: `cargo check`
Expected: clean.

- [ ] **Step 2: Full cargo clippy**

Run: `cargo clippy -- -D warnings`
Expected: clean.

- [ ] **Step 3: Build the release binary**

Run: `cargo build --release`
Expected: builds successfully.

- [ ] **Step 4: Manual smoke test**

Run: `cargo run`
Expected behavior:
1. TUI launches with no borders anywhere.
2. Top row shows `Libraries` (the breadcrumb starts with just the root).
3. Row below is blank.
4. List on the left shows libraries with `▸` folder icons.
5. Details pane on the right shows info about the selected library (no box).
6. Bottom row shows a connection status message on the left and `Enter open  h back  r reload  q quit` dimmed on the right.
7. Moving with `j/k` highlights rows with a subtle grey background bar (no `> ` prefix arrow).
8. Pressing Enter on a library drills in; breadcrumb becomes `Libraries › <name>`, with the last segment bold-and-accent-colored.
9. Drilling deeper extends the breadcrumb further.
10. Shrinking the terminal width forces breadcrumb segments to truncate with `…`, starting with the longest.
11. `h` or Backspace pops back up one level.
12. `r` triggers a reload and the status message updates.
13. `q` or `Ctrl-C` exits cleanly and the terminal returns to normal (no stuck alt-screen or raw mode).

- [ ] **Step 5: Manual accent color verification**

Add `accent_color = "magenta"` to your config, launch again, and confirm:
- The current (last) breadcrumb segment is bold magenta.
- The `›` separators remain dim.

Then try `accent_color = "#5fafff"` and confirm the last segment is a light blue.

Then try `accent_color = "not-a-color"` and confirm:
- The app still launches (no panic).
- A single warning line prints to stderr before the TUI takes over: `geltui: unrecognized accent_color ...`.
- The breadcrumb's last segment is cyan (default).

- [ ] **Step 6: Tag the working state (optional)**

If everything above passes, the refactor is done. No further commit is needed unless you discovered issues and made follow-up fixes during smoke testing.

---

## Scope reminder

**In scope for this plan:** visual redesign (borderless, breadcrumb, Unicode icons, subtle selection bar, single-line footer), module split (state / theme / ui), `accent_color` config field.

**Out of scope:** async networking, test harness, multi-server support, keybinding changes, new Jellyfin features, MPV launcher changes.

Do not add scope creep. If you notice something that would be nice to fix while you're in a file (e.g., a minor clippy nit not touched by this plan), raise it separately — do not bundle.
