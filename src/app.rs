use std::{io, process::Command, time::Duration};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
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
    stack: Vec<BrowserState>,
    status: String,
}

struct BrowserState {
    parent_id: Option<String>,
    title: String,
    items: Vec<MediaItem>,
    selected: usize,
}

impl App {
    pub fn new(config: Config, session: Session) -> Result<Self> {
        let root = session.fetch_root()?;
        Ok(Self {
            config,
            session,
            stack: vec![BrowserState::new(None, "Libraries".to_string(), root)],
            status: "Connected to Jellyfin. Press Enter to open or play.".to_string(),
        })
    }

    pub fn run(mut self) -> Result<()> {
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
                    && self.handle_key(key.code)?
                {
                    break;
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
        let Some(item) = self.current().selected_item().cloned() else {
            self.status = "Nothing selected.".to_string();
            return Ok(());
        };

        if item.is_folder {
            let title = item.name.clone();
            let items = self.session.fetch_children(&item.id)?;
            self.stack
                .push(BrowserState::new(Some(item.id.clone()), title, items));
            self.status = "Loaded folder.".to_string();
            return Ok(());
        }

        if !item.is_playable() {
            self.status = format!("`{}` is not a playable item.", item.name);
            return Ok(());
        }

        let url = self.session.playback_url(&item.id)?;
        let mpv_bin = self.config.mpv_bin.as_deref().unwrap_or("mpv");
        let mut command = Command::new(mpv_bin);

        if let Some(extra_args) = self.config.mpv_args.as_deref() {
            command.args(extra_args);
        }

        command.arg(url).spawn().with_context(|| {
            format!("failed to launch `{mpv_bin}`; set `mpv_bin` in config if needed")
        })?;

        self.status = format!("Playing `{}` in MPV.", item.name);
        Ok(())
    }

    fn reload_current(&mut self) -> Result<()> {
        let refreshed = if self.stack.len() == 1 {
            self.session.fetch_root()?
        } else {
            match self.current().parent_id.as_deref() {
                Some(parent_id) => self.session.fetch_children(parent_id)?,
                None => self.session.fetch_root()?,
            }
        };

        let parent_id = self.current().parent_id.clone();
        let title = self.current().title.clone();
        let selected = self.current().selected;
        *self.current_mut() = BrowserState::with_selection(parent_id, title, refreshed, selected);
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

        let title = Paragraph::new(self.current().title.clone())
            .block(Block::default().borders(Borders::ALL).title("geltui"))
            .style(Style::default().add_modifier(Modifier::BOLD));
        frame.render_widget(title, header);

        let items: Vec<ListItem> = self
            .current()
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
                ListItem::new(Line::from(format!("{icon} {}", item.name)))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Browse"))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        let mut state = ListState::default();
        if !self.current().items.is_empty() {
            state.select(Some(self.current().selected));
        }
        frame.render_stateful_widget(list, list_area, &mut state);

        let details = self
            .current()
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
    fn new(parent_id: Option<String>, title: String, items: Vec<MediaItem>) -> Self {
        Self::with_selection(parent_id, title, items, 0)
    }

    fn with_selection(
        parent_id: Option<String>,
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
            title,
            items,
            selected,
        }
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

fn render_details(item: &MediaItem) -> Text<'static> {
    let mut lines = vec![
        Line::from(item.name.clone()),
        Line::from(item.secondary_label()),
        Line::from(""),
    ];

    match item.overview.as_deref() {
        Some(overview) if !overview.trim().is_empty() => {
            lines.push(Line::from(overview.to_string()))
        }
        _ => lines.push(Line::from("No overview available.")),
    }

    Text::from(lines)
}
