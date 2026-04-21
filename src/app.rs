use std::{
    io,
    process::{Command, Stdio},
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame,
    Terminal,
    backend::CrosstermBackend,
};

use crate::{
    config::Config,
    jellyfin::Session,
    state::{BrowserState, Navigator},
};

pub struct App {
    config: Config,
    session: Session,
    theme: crate::theme::Theme,
    navigator: Navigator,
    status: String,
}

impl App {
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

    pub fn run(mut self) -> Result<()> {
        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
            original_hook(info);
        }));

        enable_raw_mode().context("failed to enable raw mode")?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).context("failed to enter alternate screen")?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).context("failed to initialize terminal")?;

        let result = self.run_loop(&mut terminal);

        disable_raw_mode().context("failed to disable raw mode")?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)
            .context("failed to leave alternate screen")?;
        terminal.show_cursor().context("failed to restore cursor")?;

        result
    }

    fn run_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        loop {
            terminal
                .draw(|frame| self.render(frame))
                .context("failed to render terminal frame")?;

            if event::poll(Duration::from_millis(200)).context("failed to poll terminal events")? {
                let event = event::read().context("failed to read terminal event")?;
                if let Event::Key(key) = event
                    && key.kind == KeyEventKind::Press
                {
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        break;
                    }

                    if self.handle_key(key.code)? {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_key(&mut self, code: KeyCode) -> Result<bool> {
        match code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Down | KeyCode::Char('j') => self.navigator.current_mut().next(),
            KeyCode::Up | KeyCode::Char('k') => self.navigator.current_mut().previous(),
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => self.open_selected()?,
            KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => self.go_back(),
            KeyCode::Char('r') => self.reload_current()?,
            KeyCode::Char('s') => self.shuffle_selected()?,
            _ => {}
        }

        Ok(false)
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
        let mpv_bin = self.config.mpv_bin.as_deref().unwrap_or("mpv");
        let mut command = Command::new(mpv_bin);

        command.arg(format!(
            "--http-header-fields=X-MediaBrowser-Token: {auth_token}"
        ));

        if let Some(extra_args) = self.config.mpv_args.as_deref() {
            command.args(extra_args);
        }

        if self.config.mpv_ontop {
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

        let url = self.session.playback_url(&id)?;
        let auth_token = self.session.auth_token().to_string();
        let mpv_bin = self.config.mpv_bin.as_deref().unwrap_or("mpv");
        let mut command = Command::new(mpv_bin);

        command.arg(format!(
            "--http-header-fields=X-MediaBrowser-Token: {auth_token}"
        ));

        if let Some(extra_args) = self.config.mpv_args.as_deref() {
            command.args(extra_args);
        }

        if self.config.mpv_ontop {
            command.arg("--ontop");
        }

        let mut child = command
            .arg(url)
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

        self.status = format!("Playing `{name}` in MPV.");
        Ok(())
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

    fn render(&self, frame: &mut Frame) {
        crate::ui::render(frame, &self.navigator, &self.theme, &self.status);
    }
}
