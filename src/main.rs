mod app;
mod scanner;
mod ui;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use ratatui_image::picker::Picker;

use app::{App, AppState};
use scanner::scan_directory;

#[derive(Parser)]
#[command(name = "darkroom", about = "Terminal image viewer", version)]
struct Args {
    /// Directory or image file to open (default: current directory)
    path: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let (images, initial_state) = match args.path {
        None => {
            let images = scan_directory(&std::env::current_dir()?)?;
            (images, AppState::Grid)
        }
        Some(ref p) if p.is_dir() => {
            let images = scan_directory(p)?;
            (images, AppState::Grid)
        }
        Some(ref p) if p.is_file() && scanner::is_supported_image(p) => {
            let entry = scanner::ImageEntry {
                path: p.clone(),
                filename: p.file_name().unwrap_or_default().to_string_lossy().into_owned(),
                thumbnail: None,
            };
            (vec![entry], AppState::Preview)
        }
        Some(ref p) => {
            eprintln!("darkroom: '{}' is not a supported image or directory", p.display());
            std::process::exit(1);
        }
    };

    if images.is_empty() && matches!(initial_state, AppState::Grid) {
        eprintln!("darkroom: no images found in the specified directory");
        std::process::exit(0);
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    if let Err(e) = execute!(stdout, EnterAlternateScreen) {
        let _ = disable_raw_mode();
        return Err(e.into());
    }
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, images, initial_state);

    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);

    result
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    images: Vec<scanner::ImageEntry>,
    initial_state: AppState,
) -> Result<()> {
    let mut app = App::new(images, initial_state);

    let mut picker = Picker::from_termios().unwrap_or_else(|_| {
        Picker::new((8, 12))
    });

    let mut preview_state: Option<Box<dyn ratatui_image::protocol::StatefulProtocol>> = None;
    let mut last_preview_index: Option<usize> = None;

    loop {
        let size = terminal.size()?;
        let visible_rows = (size.height / app::CELL_HEIGHT as u16) as usize;
        app.update_layout(size.width, visible_rows.max(1));

        if app.state == AppState::Preview {
            if last_preview_index != Some(app.selected) {
                if let Some(entry) = app.images.get(app.selected) {
                    if let Ok(img) = image::open(&entry.path) {
                        preview_state = Some(picker.new_resize_protocol(img));
                        last_preview_index = Some(app.selected);
                    }
                }
            }
        }

        let ps = preview_state.as_mut();
        terminal.draw(|f| ui::draw(f, &mut app, ps))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                let should_quit = handle_key(&mut app, key.code, key.modifiers, visible_rows);
                if should_quit {
                    break;
                }
            }
        }
    }

    Ok(())
}

fn handle_key(
    app: &mut App,
    code: KeyCode,
    modifiers: KeyModifiers,
    visible_rows: usize,
) -> bool {
    match app.state {
        AppState::Grid => match code {
            KeyCode::Char('q') => return true,
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return true,
            KeyCode::Left => app.navigate_left(),
            KeyCode::Right => app.navigate_right(),
            KeyCode::Up => app.navigate_up(),
            KeyCode::Down => app.navigate_down(),
            KeyCode::PageDown | KeyCode::Char(' ') => app.navigate_page_down(visible_rows),
            KeyCode::PageUp => app.navigate_page_up(visible_rows),
            KeyCode::Home => app.navigate_home(),
            KeyCode::End => app.navigate_end(),
            KeyCode::Enter => app.enter_preview(),
            _ => {}
        },
        AppState::Preview => match code {
            KeyCode::Char('q') | KeyCode::Esc => app.exit_preview(),
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return true,
            KeyCode::Left => app.preview_prev(),
            KeyCode::Right => app.preview_next(),
            _ => {}
        },
    }
    false
}
