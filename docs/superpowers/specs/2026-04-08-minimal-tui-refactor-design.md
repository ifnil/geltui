# Minimal TUI Refactor — Design

**Date:** 2026-04-08
**Status:** Draft for review

## Goal

Tighten geltui's TUI into a minimal, borderless interface and split the current monolithic `app.rs` (419 lines) into focused modules so styling and maintenance work touches small, purpose-built files.

Two deliverables in one change:

1. **Visual redesign** — drop all `Block::borders`, replace the titled boxes with a breadcrumb header, a two-column borderless body, and a single-line status+hints footer.
2. **Module split** — introduce `state.rs`, `theme.rs`, and a `ui/` directory so the controller, navigation state, styling, and each rendered pane each live in their own file.

Out of scope: async networking, multi-server support, test coverage, config schema changes beyond adding an accent color.

## Visual design

### Layout

Borderless, four vertical regions (top to bottom):

```
Libraries › Shows › The Expanse › Season 1        ← row 0: breadcrumb (1 line)
                                                  ← row 1: blank spacer (1 line)
▸ Movies                         The Expanse      ← body: list | details
▸ Shows                          Season 1, Episode 3 — "Remember the Cant"
▸ Music
                                 TV-14  |  ★ 8.4  |  2015  |  42m
                                 Drama, Sci-Fi

                                 The Rocinante picks up a distress call...

Playing `The Expanse` in MPV.    Enter open  h back  r reload  q quit   ← footer (1 line)
```

Vertical constraints: `Length(1)` breadcrumb, `Length(1)` spacer, `Min(0)` body, `Length(1)` footer.
Horizontal body constraints: `Percentage(42)` list, `Length(2)` gutter, `Min(0)` details. No vertical rule — the gutter is whitespace.

No `Block`s, no borders, no titles anywhere.

### Breadcrumb

- Segments joined by ` › ` rendered in the dim style.
- Last segment is bold (default weight on all others).
- Built from the navigation stack: segment text is each `BrowserState.title`.
- **Truncation:** if the joined line exceeds terminal width, iteratively shorten the longest segment from the right with `…` until it fits. Minimum per-segment length is 3 characters (`X…`); segments already at the minimum are skipped in favor of the next-longest. If every segment is at its minimum and the line still overflows, fall back to truncating the whole line from the left with a leading `…` so the most recent (deepest) part of the trail stays visible.

### Selection highlight

- Full-width subtle background bar across the list column only (not the details pane), drawn on the selected row.
- Background color: a fixed dark grey (e.g., `Color::Rgb(40, 40, 40)` or `Color::Indexed(237)`).
- No selection prefix character. No foreground change beyond whatever the row already had.

### List rows

- Format: `{icon} {label}`.
- Icons (Unicode): `▸` folder, `▶` playable, `·` other.
- Non-playable, non-folder rows rendered in the dim style.
- Label format unchanged from current behavior (season views prepend `NN — ` episode numbers).

### Details pane

Content unchanged from current; only the box is removed. Lines in order:

1. Title (bold).
2. Series/season/episode line (dim), if applicable.
3. Blank.
4. Metadata line: official rating, community rating (`★ 8.4`), year, child count, runtime — separated by ` | `.
5. Collection type (dim), if applicable.
6. Genres (dim), if any.
7. Blank.
8. Overview (wrapped with `Wrap { trim: true }`), or `No overview available.` (dim).

### Footer

- Single line.
- Status message left-aligned, hotkey hints right-aligned.
- Hotkeys rendered in the dim style: `Enter open  h back  r reload  q quit`.
- If the terminal is too narrow to fit both with at least 2 spaces of gap, hotkeys are truncated with `…` first; status always wins.

### Colors

Resolved once at startup into a `Theme` value. Three concrete palette entries:

- **Accent** — configurable via `config.toml`, default `Color::Cyan`. Used for the breadcrumb separator (`›`) and the bold last-segment of the breadcrumb.
- **Selection background** — fixed dark grey.
- **Dim** — `Style::default().add_modifier(Modifier::DIM)`. Used for metadata, non-playable rows, hotkey hints, genre lines.

## Module structure

```
src/
├── main.rs           # unchanged
├── config.rs         # + optional accent_color field
├── jellyfin.rs       # unchanged
├── app.rs            # controller: App struct, run loop, key handling, actions
├── state.rs          # Navigator + BrowserState
├── theme.rs          # Theme struct, icon constants, truncate helper
└── ui/
    ├── mod.rs        # top-level render() + layout computation
    ├── breadcrumb.rs # breadcrumb line + truncation logic
    ├── browser.rs    # list pane
    ├── details.rs    # details pane
    └── footer.rs     # status + hotkeys line
```

### `state.rs`

Owns all navigation state. No ratatui dependency.

```rust
pub struct BrowserState {
    pub parent_id: Option<String>,
    pub parent_kind: Option<String>,
    pub title: String,
    pub items: Vec<MediaItem>,
    pub selected: usize,
}

impl BrowserState {
    pub fn new(parent_id: Option<String>, parent_kind: Option<String>,
               title: String, items: Vec<MediaItem>) -> Self;
    pub fn with_selection(..., selected: usize) -> Self;
    pub fn next(&mut self);
    pub fn previous(&mut self);
    pub fn selected_item(&self) -> Option<&MediaItem>;
    pub fn is_season_view(&self) -> bool;
}

pub struct Navigator {
    stack: Vec<BrowserState>,  // invariant: never empty
}

impl Navigator {
    pub fn new(root_title: String, items: Vec<MediaItem>) -> Self;
    pub fn current(&self) -> &BrowserState;
    pub fn current_mut(&mut self) -> &mut BrowserState;
    pub fn push(&mut self, state: BrowserState);
    pub fn pop(&mut self) -> bool;          // false if at root
    pub fn depth(&self) -> usize;
    pub fn trail(&self) -> &[BrowserState]; // for breadcrumb rendering
    pub fn replace_current(&mut self, state: BrowserState); // for reload
}
```

The never-empty invariant is enforced by the constructor (`new` seeds exactly one state) and by `pop` returning `false` instead of popping the last element. `current()`/`current_mut()` safely unwrap because of this invariant.

### `theme.rs`

```rust
pub const FOLDER_ICON: &str = "▸";
pub const PLAYABLE_ICON: &str = "▶";
pub const OTHER_ICON: &str = "·";
pub const BREADCRUMB_SEP: &str = " › ";

pub struct Theme {
    pub accent: Color,
    pub selection_bg: Color,
    pub dim: Style,
    pub bold: Style,
    pub accent_bold: Style,
    pub selection: Style,
}

impl Theme {
    pub fn from_config(config: &Config) -> Self;
}

pub fn truncate(s: &str, max_chars: usize) -> Cow<'_, str>;
```

`from_config` parses `config.accent_color` (see below). Invalid strings fall back to `Color::Cyan` with a warning printed to stderr during startup — not a hard error.

### `ui/mod.rs`

```rust
pub fn render(frame: &mut Frame, app: &App, theme: &Theme);
```

Computes the layout (breadcrumb / spacer / body / footer rows; list / gutter / details columns) and delegates each non-spacer region to its submodule. Each submodule exposes a single `render` function that takes the minimum slice of data it needs — never `&App`.

### `ui/breadcrumb.rs`

```rust
pub fn render(frame: &mut Frame, area: Rect, trail: &[BrowserState], theme: &Theme);
```

Builds the joined breadcrumb line, runs truncation to fit `area.width`, and renders as a single `Paragraph`.

### `ui/browser.rs`

```rust
pub fn render(frame: &mut Frame, area: Rect, state: &BrowserState, theme: &Theme);
```

Builds the `List`, `ListState`, highlight style from `theme.selection`, and renders. Label formatting (episode number prefix in season views) moves here as a private helper.

### `ui/details.rs`

```rust
pub fn render(frame: &mut Frame, area: Rect, item: Option<&MediaItem>, theme: &Theme);
```

The existing `render_details` function moves here and becomes a private helper returning `Text<'static>`, styled with `theme.dim` / `theme.bold` for the appropriate lines.

### `ui/footer.rs`

```rust
pub fn render(frame: &mut Frame, area: Rect, status: &str, theme: &Theme);
```

Renders status on the left, dimmed hotkeys on the right. Hotkey string is a module-level constant: `"Enter open  h back  r reload  q quit"`.

### `app.rs` (shrunk)

Keeps only the controller responsibilities:

- `App` struct (`config`, `session`, `theme`, `navigator`, `status`).
- `App::new` — builds `Theme::from_config(&config)`, calls `session.fetch_root()`, constructs `Navigator::new`.
- `App::run`, `App::run_loop` — terminal setup, event polling, panic hook.
- `App::handle_key` — unchanged key mappings.
- `App::open_selected`, `App::reload_current`, `App::go_back` — same behavior, but `go_back` uses `self.navigator.pop()` and `reload_current` uses `self.navigator.replace_current(...)`.
- A `render` wrapper that's a one-liner: `ui::render(frame, self, &self.theme)`.

No `current()`/`current_mut()` on `App` — callers use `self.navigator.current()` directly.

### `config.rs` change

Add one field:

```rust
#[serde(default)]
pub accent_color: Option<String>,
```

Accepted values: ANSI color names (`"black"`, `"red"`, `"green"`, `"yellow"`, `"blue"`, `"magenta"`, `"cyan"`, `"white"`, each also with a `"bright "` prefix) and `#rrggbb` hex strings. Parsing is a small helper in `theme.rs` — no reliance on ratatui's own string parsing (which varies by version). Default when absent or invalid: `Color::Cyan`. Parsing lives in `theme.rs`, not `config.rs`, so the config module stays a pure schema definition.

## Key bindings

Unchanged. `j/k/↑/↓` move, `Enter/l/→` open, `h/←/Backspace` back, `r` reload, `q` or `Ctrl-C` quit.

## Error handling

- Invalid `accent_color` in config: log a warning to stderr, use default. Do not block launch.
- Terminal too narrow for breadcrumb: truncate segments (see Breadcrumb section).
- Terminal too narrow for footer: truncate hotkeys first, status second.
- No terminal size assumptions beyond ratatui's own minimums.

## Testing

No automated tests added as part of this change (repo currently has none). Manual verification:

1. `cargo check` and `cargo clippy` clean.
2. `cargo run` connects, renders the new layout.
3. Navigate Libraries → Shows → a series → a season → an episode; confirm breadcrumb grows and truncates on resize.
4. Play an episode in MPV; confirm status message updates.
5. Resize terminal narrow and wide; confirm no panics and layout degrades gracefully.
6. Launch with and without `accent_color` set in config, including an invalid value, to confirm the fallback warning.

## Migration / build sequence (for the implementation plan)

Rough order (the writing-plans skill will formalize this):

1. Create `theme.rs` with icons, `Theme` struct, accent parsing, `truncate` helper.
2. Create `state.rs`, move `BrowserState` out of `app.rs`, add `Navigator`.
3. Wire `Navigator` into `App`; remove `App::current`/`current_mut`; update `go_back`/`reload_current`.
4. Create `ui/mod.rs` with a placeholder that renders the existing (bordered) layout via delegated submodules — `ui/breadcrumb.rs`, `ui/browser.rs`, `ui/details.rs`, `ui/footer.rs` — so the split lands before the visual redesign.
5. Redesign visuals: drop borders, add breadcrumb, new selection style, Unicode icons, single-line footer with dimmed hotkeys.
6. Add `accent_color` to `config.rs` and plumb it through `Theme::from_config`.
7. `cargo check` / `cargo clippy` / manual smoke test.
