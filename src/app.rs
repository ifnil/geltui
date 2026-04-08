use std::{io, process::Command, thread, time::Duration};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::{
    config::Config,
    jellyfin::{MediaItem, Session},
};

pub struct App {
    config: Config,
    session: Session,
    #[allow(dead_code)] // removed in Task 4 when ui::render reads it
    theme: crate::theme::Theme,
    stack: Vec<BrowserState>,
    status: String,
}

struct BrowserState {
    parent_id: Option<String>,
    parent_kind: Option<String>,
    title: String,
    items: Vec<MediaItem>,
    selected: usize,
}

impl App {
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
            KeyCode::Down | KeyCode::Char('j') => self.current_mut().next(),
            KeyCode::Up | KeyCode::Char('k') => self.current_mut().previous(),
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => self.open_selected()?,
            KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => self.go_back(),
            KeyCode::Char('r') => self.reload_current()?,
            _ => {}
        }

        Ok(false)
    }

    fn open_selected(&mut self) -> Result<()> {
        let Some(item) = self.current().selected_item() else {
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
            self.stack
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

        let mut child = command.arg(url).spawn().with_context(|| {
            format!("failed to launch `{mpv_bin}`; set `mpv_bin` in config if needed")
        })?;

        thread::spawn(move || {
            let _ = child.wait();
        });

        self.status = format!("Playing `{name}` in MPV.");
        Ok(())
    }

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

    fn go_back(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
            self.status = "Returned to previous view.".to_string();
        }
    }

    fn render(&self, frame: &mut Frame) {
        let current = self.current();

        let [header, body, footer_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(8),
                Constraint::Length(4),
            ])
            .areas(frame.area());

        let [list_area, detail_area] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(46), Constraint::Percentage(54)])
            .areas(body);

        let title = Paragraph::new(current.title.clone())
            .block(Block::default().borders(Borders::ALL).title("geltui"))
            .style(Style::default().add_modifier(Modifier::BOLD));
        frame.render_widget(title, header);

        let is_season = current.is_season_view();
        let items: Vec<ListItem> = current
            .items
            .iter()
            .map(|item| {
                let icon = if item.is_folder {
                    ">"
                } else if item.is_playable() {
                    "*"
                } else {
                    "-"
                };
                let label = format_list_label(item, is_season);
                ListItem::new(Line::from(format!("{icon} {label}")))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Browse"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        let mut state = ListState::default();
        if !current.items.is_empty() {
            state.select(Some(current.selected));
        }
        frame.render_stateful_widget(list, list_area, &mut state);

        let details = current
            .selected_item()
            .map(render_details)
            .unwrap_or_else(|| {
                Text::from(vec![
                    Line::from("No items found."),
                    Line::from(""),
                    Line::from("Check the Jellyfin credentials and library visibility."),
                ])
            });

        let detail = Paragraph::new(details)
            .block(Block::default().borders(Borders::ALL).title("Details"))
            .wrap(Wrap { trim: true });
        frame.render_widget(detail, detail_area);

        let footer_text = Text::from(vec![
            Line::from(self.status.clone()),
            Line::from("Enter open/play  h/backspace back  r reload  q quit"),
        ]);
        let footer = Paragraph::new(footer_text)
            .block(Block::default().borders(Borders::ALL).title("Status"));
        frame.render_widget(footer, footer_area);
    }

    fn current(&self) -> &BrowserState {
        self.stack
            .last()
            .expect("app must always keep at least one browser state")
    }

    fn current_mut(&mut self) -> &mut BrowserState {
        self.stack
            .last_mut()
            .expect("app must always keep at least one browser state")
    }
}

impl BrowserState {
    fn new(
        parent_id: Option<String>,
        parent_kind: Option<String>,
        title: String,
        items: Vec<MediaItem>,
    ) -> Self {
        Self::with_selection(parent_id, parent_kind, title, items, 0)
    }

    fn with_selection(
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

    fn is_season_view(&self) -> bool {
        self.parent_kind.as_deref() == Some("Season")
    }

    fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }

        self.selected = (self.selected + 1).min(self.items.len() - 1);
    }

    fn previous(&mut self) {
        if self.items.is_empty() {
            return;
        }

        self.selected = self.selected.saturating_sub(1);
    }

    fn selected_item(&self) -> Option<&MediaItem> {
        self.items.get(self.selected)
    }
}

fn format_list_label(item: &MediaItem, is_season: bool) -> String {
    if is_season
        && let Some(ep) = item.index_number
    {
        return format!("{ep:02} \u{2014} {}", item.name);
    }
    item.name.clone()
}

fn render_details(item: &MediaItem) -> Text<'static> {
    use std::fmt::Write;

    let mut lines = vec![Line::from(item.name.clone())];

    // Series/season line for episodes
    if let Some(series) = &item.series_name {
        let season_ep = match (item.parent_index_number, item.index_number) {
            (Some(s), Some(e)) => format!("{series} \u{2014} Season {s}, Episode {e}"),
            (None, Some(e)) => format!("{series} \u{2014} Episode {e}"),
            _ => series.clone(),
        };
        lines.push(Line::from(season_ep));
    }

    lines.push(Line::from(""));

    // Metadata line: official rating, community rating, year, child count, runtime
    let mut meta = String::new();

    if let Some(rating) = &item.official_rating {
        meta.push_str(rating);
    }

    if let Some(score) = item.community_rating {
        if !meta.is_empty() {
            meta.push_str(" | ");
        }
        write!(meta, "\u{2605} {score:.1}").unwrap();
    }

    if let Some(yr) = item.production_year {
        if !meta.is_empty() {
            meta.push_str(" | ");
        }
        write!(meta, "{yr}").unwrap();
    }

    if let Some(count) = item.child_count {
        if !meta.is_empty() {
            meta.push_str(" | ");
        }
        write!(meta, "{count} items").unwrap();
    }

    if let Some(rt) = item.runtime_ticks {
        if !meta.is_empty() {
            meta.push_str(" | ");
        }
        meta.push_str(&crate::jellyfin::format_runtime(rt));
    }

    if !meta.is_empty() {
        lines.push(Line::from(meta));
    }

    // Collection type (for library folders)
    if let Some(ct) = &item.collection_type {
        lines.push(Line::from(ct.clone()));
    }

    // Genres
    if !item.genres.is_empty() {
        lines.push(Line::from(item.genres.join(", ")));
    }

    lines.push(Line::from(""));

    // Overview
    match item.overview.as_deref() {
        Some(overview) if !overview.trim().is_empty() => {
            lines.push(Line::from(overview.to_string()))
        }
        _ => lines.push(Line::from("No overview available.")),
    }

    Text::from(lines)
}
