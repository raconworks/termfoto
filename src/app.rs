use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, Sender};

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Size;
use ratatui_image::{
    picker::Picker,
    protocol::Protocol,
    Resize, FilterType,
};

use crate::lang::Lang;
use crate::scanner::ImageEntry;
use crate::ui::search::{SearchAction, SearchState};

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
    pub protocol_cache: HashMap<usize, Protocol>,
    pub fullscreen_protocol: Option<Protocol>,
    pub fullscreen_pending: bool,
    pub cache_width: u16,
    pub cache_height: u16,
    pub grid_cols: usize,
    pub thumb_w: u16,
    pub thumb_h: u16,
    pub visible_rows: usize,
    pub requested: HashSet<usize>,
    pub search: Option<SearchState>,
    pub lang: Lang,
    load_tx: Sender<LoadRequest>,
    load_rx: Receiver<(usize, Protocol)>,
}

pub const MIN_CELL: u16 = 24;
pub const LOGO_HEIGHT: u16 = 6;
pub const MIN_LOGO_WIDTH: u16 = 70;
const MAX_CACHE_SIZE: usize = 200;

impl App {
    pub fn new(
        images: Vec<ImageEntry>,
        state: AppState,
        load_tx: Sender<LoadRequest>,
        load_rx: Receiver<(usize, Protocol)>,
        lang: Lang,
    ) -> Self {
        Self {
            state,
            images,
            selected: 0,
            scroll_row: 0,
            protocol_cache: HashMap::new(),
            fullscreen_protocol: None,
            fullscreen_pending: false,
            cache_width: 0,
            cache_height: 0,
            grid_cols: 8,
            thumb_w: 0,
            thumb_h: 0,
            visible_rows: 1,
            requested: HashSet::new(),
            search: None,
            lang,
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
        self.selected = self.selected.saturating_sub(self.grid_cols);
    }

    pub fn navigate_down(&mut self) {
        let next = self.selected + self.grid_cols;
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
        let step = visible_rows * self.grid_cols;
        let next = (self.selected + step).min(self.images.len().saturating_sub(1));
        self.selected = next;
    }

    pub fn navigate_page_up(&mut self, visible_rows: usize) {
        let step = visible_rows * self.grid_cols;
        self.selected = self.selected.saturating_sub(step);
    }

    pub fn clamp_scroll(&mut self, visible_rows: usize) {
        let selected_row = self.selected / self.grid_cols.max(1);
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
            self.requested.remove(&idx);
            if self.state == AppState::Fullscreen && idx == self.selected {
                self.fullscreen_protocol = Some(proto);
                self.fullscreen_pending = false;
            } else {
                // Discard protocols that exceed current cell (stale from terminal resize)
                let psize = proto.size();
                if self.thumb_w > 0 && (psize.width > self.thumb_w || psize.height > self.thumb_h) {
                    continue;
                }
                self.insert_cache(idx, proto);
            }
        }
    }

    fn insert_cache(&mut self, idx: usize, proto: Protocol) {
        self.protocol_cache.insert(idx, proto);
        if self.protocol_cache.len() > MAX_CACHE_SIZE {
            // Evict the oldest MAX_CACHE_SIZE/2 entries (HashMap preserves insertion order)
            let remove_count = MAX_CACHE_SIZE / 2;
            let stale: Vec<usize> = self
                .protocol_cache
                .keys()
                .take(remove_count)
                .copied()
                .collect();
            for k in stale {
                self.protocol_cache.remove(&k);
            }
        }
    }

    pub fn request_load(&mut self, idx: usize, size: LoadSize) {
        if self.requested.contains(&idx) {
            return;
        }
        self.requested.insert(idx);
        let _ = self.load_tx.send(LoadRequest { idx, size });
    }

    pub fn clear_protocol_cache(&mut self) {
        self.protocol_cache.clear();
        self.requested.clear();
        self.cache_width = 0;
    }

    /// Handle a key event. Returns true if the app should quit.
    pub fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        match self.state {
            AppState::Browser => {
                // In search mode, delegate to search handler
                if self.search.is_some() {
                    return self.handle_search_key(code, modifiers);
                }

                match code {
                    KeyCode::Char('q') => return true,
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return true,
                    KeyCode::Char('L') | KeyCode::Char('l') => {
                        self.lang.toggle();
                    }
                    KeyCode::Char('/') | KeyCode::Char('\\') => {
                        let trigger = match code {
                            KeyCode::Char(c) => c,
                            _ => '/',
                        };
                        self.search = Some(SearchState::new(self.selected, trigger));
                        return false;
                    }
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
                }
            }
            AppState::Fullscreen => match code {
                KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => self.exit_fullscreen(),
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return true,
                KeyCode::Char('L') | KeyCode::Char('l') => {
                    self.lang.toggle();
                }
                KeyCode::Left => self.fullscreen_prev(),
                KeyCode::Right => self.fullscreen_next(),
                _ => {}
            },
        }
        false
    }

    fn handle_search_key(&mut self, code: KeyCode, _modifiers: KeyModifiers) -> bool {
        // Enter in search mode: open fullscreen for current match
        if code == KeyCode::Enter {
            self.search = None;
            self.enter_fullscreen();
            return false;
        }

        let search = self.search.as_mut().unwrap();
        match search.handle_key(code, _modifiers, &self.images) {
            SearchAction::JumpTo(idx) => {
                self.selected = idx;
                self.clamp_scroll(self.visible_rows.max(1));
                false
            }
            SearchAction::Cancel => {
                self.selected = self.search.as_ref().unwrap().saved_selected;
                self.clamp_scroll(self.visible_rows.max(1));
                self.search = None;
                false
            }
            SearchAction::Continue => false,
        }
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

/// Spawn a background thread pool that loads images and creates Protocols in parallel.
/// Returns (sender, receiver) for App to use.
pub fn spawn_image_loader(
    picker: Picker,
    paths: Vec<std::path::PathBuf>,
) -> (Sender<LoadRequest>, Receiver<(usize, Protocol)>) {
    let (load_tx, load_rx) = std::sync::mpsc::channel::<LoadRequest>();
    let (done_tx, done_rx) = std::sync::mpsc::channel::<(usize, Protocol)>();
    let paths = std::sync::Arc::new(paths);
    let rx = std::sync::Arc::new(std::sync::Mutex::new(load_rx));

    const WORKERS: usize = 4;
    for _ in 0..WORKERS {
        let picker = picker.clone();
        let paths = std::sync::Arc::clone(&paths);
        let done_tx = done_tx.clone();
        let rx = std::sync::Arc::clone(&rx);

        std::thread::spawn(move || loop {
            // Lock only for receiving; release during processing
            let req = {
                let rx = rx.lock().unwrap();
                match rx.recv() {
                    Ok(req) => req,
                    Err(_) => return, // Sender dropped, exit worker
                }
            };

            if let Some(path) = paths.get(req.idx) {
                if let Ok(img) = image::open(path) {
                    let font_size = picker.font_size();
                    let (img, size, filter) = match req.size {
                        LoadSize::Thumbnail { w, h } => {
                            let pixel_w = w as u32 * font_size.width as u32 * 2;
                            let pixel_h = h as u32 * font_size.height as u32 * 2;
                            let thumb = img.thumbnail(pixel_w, pixel_h);
                            let size = Size::new(w, h);
                            (thumb, size, FilterType::Nearest)
                        }
                        LoadSize::Original => {
                            let nat_w = img.width().div_ceil(font_size.width as u32) as u16;
                            let nat_h = img.height().div_ceil(font_size.height as u32) as u16;
                            let size = Size::new(nat_w.max(1), nat_h.max(1));
                            (img, size, FilterType::Lanczos3)
                        }
                    };
                    if let Ok(proto) = picker.new_protocol(
                        img,
                        size,
                        Resize::Fit(Some(filter)),
                    ) {
                        let _ = done_tx.send((req.idx, proto));
                    }
                }
            }
        });
    }

    (load_tx, done_rx)
}

#[cfg(test)]
mod tests {
    use super::*;
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
        App::new(images, AppState::Browser, tx, rx2, Lang::Zh)
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

    // ---- Search tests ----

    fn make_app_with_names(names: &[&str]) -> App {
        let images: Vec<ImageEntry> = names
            .iter()
            .map(|name| ImageEntry {
                path: PathBuf::from(name),
                filename: name.to_string(),
            })
            .collect();
        let (tx, _rx) = std::sync::mpsc::channel::<LoadRequest>();
        let (_tx2, rx2) = std::sync::mpsc::channel::<(usize, Protocol)>();
        App::new(images, AppState::Browser, tx, rx2, Lang::Zh)
    }

    #[test]
    fn test_search_triggers_on_slash() {
        let mut app = make_app(20);
        assert!(app.search.is_none());
        app.handle_key(KeyCode::Char('/'), KeyModifiers::NONE);
        assert!(app.search.is_some());
        assert_eq!(app.search.as_ref().unwrap().trigger_char, '/');
    }

    #[test]
    fn test_search_triggers_on_backslash() {
        let mut app = make_app(20);
        app.handle_key(KeyCode::Char('\\'), KeyModifiers::NONE);
        assert!(app.search.is_some());
        assert_eq!(app.search.as_ref().unwrap().trigger_char, '\\');
    }

    #[test]
    fn test_search_esc_exits_search() {
        let mut app = make_app(20);
        app.selected = 10;
        app.handle_key(KeyCode::Char('/'), KeyModifiers::NONE);
        assert!(app.search.is_some());
        app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(app.search.is_none());
        assert_eq!(app.selected, 10);
    }

    #[test]
    fn test_search_char_jumps_and_pushes_to_query() {
        let mut app = make_app(20);
        app.handle_key(KeyCode::Char('/'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('0'), KeyModifiers::NONE);
        let search = app.search.as_ref().unwrap();
        assert_eq!(search.query, "0");
        assert!(!search.matches.is_empty());
    }

    #[test]
    fn test_search_backspace_works() {
        let mut app = make_app(20);
        app.handle_key(KeyCode::Char('/'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('x'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Backspace, KeyModifiers::NONE);
        let search = app.search.as_ref().unwrap();
        assert_eq!(search.query, "");
    }

    #[test]
    fn test_search_tab_cycles_matches() {
        let mut app = make_app_with_names(&["a_a.png", "a_b.png", "a_c.png", "x.png"]);
        app.handle_key(KeyCode::Char('/'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('a'), KeyModifiers::NONE);
        let first_match_idx = app.search.as_ref().unwrap().match_idx;
        app.handle_key(KeyCode::Tab, KeyModifiers::NONE);
        let search = app.search.as_ref().unwrap();
        let expected = (first_match_idx + 1) % search.matches.len();
        assert_eq!(search.match_idx, expected);
    }
}
