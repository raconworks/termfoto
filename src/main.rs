mod app;
mod favorites;
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
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use ratatui_image::picker::Picker;

use app::{spawn_image_loader, App, AppStart, AppState, LoadControl, MIN_CELL};
use lang::Lang;
use scanner::scan_directory;
use ui::browser::populate_protocol_cache;
use ui::layout::gallery_inner_size;

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

    let (images, image_dir, initial_state, selected) = match path {
        None => {
            let image_dir = std::env::current_dir()?;
            let images = scan_directory(&image_dir)?;
            (images, image_dir, AppState::Browser, 0_usize)
        }
        Some(ref p) if p.is_dir() => {
            let image_dir = absolute_path(p)?;
            let images = scan_directory(&image_dir)?;
            (images, image_dir, AppState::Browser, 0_usize)
        }
        Some(ref p) if p.is_file() && scanner::is_supported_image(p) => {
            let parent = p.parent().unwrap_or_else(|| Path::new("."));
            let image_dir = absolute_path(parent)?;
            let images = scan_directory(&image_dir)?;
            // Find the index of the specified file in the scanned list.
            // scan_directory normalises filenames so we compare by filename.
            let target_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let selected = images
                .iter()
                .position(|e| e.filename == target_name)
                .unwrap_or(0);
            (images, image_dir, AppState::Fullscreen, selected)
        }
        Some(ref p) => {
            eprintln!(
                "termfoto: '{}' is not a supported image or directory",
                p.display()
            );
            std::process::exit(1);
        }
    };

    let _term = TermGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    run(&mut terminal, images, image_dir, initial_state, selected)
}

fn absolute_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
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
    image_dir: PathBuf,
    initial_state: AppState,
    selected: usize,
) -> Result<()> {
    let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());

    // Spawn background image loader: opens images + creates chafa Protocols
    let paths: Vec<PathBuf> = images.iter().map(|e| e.path.clone()).collect();
    let load_control = LoadControl::new();
    let (load_tx, load_rx) = spawn_image_loader(picker.clone(), paths, load_control.clone());

    let mut app = App::new_with_load_control(
        AppStart {
            images,
            image_dir,
            state: initial_state,
            selected,
        },
        load_tx,
        load_rx,
        Lang::detect(),
        picker.clone(),
        load_control,
    );

    loop {
        let size = terminal.size()?;
        // Dynamic grid: visually square cells (终端字符 ≈ 1:2 宽高比)
        let gallery_size = gallery_inner_size(size);
        let avail_h = gallery_size.height.max(1);
        let avail_w = gallery_size.width.max(1);
        let char_ratio = picker.font_size().height as f32 / picker.font_size().width as f32;
        let cols = if avail_w >= MIN_CELL * 2 {
            (avail_w / MIN_CELL) as usize
        } else {
            1
        };
        let cell_w = (avail_w / cols as u16).max(1);
        // Visual square: cell_h ≈ cell_w / char_ratio
        let cell_h = ((cell_w as f32 / char_ratio) as u16).max(1);
        let rows = (avail_h / cell_h).max(1) as usize;
        let cell_h = avail_h / rows as u16;

        app.set_grid_layout(cols, rows);

        if app.state == AppState::Browser {
            populate_protocol_cache(&mut app, cell_w, cell_h, size);
        }

        // Check for completed background image loads
        app.collect_loads();
        app.collect_render_results();
        app.advance_animation(Instant::now());

        // Submit async render work for any settled fullscreen state known before draw.
        app.drive_render_queue(Instant::now());

        // Render
        terminal.draw(|f| ui::draw(f, &mut app, cell_w, cell_h))?;

        // Preview rendering records its viewport during draw; submit work after that too.
        app.drive_render_queue(Instant::now());

        let now = Instant::now();
        let animation_timeout = app
            .next_animation_deadline()
            .map(|deadline| deadline.saturating_duration_since(now));
        let render_timeout = app
            .next_render_deadline()
            .map(|deadline| deadline.saturating_duration_since(now));
        let poll_timeout = animation_timeout
            .into_iter()
            .chain(render_timeout)
            .min()
            .unwrap_or_else(|| Duration::from_millis(50))
            .min(Duration::from_millis(50));

        if event::poll(poll_timeout)? {
            let mut should_quit = false;
            // Drain all pending events so rapid zoom/pan keystrokes are
            // batched into a single protocol regeneration.
            loop {
                if let Event::Key(key) = event::read()? {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    if app.handle_key(key.code, key.modifiers) {
                        should_quit = true;
                    }
                }
                // No more events queued
                if should_quit || !event::poll(Duration::ZERO)? {
                    break;
                }
            }
            if should_quit {
                break;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_path_expands_relative_path_from_current_dir() {
        let current = std::env::current_dir().unwrap();

        let path = absolute_path(Path::new("photos")).unwrap();

        assert_eq!(path, current.join("photos"));
        assert!(path.is_absolute());
    }

    #[test]
    fn absolute_path_keeps_absolute_path() {
        let path = PathBuf::from("/tmp/termfoto");

        assert_eq!(absolute_path(&path).unwrap(), path);
    }
}
