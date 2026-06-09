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

use app::{spawn_image_loader, App, AppState, CELL_HEIGHT, IMAGES_PER_ROW};
use scanner::scan_directory;
use ui::browser::populate_protocol_cache;

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
            (images, AppState::Browser)
        }
        Some(ref p) if p.is_dir() => {
            let images = scan_directory(p)?;
            (images, AppState::Browser)
        }
        Some(ref p) if p.is_file() && scanner::is_supported_image(p) => {
            let entry = scanner::ImageEntry {
                path: p.clone(),
                filename: p.file_name().unwrap_or_default().to_string_lossy().into_owned(),
            };
            (vec![entry], AppState::Fullscreen)
        }
        Some(ref p) => {
            eprintln!(
                "darkroom: '{}' is not a supported image or directory",
                p.display()
            );
            std::process::exit(1);
        }
    };

    if images.is_empty() && matches!(initial_state, AppState::Browser) {
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
    let mut terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(e) => {
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
            return Err(e.into());
        }
    };

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
    let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());

    // Spawn background image loader: opens images + creates chafa Protocols
    let paths: Vec<PathBuf> = images.iter().map(|e| e.path.clone()).collect();
    let (load_tx, load_rx) = spawn_image_loader(picker.clone(), paths);

    let mut app = App::new(images, initial_state, picker, load_tx, load_rx);

    let mut last_terminal_width: u16 = 0;

    loop {
        let size = terminal.size()?;
        let visible_rows = (size.height / CELL_HEIGHT as u16) as usize;
        let cell_w = (size.width / IMAGES_PER_ROW as u16).max(1);
        let cell_h = CELL_HEIGHT as u16;

        if size.width != last_terminal_width {
            app.clear_protocol_cache();
            last_terminal_width = size.width;
        }

        if app.state == AppState::Browser {
            populate_protocol_cache(&mut app, cell_w, cell_h, size.width, visible_rows.max(1));
        }

        // Check for completed background image loads
        app.collect_loads();

        // Render (take protocol out to avoid borrow conflict)
        let proto = app.fullscreen_protocol.take();
        let proto_ref = proto.as_ref();
        terminal.draw(|f| ui::draw(f, &mut app, cell_w, cell_h, proto_ref))?;
        app.fullscreen_protocol = proto;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                let should_quit = handle_key(&mut app, key.code, key.modifiers, visible_rows.max(1));
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
        AppState::Browser => match code {
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
            KeyCode::Enter => app.enter_fullscreen(),
            _ => {}
        },
        AppState::Fullscreen => match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => app.exit_fullscreen(),
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return true,
            KeyCode::Left => app.fullscreen_prev(),
            KeyCode::Right => app.fullscreen_next(),
            _ => {}
        },
    }
    false
}
