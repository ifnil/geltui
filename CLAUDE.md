# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Check Commands

```bash
cargo check          # Fast type-check without codegen
cargo clippy         # Lint — must pass clean (no warnings)
cargo build          # Debug build
cargo build --release
cargo run            # Run the TUI (requires a valid config file)
```

There are no tests yet. The project has no rustfmt.toml or clippy.toml overrides — default settings apply.

## Architecture

geltui is a terminal UI browser for Jellyfin media servers. It lets you navigate libraries, browse folders, and launch playback in MPV.

**Four source files, three clear layers:**

- `main.rs` — Entry point. Loads config, connects to Jellyfin, runs the app.
- `config.rs` — TOML config loading from `~/.config/geltui/config.toml` (or `$GELTUI_CONFIG`). Supports API key or username/password auth. Uses `#[serde(deny_unknown_fields)]`.
- `jellyfin.rs` — Blocking HTTP client (`reqwest::blocking`) for the Jellyfin REST API. Handles authentication, fetching views/items, and building playback URLs. The `MediaItem` struct maps Jellyfin's PascalCase JSON fields via serde rename attributes.
- `app.rs` — TUI application using ratatui/crossterm. Three-pane layout (header, browse+details, footer). Navigation uses a `Vec<BrowserState>` stack — entering a folder pushes, going back pops. `BrowserState` tracks `parent_kind` to enable context-aware rendering (e.g., episode numbers only shown inside season views).

**Key design decisions:**

- All HTTP is synchronous/blocking — network calls freeze the UI. This is a known limitation.
- MPV is spawned via `std::process::Command` with a reaper thread to avoid zombies. Auth tokens are passed via `--http-header-fields`, not URL query params.
- A panic hook restores the terminal (raw mode + alternate screen) before printing the panic.
- The Jellyfin API fields requested in `fetch_children` must match the fields on `MediaItem` — if you add a field to the struct, add it to the `Fields` query param too (unless Jellyfin returns it by default).

## Rust Edition

Uses `edition = "2024"` (requires Rust 1.85+). This enables `let chains` in if/while expressions natively.
