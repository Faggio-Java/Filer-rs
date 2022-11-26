use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use std::{
    error::Error, io, time::{Duration, Instant}, thread, path::Path, fs::{read_dir, remove_file}, fs,
};

use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Corner, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Gauge, Paragraph, Wrap, Tabs, List, ListItem, ListState},
    Frame, Terminal,
};

struct StatefulList<T> {
    state: ListState,
    files: Vec<T>,
}

impl<T> StatefulList<T> {
    fn with_items(files: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            files,
        }
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.files.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.files.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

struct App {
    files: StatefulList<String>,
    home: String,
    currentdir: String,
    copied: String,
    cname: String,
}

impl App {
    fn new() -> App {
        App {
            files: StatefulList::with_items(vec![("Proccessing".to_owned())]),
            currentdir: "Null".to_owned(),
            home: "Null".to_owned(),
            copied: "Null".to_owned(),
            cname: "Null".to_owned(),
        }
    }

    fn enter(&mut self) {
        let finder = self.files.state.selected().expect("Failed").to_string();
         let i: u32 = finder.parse().unwrap();
          let arg = format!("{}/{}", self.currentdir.clone(), self.files.files.remove(i as usize));
          if &Path::new(&arg).is_dir() == &true {
            self.folder(&Path::new(&arg));
          } else {
            self.folder(&Path::new(&self.currentdir.clone()));
          }
          self.files.next();
    }

    fn back(&mut self) {
        let temp = self.currentdir.clone();
         let temp2 = temp.replace("/", "#/");
         let mut chunks: Vec<_> = temp2.split('#').collect();
         let last = chunks.last().unwrap();

          if last.to_string() == "/home".to_string() {
            self.folder(&Path::new("/"));
          } else if last.to_string() == "/".to_string() {
            self.folder(&Path::new("/"));
          } else {
            chunks.pop();
            let dir = chunks.iter().map(|&s| s.to_string()).collect::<String>();
            self.folder(&Path::new(&dir));
        }
        self.files.next();
    }

    fn delete(&mut self) {
        let finder = self.files.state.selected().expect("Failed").to_string();
         let i: u32 = finder.parse().unwrap();
          let arg = format!("{}/{}", self.currentdir.clone(), self.files.files.remove(i as usize));
        fs::remove_file(&arg)
            .expect("Delete failed");
        self.folder(&Path::new(&self.currentdir.clone()));
        self.files.next();
    }

    fn copy(&mut self) {
        let finder = self.files.state.selected().expect("Failed").to_string();
         let i: u32 = finder.parse().unwrap();
          let arg = format!("{}/{}", self.currentdir.clone(), self.files.files.remove(i as usize));
          let arg2 = format!("{}", arg.replace(&self.currentdir, ""));

        self.copied = arg;
        self.cname = arg2;
        self.folder(&Path::new(&self.currentdir.clone()));
        self.files.next();
    }

    fn paste(&mut self) {
        let arg = format!("{}/{}", self.currentdir.clone(), self.cname.clone());
        if &Path::new(&self.copied).is_file() == &true {
         fs::copy(&self.copied, arg)
             .expect("Copy failed");
        }
        self.folder(&Path::new(&self.currentdir.clone()));
        self.files.next();
    }

    fn setup(&mut self) {
        let home = format!("{}", home::home_dir().expect("Home is null").display());
        self.folder(&Path::new(home.as_str()));
        self.home = home.to_string();
        self.files.next();
    }

    fn folder(&mut self, dir: &Path) {
        let paths = read_dir(dir).unwrap();
        
        let files =
        paths.filter_map(|entry| {
          entry.ok()
            .and_then(|e| e.path().file_name()
            .and_then(|n| n.to_str().map(String::from))
          )
        }).collect::<Vec<String>>();
        self.files = StatefulList::with_items(files);
        self.currentdir = dir.display().to_string();
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
   
    let mut tick_rate = Duration::from_millis(250);
    let mut uprate = Duration::from_millis(500);
    let app = App::new();
    let res = run_app(&mut terminal, app, tick_rate, uprate);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    mut tick_rate: Duration,
    mut uprate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    let mut up_tick = Instant::now();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Down => app.files.next(),
                    KeyCode::Up => app.files.previous(),
                    KeyCode::Right => app.enter(),
                    KeyCode::Left => app.back(),
                    KeyCode::Char('d') => app.delete(),
                    KeyCode::Char('c') => app.copy(),
                    KeyCode::Char('p') => app.paste(),
                    _ => {}
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            //app.tick();
            tick_rate = Duration::from_secs(3600);
        }
        if up_tick.elapsed() >= uprate {
            app.setup();
            uprate = Duration::from_secs(3600);
            up_tick = Instant::now();
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let size = f.size();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(2)
        .constraints(
            [
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ]
            .as_ref(),
        )
        .split(size);

        let files: Vec<ListItem> = app
        .files
        .files
        .iter()
        .map(|i| {
            let log = Spans::from(vec![Span::raw(i)]);

            ListItem::new(vec![
                Spans::from("-".repeat(chunks[1].width as usize)),
                log,
            ])
        })
        .collect();
    
        let viewer = List::new(files)
            .block(Block::default().borders(Borders::ALL))
            .start_corner(Corner::BottomLeft)
            .highlight_style(
                Style::default()
                    .bg(Color::Yellow)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            );
    
        f.render_stateful_widget(viewer, chunks[0], &mut app.files.state);
/*
        let currentsong = format!("Song: {}", app.currentsong);
        let paused = format!("Paused: {}", app.paused);
let text = vec![
    Spans::from(currentsong),
    Spans::from(paused),
];

let create_block = |title| {
    Block::default()
        .borders(Borders::ALL)
        .style(Style::default())
        .title(Span::styled(
            title,
            Style::default().add_modifier(Modifier::BOLD),
        ))
};

let paragraph = Paragraph::new(text.clone())
.style(Style::default())
.block(create_block("Player Info"))
.alignment(Alignment::Left)
.wrap(Wrap { trim: true });
f.render_widget(paragraph, chunks[1]); */
}
