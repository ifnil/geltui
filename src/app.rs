use std::{
    io,
    process::{Command, Stdio},
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame,
    Terminal,
    backend::CrosstermBackend,
    layout::Rect,
    widgets::ListState,
};

use crate::{
    config::Config,
    jellyfin::Session,
    state::{BrowserState, Navigator},
    ui::menu::Menu,
};

enum Modal {
    Help,
    Context { menu: Menu, actions: Vec<ContextAction> },
}

#[derive(Debug, Clone)]
enum ContextAction {
    ToggleWatched(String, bool), // item id, current played state
    ToggleFavorite(String, bool),
    Shuffle,
    ToggleAutoplay,
    Close,
}

const HELP_LINES: &[(&str, &str)] = &[
    ("j / \u{2193}", "next item"),
    ("k / \u{2191}", "previous item"),
    ("l / \u{2192} / Enter", "open / play"),
    ("h / \u{2190} / Backspace", "back"),
    ("s", "shuffle show"),
    ("m", "item menu"),
    ("n", "toggle autoplay next episode"),
    ("r", "reload view"),
    ("?", "toggle help"),
    ("q / Ctrl-C", "quit"),
    ("Esc", "close menu"),
];

pub struct App {
    config: Config,
    session: Session,
    theme: crate::theme::Theme,
    navigator: Navigator,
    status: String,
    list_state: ListState,
    list_area: Rect,
    modal: Option<Modal>,
    autoplay_next: bool,
    /// Set when something external (e.g. mpv) may have corrupted the terminal
    /// so the next frame must be fully redrawn.
    needs_full_redraw: bool,
}

impl App {
    pub fn new(config: Config, session: Session) -> Result<Self> {
        let root_items = session.fetch_root()?;
        let theme = crate::theme::Theme::from_config(&config);
        let root = BrowserState::new(None, None, "Libraries".to_string(), root_items);
        let autoplay_next = config.general.autoplay_next;
        Ok(Self {
            config,
            session,
            theme,
            navigator: Navigator::new(root),
            status: "Connected to Jellyfin. Press Enter to open or play.".to_string(),
            list_state: ListState::default(),
            list_area: Rect::default(),
            modal: None,
            autoplay_next,
            needs_full_redraw: false,
        })
    }

    pub fn run(mut self) -> Result<()> {
        let mouse_enabled = self.config.general.mouse;
        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), DisableMouseCapture, LeaveAlternateScreen);
            original_hook(info);
        }));

        enable_raw_mode().context("failed to enable raw mode")?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).context("failed to enter alternate screen")?;
        if mouse_enabled {
            execute!(stdout, EnableMouseCapture).context("failed to enable mouse capture")?;
        }

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).context("failed to initialize terminal")?;

        let result = self.run_loop(&mut terminal);

        if mouse_enabled {
            let _ = execute!(terminal.backend_mut(), DisableMouseCapture);
        }
        disable_raw_mode().context("failed to disable raw mode")?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)
            .context("failed to leave alternate screen")?;
        terminal.show_cursor().context("failed to restore cursor")?;

        result
    }

    fn run_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        loop {
            if self.needs_full_redraw {
                terminal.clear().context("failed to clear terminal")?;
                self.needs_full_redraw = false;
            }

            terminal
                .draw(|frame| self.render(frame))
                .context("failed to render terminal frame")?;

            if event::poll(Duration::from_millis(200)).context("failed to poll terminal events")? {
                let event = event::read().context("failed to read terminal event")?;
                match event {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        if key.code == KeyCode::Char('c')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            break;
                        }

                        if self.handle_key(key.code)? {
                            break;
                        }
                    }
                    Event::Mouse(mouse) => self.handle_mouse(mouse)?,
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn handle_key(&mut self, code: KeyCode) -> Result<bool> {
        if self.modal.is_some() {
            return self.handle_modal_key(code);
        }

        match code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Down | KeyCode::Char('j') => self.navigator.current_mut().next(),
            KeyCode::Up | KeyCode::Char('k') => self.navigator.current_mut().previous(),
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => self.open_selected()?,
            KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => self.go_back(),
            KeyCode::Char('r') => self.reload_current()?,
            KeyCode::Char('s') => self.shuffle_selected()?,
            KeyCode::Char('?') => self.modal = Some(Modal::Help),
            KeyCode::Char('m') => self.open_context_menu(),
            KeyCode::Char('n') => self.toggle_autoplay(),
            _ => {}
        }

        Ok(false)
    }

    fn handle_modal_key(&mut self, code: KeyCode) -> Result<bool> {
        let Some(modal) = self.modal.as_mut() else {
            return Ok(false);
        };

        match modal {
            Modal::Help => match code {
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => self.modal = None,
                _ => {}
            },
            Modal::Context { menu, actions } => match code {
                KeyCode::Esc | KeyCode::Char('m') | KeyCode::Char('q') => self.modal = None,
                KeyCode::Down | KeyCode::Char('j') => menu.next(),
                KeyCode::Up | KeyCode::Char('k') => menu.previous(),
                KeyCode::Enter => {
                    let action = actions.get(menu.selected).cloned();
                    self.modal = None;
                    if let Some(action) = action {
                        self.apply_context_action(action)?;
                    }
                }
                _ => {}
            },
        }

        Ok(false)
    }

    fn open_context_menu(&mut self) {
        let Some(item) = self.navigator.current().selected_item() else {
            self.status = "Nothing selected.".to_string();
            return;
        };

        let mut entries: Vec<String> = Vec::new();
        let mut actions: Vec<ContextAction> = Vec::new();

        let played = item.user_data.played;
        entries.push(if played {
            "Mark unwatched".to_string()
        } else {
            "Mark watched".to_string()
        });
        actions.push(ContextAction::ToggleWatched(item.id.clone(), played));

        let fav = item.user_data.is_favorite;
        entries.push(if fav {
            "Remove from favorites".to_string()
        } else {
            "Add to favorites".to_string()
        });
        actions.push(ContextAction::ToggleFavorite(item.id.clone(), fav));

        if item.kind == "Series" {
            entries.push("Shuffle episodes".to_string());
            actions.push(ContextAction::Shuffle);
        }

        entries.push(format!(
            "Autoplay next: {}",
            if self.autoplay_next { "on" } else { "off" }
        ));
        actions.push(ContextAction::ToggleAutoplay);

        entries.push("Cancel".to_string());
        actions.push(ContextAction::Close);

        let title = format!("{}  ({})", item.name, item.kind);
        self.modal = Some(Modal::Context {
            menu: Menu::new(title, entries),
            actions,
        });
    }

    fn apply_context_action(&mut self, action: ContextAction) -> Result<()> {
        match action {
            ContextAction::ToggleWatched(id, was_played) => {
                self.session.set_watched(&id, !was_played)?;
                self.status = if was_played {
                    "Marked unwatched.".to_string()
                } else {
                    "Marked watched.".to_string()
                };
                self.reload_current()?;
            }
            ContextAction::ToggleFavorite(id, was_fav) => {
                self.session.set_favorite(&id, !was_fav)?;
                self.status = if was_fav {
                    "Removed from favorites.".to_string()
                } else {
                    "Added to favorites.".to_string()
                };
                self.reload_current()?;
            }
            ContextAction::Shuffle => {
                self.shuffle_selected()?;
            }
            ContextAction::ToggleAutoplay => {
                self.toggle_autoplay();
            }
            ContextAction::Close => {}
        }
        Ok(())
    }

    fn shuffle_selected(&mut self) -> Result<()> {
        let current = self.navigator.current();
        let series_id = if let Some(item) = current.selected_item()
            && item.kind == "Series"
        {
            Some(item.id.clone())
        } else if current.parent_kind.as_deref() == Some("Series") {
            current.parent_id.clone()
        } else {
            None
        };

        let Some(series_id) = series_id else {
            self.status = "Select a show (or open one) to shuffle.".to_string();
            return Ok(());
        };

        let episodes = self.session.fetch_shuffled_episodes(&series_id, 100)?;
        if episodes.is_empty() {
            self.status = "No episodes found to shuffle.".to_string();
            return Ok(());
        }

        let urls: Vec<String> = episodes
            .iter()
            .map(|ep| self.session.playback_url(&ep.id))
            .collect::<Result<_>>()?;

        let auth_token = self.session.auth_token().to_string();
        let mpv_bin = self.config.mpv.bin.as_deref().unwrap_or("mpv");
        let mut command = Command::new(mpv_bin);

        command.arg(format!(
            "--http-header-fields=X-MediaBrowser-Token: {auth_token}"
        ));

        command.args(&self.config.mpv.args);

        if self.config.mpv.ontop {
            command.arg("--ontop");
        }

        let count = urls.len();
        let mut child = command
            .args(urls)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| {
                format!("failed to launch `{mpv_bin}`; set `mpv_bin` in config if needed")
            })?;

        thread::spawn(move || {
            let _ = child.wait();
        });

        self.needs_full_redraw = true;
        self.status = format!("Shuffling {count} episodes in MPV.");
        Ok(())
    }

    fn open_selected(&mut self) -> Result<()> {
        let Some(item) = self.navigator.current().selected_item() else {
            self.status = "Nothing selected.".to_string();
            return Ok(());
        };

        let id = item.id.clone();
        let name = item.name.clone();
        let kind = item.kind.clone();
        let is_folder = item.is_folder;
        let is_playable = item.is_playable();

        if is_folder {
            let items = self.session.fetch_children(&id)?;
            self.navigator
                .push(BrowserState::new(Some(id), Some(kind), name, items));
            self.status = "Loaded folder.".to_string();
            return Ok(());
        }

        if !is_playable {
            self.status = format!("`{name}` is not a playable item.");
            return Ok(());
        }

        // Build playlist. For Episode + autoplay_next, queue this episode and
        // every subsequent one in the series.
        let (urls, queued_after) = if self.autoplay_next && kind == "Episode" {
            self.build_episode_queue(&id, item.series_id.as_deref())?
        } else {
            (vec![self.session.playback_url(&id)?], 0)
        };

        let auth_token = self.session.auth_token().to_string();
        let mpv_bin = self.config.mpv.bin.as_deref().unwrap_or("mpv");
        let mut command = Command::new(mpv_bin);

        command.arg(format!(
            "--http-header-fields=X-MediaBrowser-Token: {auth_token}"
        ));

        command.args(&self.config.mpv.args);

        if self.config.mpv.ontop {
            command.arg("--ontop");
        }

        let mut child = command
            .args(urls)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| {
                format!("failed to launch `{mpv_bin}`; set `mpv_bin` in config if needed")
            })?;

        thread::spawn(move || {
            let _ = child.wait();
        });

        self.needs_full_redraw = true;
        self.status = if queued_after > 0 {
            format!("Playing `{name}` in MPV (+{queued_after} queued).")
        } else {
            format!("Playing `{name}` in MPV.")
        };
        Ok(())
    }

    /// Return (urls, count_queued_after_current). Falls back to a single-URL
    /// playlist if the series lookup fails or yields nothing past the current
    /// episode.
    fn build_episode_queue(
        &self,
        episode_id: &str,
        series_id: Option<&str>,
    ) -> Result<(Vec<String>, usize)> {
        let Some(series_id) = series_id else {
            return Ok((vec![self.session.playback_url(episode_id)?], 0));
        };

        let episodes = match self.session.fetch_series_episodes(series_id) {
            Ok(list) => list,
            Err(_) => return Ok((vec![self.session.playback_url(episode_id)?], 0)),
        };

        let start = episodes
            .iter()
            .position(|e| e.id == episode_id)
            .unwrap_or(0);
        let queue: Vec<_> = episodes.iter().skip(start).collect();
        if queue.is_empty() {
            return Ok((vec![self.session.playback_url(episode_id)?], 0));
        }

        let urls = queue
            .iter()
            .map(|e| self.session.playback_url(&e.id))
            .collect::<Result<Vec<_>>>()?;
        let queued_after = urls.len().saturating_sub(1);
        Ok((urls, queued_after))
    }

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

    fn go_back(&mut self) {
        if self.navigator.pop() {
            self.status = "Returned to previous view.".to_string();
        }
    }

    fn toggle_autoplay(&mut self) {
        self.autoplay_next = !self.autoplay_next;
        self.status = format!(
            "Autoplay next episode: {}.",
            if self.autoplay_next { "on" } else { "off" }
        );
    }

    fn render(&mut self, frame: &mut Frame) {
        let areas = crate::ui::render(
            frame,
            &self.navigator,
            &self.theme,
            &self.status,
            &mut self.list_state,
        );
        self.list_area = areas.list;

        match &self.modal {
            Some(Modal::Help) => crate::ui::menu::render_help(frame, HELP_LINES, &self.theme),
            Some(Modal::Context { menu, .. }) => {
                crate::ui::menu::render_menu(frame, menu, &self.theme);
            }
            None => {}
        }
    }

    fn handle_mouse(&mut self, event: MouseEvent) -> Result<()> {
        if self.modal.is_some() {
            return Ok(());
        }
        let col = event.column;
        let row = event.row;
        let in_list = col >= self.list_area.x
            && col < self.list_area.x + self.list_area.width
            && row >= self.list_area.y
            && row < self.list_area.y + self.list_area.height;

        match event.kind {
            MouseEventKind::ScrollDown if in_list => {
                self.navigator.current_mut().next();
            }
            MouseEventKind::ScrollUp if in_list => {
                self.navigator.current_mut().previous();
            }
            MouseEventKind::Down(MouseButton::Left) if in_list => {
                let offset = self.list_state.offset();
                let row_idx = offset + (row - self.list_area.y) as usize;
                let state = self.navigator.current_mut();
                if row_idx < state.items.len() {
                    let was_selected = state.selected == row_idx;
                    state.selected = row_idx;
                    if was_selected {
                        self.open_selected()?;
                    }
                }
            }
            MouseEventKind::Down(MouseButton::Right) if in_list => {
                let offset = self.list_state.offset();
                let row_idx = offset + (row - self.list_area.y) as usize;
                let state = self.navigator.current_mut();
                if row_idx < state.items.len() {
                    state.selected = row_idx;
                    self.open_context_menu();
                }
            }
            _ => {}
        }

        Ok(())
    }
}
