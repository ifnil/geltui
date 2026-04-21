# Repository Guidelines

## Project Structure & Module Organization
`src/` contains the application code. `main.rs` wires config, Jellyfin session setup, and app startup. Core modules live in `src/config.rs`, `src/jellyfin.rs`, `src/state.rs`, and `src/theme.rs`. TUI rendering is split under `src/ui/` by surface area such as `browser.rs`, `details.rs`, and `footer.rs`. Design notes and refactor plans live in `docs/superpowers/`.

## Build, Test, and Development Commands
Use Cargo for all local work:

- `cargo check` validates the crate quickly without code generation.
- `cargo clippy -- -D warnings` enforces a clean lint pass.
- `cargo build` builds the debug binary.
- `cargo build --release` produces the optimized binary.
- `cargo run` launches the TUI; it expects a valid config file.

Local config defaults to `~/.config/geltui/config.toml`. Set `GELTUI_CONFIG=/path/to/config.toml` to test against an alternate file.

## Coding Style & Naming Conventions
This project uses Rust 2024 with default `rustfmt` behavior. Follow existing Rust naming conventions: `snake_case` for functions and modules, `PascalCase` for structs and enums, and short, explicit method names such as `fetch_root` or `replace_current`. Keep modules focused; if a TUI concern grows, split it into `src/ui/` rather than expanding `app.rs`.

## Testing Guidelines
There is no dedicated test suite yet. Until tests are added, contributors should treat `cargo check` and `cargo clippy -- -D warnings` as mandatory before opening a PR. When adding tests, prefer inline unit tests in the owning module and descriptive names like `loads_config_from_env_override`.

## Commit & Pull Request Guidelines
Recent history uses scoped, imperative commit subjects like `ui: split rendering into submodules` and `config: add optional accent_color field`. Keep that format: `<scope>: <imperative summary>`. PRs should include a concise description, note any config or behavior changes, and attach terminal screenshots or recordings for visible TUI updates.

## Configuration & Safety Notes
`Config` uses `#[serde(deny_unknown_fields)]`, so document any new TOML keys when you add them. If you expand Jellyfin API fields on `MediaItem`, keep the requested `Fields` query parameter in sync.
