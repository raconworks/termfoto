use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Size;
use ratatui_image::{
    picker::Picker,
    protocol::Protocol,
    Resize, FilterType,
};

use crate::scanner::ImageEntry;

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Browser,
    Fullscreen,
}

pub struct App {
    pub state: AppState,
    pub images: Vec<ImageEntry>,
    pub selected: usize,
    pub scroll_row: usize,
    pub picker: Picker,
    pub protocol_cache: HashMap<usize, Protocol>,
    pub fullscreen_protocol: Option<Protocol>,
    pub fullscreen_pending: bool,
    pub cache_width: u16,
    pub visible_rows: usize,
    load_tx: Sender<LoadRequest>,
    load_rx: Receiver<(usize, Protocol)>,
}

pub const IMAGES_PER_ROW: usize = 8;
pub const CELL_HEIGHT: usize = 10;

impl App {
    pub fn new(
        images: Vec<ImageEntry>,
        state: AppState,
        picker: Picker,
        load_tx: Sender<LoadRequest>,
        load_rx: Receiver<(usize, Protocol)>,
    ) -> Self {
        Self {
            state,
            images,
            selected: 0,
            scroll_row: 0,
            picker,
            protocol_cache: HashMap::new(),
            fullscreen_protocol: None,
            fullscreen_pending: false,
            cache_width: 0,
            visible_rows: 1,
            load_tx,
            load_rx,
        }
    }

    pub fn navigate_left(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn navigate_right(&mut self) {
        if self.selected + 1 < self.images.len() {
            self.selected += 1;
        }
    }

    pub fn navigate_up(&mut self) {
        self.selected = self.selected.saturating_sub(IMAGES_PER_ROW);
    }

    pub fn navigate_down(&mut self) {
        let next = self.selected + IMAGES_PER_ROW;
        if next < self.images.len() {
            self.selected = next;
        }
    }

    pub fn navigate_home(&mut self) {
        self.selected = 0;
    }

    pub fn navigate_end(&mut self) {
        self.selected = self.images.len().saturating_sub(1);
    }

    pub fn navigate_page_down(&mut self, visible_rows: usize) {
        let step = visible_rows * IMAGES_PER_ROW;
        let next = (self.selected + step).min(self.images.len().saturating_sub(1));
        self.selected = next;
    }

    pub fn navigate_page_up(&mut self, visible_rows: usize) {
        let step = visible_rows * IMAGES_PER_ROW;
        self.selected = self.selected.saturating_sub(step);
    }

    pub fn clamp_scroll(&mut self, visible_rows: usize) {
        let selected_row = self.selected / IMAGES_PER_ROW.max(1);
        if selected_row < self.scroll_row {
            self.scroll_row = selected_row;
        } else if selected_row >= self.scroll_row + visible_rows {
            self.scroll_row = selected_row + 1 - visible_rows;
        }
    }

    pub fn enter_fullscreen(&mut self) {
        if !self.images.is_empty() {
            self.state = AppState::Fullscreen;
            self.fullscreen_protocol = None;
            self.fullscreen_pending = true;
            self.request_load(self.selected, LoadSize::Original);
        }
    }

    pub fn exit_fullscreen(&mut self) {
        self.state = AppState::Browser;
        self.fullscreen_protocol = None;
        self.fullscreen_pending = false;
    }

    pub fn fullscreen_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.fullscreen_protocol = None;
            self.fullscreen_pending = true;
            self.request_load(self.selected, LoadSize::Original);
        }
    }

    pub fn fullscreen_next(&mut self) {
        if self.selected + 1 < self.images.len() {
            self.selected += 1;
            self.fullscreen_protocol = None;
            self.fullscreen_pending = true;
            self.request_load(self.selected, LoadSize::Original);
        }
    }

    /// Check for completed background image loads.
    /// In Browser mode, results go into protocol_cache.
    /// In Fullscreen mode, result for the selected image becomes fullscreen_protocol.
    pub fn collect_loads(&mut self) {
        while let Ok((idx, proto)) = self.load_rx.try_recv() {
            if self.state == AppState::Fullscreen && idx == self.selected {
                self.fullscreen_protocol = Some(proto);
                self.fullscreen_pending = false;
            } else {
                self.protocol_cache.insert(idx, proto);
            }
        }
    }

    pub fn request_load(&self, idx: usize, size: LoadSize) {
        let _ = self.load_tx.send(LoadRequest { idx, size });
    }

    pub fn clear_protocol_cache(&mut self) {
        self.protocol_cache.clear();
        self.cache_width = 0;
    }

    /// Handle a key event. Returns true if the app should quit.
    pub fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        match self.state {
            AppState::Browser => match code {
                KeyCode::Char('q') => return true,
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return true,
                KeyCode::Left => self.navigate_left(),
                KeyCode::Right => self.navigate_right(),
                KeyCode::Up => self.navigate_up(),
                KeyCode::Down => self.navigate_down(),
                KeyCode::PageDown | KeyCode::Char(' ') => {
                    self.navigate_page_down(self.visible_rows)
                }
                KeyCode::PageUp => self.navigate_page_up(self.visible_rows),
                KeyCode::Home => self.navigate_home(),
                KeyCode::End => self.navigate_end(),
                KeyCode::Enter => self.enter_fullscreen(),
                _ => {}
            },
            AppState::Fullscreen => match code {
                KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => self.exit_fullscreen(),
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return true,
                KeyCode::Left => self.fullscreen_prev(),
                KeyCode::Right => self.fullscreen_next(),
                _ => {}
            },
        }
        false
    }
}

/// Size mode for background image loading.
#[derive(Debug, Clone)]
pub enum LoadSize {
    /// Browser thumbnail at fixed cell dimensions.
    Thumbnail { w: u16, h: u16 },
    /// Fullscreen: original image size computed from font metrics.
    Original,
}

/// A request sent to the background loader.
#[derive(Debug, Clone)]
pub struct LoadRequest {
    pub idx: usize,
    pub size: LoadSize,
}

/// Spawn a background thread that loads images and creates chafa-encoded Protocols.
/// Returns (sender, receiver) for App to use.
pub fn spawn_image_loader(
    picker: Picker,
    paths: Vec<std::path::PathBuf>,
) -> (Sender<LoadRequest>, Receiver<(usize, Protocol)>) {
    let (load_tx, load_rx) = std::sync::mpsc::channel::<LoadRequest>();
    let (done_tx, done_rx) = std::sync::mpsc::channel::<(usize, Protocol)>();

    std::thread::spawn(move || {
        while let Ok(req) = load_rx.recv() {
            if let Some(path) = paths.get(req.idx) {
                if let Ok(img) = image::open(path) {
                    let size = match req.size {
                        LoadSize::Thumbnail { w, h } => Size::new(w, h),
                        LoadSize::Original => {
                            let font_size = picker.font_size();
                            let nat_w = img.width().div_ceil(font_size.width as u32) as u16;
                            let nat_h = img.height().div_ceil(font_size.height as u32) as u16;
                            Size::new(nat_w.max(1), nat_h.max(1))
                        }
                    };
                    if let Ok(proto) = picker.new_protocol(
                        img,
                        size,
                        Resize::Fit(Some(FilterType::Lanczos3)),
                    ) {
                        let _ = done_tx.send((req.idx, proto));
                    }
                }
            }
        }
    });

    (load_tx, done_rx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui_image::picker::Picker;
    use std::path::PathBuf;

    fn make_app(count: usize) -> App {
        let images = (0..count)
            .map(|i| ImageEntry {
                path: PathBuf::from(format!("img{:03}.png", i)),
                filename: format!("img{:03}.png", i),
            })
            .collect();
        let (tx, _rx) = std::sync::mpsc::channel::<LoadRequest>();
        let (_tx2, rx2) = std::sync::mpsc::channel::<(usize, Protocol)>();
        App::new(images, AppState::Browser, Picker::halfblocks(), tx, rx2)
    }

    #[test]
    fn test_navigate_right_increments() {
        let mut app = make_app(5);
        app.navigate_right();
        assert_eq!(app.selected, 1);
    }

    #[test]
    fn test_navigate_right_clamps_at_last() {
        let mut app = make_app(3);
        app.selected = 2;
        app.navigate_right();
        assert_eq!(app.selected, 2);
    }

    #[test]
    fn test_navigate_left_decrements() {
        let mut app = make_app(5);
        app.selected = 2;
        app.navigate_left();
        assert_eq!(app.selected, 1);
    }

    #[test]
    fn test_navigate_left_clamps_at_zero() {
        let mut app = make_app(5);
        app.navigate_left();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_navigate_down_skips_row() {
        let mut app = make_app(20);
        app.selected = 1;
        app.navigate_down();
        assert_eq!(app.selected, 9); // 1 + 8
    }

    #[test]
    fn test_navigate_down_clamps() {
        let mut app = make_app(10);
        app.selected = 8;
        app.navigate_down();
        assert_eq!(app.selected, 8); // 8 + 8 = 16 > 9, stays
    }

    #[test]
    fn test_navigate_up_skips_row() {
        let mut app = make_app(20);
        app.selected = 10;
        app.navigate_up();
        assert_eq!(app.selected, 2); // 10 - 8
    }

    #[test]
    fn test_navigate_up_clamps_at_zero() {
        let mut app = make_app(5);
        app.selected = 3;
        app.navigate_up();
        assert_eq!(app.selected, 0); // 3 - 8 < 0
    }

    #[test]
    fn test_navigate_home() {
        let mut app = make_app(5);
        app.selected = 4;
        app.navigate_home();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_navigate_end() {
        let mut app = make_app(5);
        app.navigate_end();
        assert_eq!(app.selected, 4);
    }

    #[test]
    fn test_clear_protocol_cache() {
        let mut app = make_app(5);
        app.cache_width = 80;
        app.clear_protocol_cache();
        assert!(app.protocol_cache.is_empty());
        assert_eq!(app.cache_width, 0);
    }
}
