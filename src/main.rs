mod app;
mod lang;
mod scanner;
mod ui;

/// RAII guard that restores terminal state on drop.
struct TermGuard {
    _stdout: std::io::Stdout,
}

impl TermGuard {
    fn enter() -> anyhow::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        Ok(Self { _stdout: stdout })
    }
}

impl Drop for TermGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use ratatui_image::picker::Picker;

use app::{spawn_image_loader, App, AppState, LOGO_HEIGHT, MIN_CELL, MIN_LOGO_WIDTH};
use lang::Lang;
use scanner::scan_directory;
use ui::browser::populate_protocol_cache;

fn main() -> Result<()> {
    let arg1 = std::env::args().nth(1);

    // Handle --help / --version before doing any I/O
    match arg1.as_deref() {
        Some("-h") | Some("--help") => {
            print_help();
            std::process::exit(0);
        }
        Some("-V") | Some("--version") => {
            println!("termfoto {}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }
        _ => {}
    }

    let path = arg1.map(PathBuf::from);

    let (images, initial_state) = match path {
        None => {
            let images = scan_directory(&std::env::current_dir()?)?;
            (images, AppState::Browser)
        }
        Some(ref p) if p.is_dir() => {
            let images = scan_directory(p)?;
            (images, AppState::Browser)
        }
        Some(ref p) if p.is_file() && scanner::is_supported_image(p) => {
            let file_size = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
            let entry = scanner::ImageEntry {
                path: p.clone(),
                filename: p
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                file_size,
            };
            (vec![entry], AppState::Fullscreen)
        }
        Some(ref p) => {
            eprintln!(
                "termfoto: '{}' is not a supported image or directory",
                p.display()
            );
            std::process::exit(1);
        }
    };

    if images.is_empty() && matches!(initial_state, AppState::Browser) {
        eprintln!("termfoto: no images found in the specified directory");
        std::process::exit(0);
    }

    let _term = TermGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    run(&mut terminal, images, initial_state)
}

fn print_help() {
    println!("termfoto — fast terminal photo viewer\n");
    println!("Usage: termfoto [PATH]\n");
    println!("  <PATH>    image file or directory (default: current directory)\n");
    println!("Options:");
    println!("  -h, --help        show this help");
    println!("  -V, --version     show version");
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

    let mut app = App::new(images, initial_state, load_tx, load_rx, Lang::detect());

    loop {
        let size = terminal.size()?;
        // Dynamic grid: visually square cells (终端字符 ≈ 1:2 宽高比)
        let logo_h = if size.width >= MIN_LOGO_WIDTH {
            LOGO_HEIGHT
        } else {
            0
        };
        let avail_h = size.height.saturating_sub(logo_h + 1); // +1 status bar
        let char_ratio = picker.font_size().height as f32 / picker.font_size().width as f32;
        let cols = (size.width / MIN_CELL).max(2) as usize;
        let cell_w = size.width / cols as u16;
        // Visual square: cell_h ≈ cell_w / char_ratio
        let cell_h = ((cell_w as f32 / char_ratio) as u16).max(1);
        let rows = (avail_h / cell_h).max(1) as usize;
        let cell_h = avail_h / rows as u16;

        app.grid_cols = cols;
        app.visible_rows = rows;

        if app.state == AppState::Browser {
            populate_protocol_cache(&mut app, cell_w, cell_h, size);
        }

        // Check for completed background image loads
        app.collect_loads();

        // Render
        terminal.draw(|f| ui::draw(f, &mut app, cell_w, cell_h))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                let should_quit = app.handle_key(key.code, key.modifiers);
                if should_quit {
                    break;
                }
            }
        }
    }

    Ok(())
}
