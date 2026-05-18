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
    let mut app = App::new(images, initial_state);

    let mut picker = Picker::from_termios().unwrap_or_else(|_| {
        Picker::new((8, 12))
    });

    let mut preview_state: Option<Box<dyn ratatui_image::protocol::StatefulProtocol>> = None;
    let mut last_preview_index: Option<usize> = None;
    let mut last_zoom_factor: f32 = 1.0;

    loop {
        let size = terminal.size()?;
        let visible_rows = (size.height / app::CELL_HEIGHT as u16) as usize;
        app.update_layout(size.width, visible_rows.max(1));

        if app.state == AppState::Preview {
            if last_preview_index != Some(app.selected) || last_zoom_factor != app.zoom_factor {
                if let Some(entry) = app.images.get(app.selected) {
                    match image::open(&entry.path) {
                        Ok(img) => {
                            let font_size = picker.font_size;
                            let area_cols = size.width;
                            let area_rows = size.height.saturating_sub(1);
                            let processed = preprocess_image_for_zoom(
                                img,
                                app.zoom_factor,
                                font_size,
                                area_cols,
                                area_rows,
                            );
                            preview_state = Some(picker.new_resize_protocol(processed));
                        }
                        Err(_) => {
                            preview_state = None;
                        }
                    }
                    last_preview_index = Some(app.selected);
                    last_zoom_factor = app.zoom_factor;
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

fn preprocess_image_for_zoom(
    img: image::DynamicImage,
    zoom: f32,
    font_size: (u16, u16),
    area_cols: u16,
    area_rows: u16,
) -> image::DynamicImage {
    if zoom == 1.0 {
        return img;
    }

    if zoom > 1.0 {
        let vis_w = ((img.width() as f32 / zoom).round() as u32).max(1);
        let vis_h = ((img.height() as f32 / zoom).round() as u32).max(1);
        let x = img.width().saturating_sub(vis_w) / 2;
        let y = img.height().saturating_sub(vis_h) / 2;
        img.crop_imm(x, y, vis_w, vis_h)
    } else {
        use image::{DynamicImage, RgbaImage, imageops};
        let canvas_w = (area_cols as u32 * font_size.0 as u32).max(1);
        let canvas_h = (area_rows as u32 * font_size.1 as u32).max(1);
        let target_w = ((canvas_w as f32 * zoom).round() as u32).max(1);
        let target_h = ((canvas_h as f32 * zoom).round() as u32).max(1);
        let scaled = img.resize(target_w, target_h, imageops::FilterType::Lanczos3);
        let mut canvas = RgbaImage::from_pixel(canvas_w, canvas_h, image::Rgba([0, 0, 0, 255]));
        let paste_x = canvas_w.saturating_sub(scaled.width()) / 2;
        let paste_y = canvas_h.saturating_sub(scaled.height()) / 2;
        imageops::overlay(&mut canvas, &scaled.to_rgba8(), paste_x as i64, paste_y as i64);
        DynamicImage::ImageRgba8(canvas)
    }
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
            KeyCode::Char('+') | KeyCode::Char('=') if modifiers.contains(KeyModifiers::CONTROL) => app.zoom_in(),
            KeyCode::Char('-') if modifiers.contains(KeyModifiers::CONTROL) => app.zoom_out(),
            KeyCode::Char('0') if modifiers.contains(KeyModifiers::CONTROL) => app.zoom_reset(),
            _ => {}
        },
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};
    use crate::scanner::ImageEntry;
    use std::path::PathBuf;

    fn make_preview_app() -> App {
        let images = vec![ImageEntry {
            path: PathBuf::from("test.png"),
            filename: "test.png".to_string(),
            thumbnail: None,
        }];
        App::new(images, AppState::Preview)
    }

    #[test]
    fn preprocess_zoom_1_returns_same_dimensions() {
        let img = image::DynamicImage::new_rgba8(100, 80);
        let result = preprocess_image_for_zoom(img, 1.0, (8, 12), 80, 24);
        assert_eq!(result.width(), 100);
        assert_eq!(result.height(), 80);
    }

    #[test]
    fn preprocess_zoom_in_crops_to_fraction() {
        let img = image::DynamicImage::new_rgba8(100, 80);
        // zoom=2.0 → vis_w=50, vis_h=40
        let result = preprocess_image_for_zoom(img, 2.0, (8, 12), 80, 24);
        assert_eq!(result.width(), 50);
        assert_eq!(result.height(), 40);
    }

    #[test]
    fn preprocess_zoom_out_produces_canvas_size() {
        let img = image::DynamicImage::new_rgba8(100, 80);
        // canvas = area_cols * font_w × area_rows * font_h = 40*8 × 12*12 = 320×144
        let result = preprocess_image_for_zoom(img, 0.5, (8, 12), 40, 12);
        assert_eq!(result.width(), 320);
        assert_eq!(result.height(), 144);
    }

    #[test]
    fn preprocess_zoom_in_is_centered_crop() {
        // Create 100x80 image where center pixel is red
        let mut img = image::RgbaImage::new(100, 80);
        img.put_pixel(50, 40, image::Rgba([255, 0, 0, 255]));
        let dyn_img = image::DynamicImage::ImageRgba8(img);
        // zoom=2.0 → crop 50x40 centered at (25, 20)..(75, 60)
        // Center pixel (50,40) in original → (25,20) in cropped
        let result = preprocess_image_for_zoom(dyn_img, 2.0, (8, 12), 80, 24);
        let rgba = result.to_rgba8();
        assert_eq!(rgba.get_pixel(25, 20), &image::Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn ctrl_plus_zooms_in() {
        let mut app = make_preview_app();
        let before = app.zoom_factor;
        handle_key(&mut app, KeyCode::Char('+'), KeyModifiers::CONTROL, 1);
        assert!(app.zoom_factor > before, "zoom_factor should increase");
    }

    #[test]
    fn ctrl_equals_also_zooms_in() {
        let mut app = make_preview_app();
        let before = app.zoom_factor;
        handle_key(&mut app, KeyCode::Char('='), KeyModifiers::CONTROL, 1);
        assert!(app.zoom_factor > before, "Ctrl+= should also zoom in");
    }

    #[test]
    fn ctrl_minus_zooms_out() {
        let mut app = make_preview_app();
        let before = app.zoom_factor;
        handle_key(&mut app, KeyCode::Char('-'), KeyModifiers::CONTROL, 1);
        assert!(app.zoom_factor < before, "zoom_factor should decrease");
    }

    #[test]
    fn ctrl_zero_resets_zoom() {
        let mut app = make_preview_app();
        app.zoom_factor = 3.0;
        handle_key(&mut app, KeyCode::Char('0'), KeyModifiers::CONTROL, 1);
        assert_eq!(app.zoom_factor, 1.0);
    }
}
