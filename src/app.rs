use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyModifiers};
use image::AnimationDecoder;
use ratatui::layout::Size;
use ratatui_image::{picker::Picker, protocol::Protocol, FilterType, Resize};

use crate::lang::Lang;
use crate::scanner::ImageEntry;
use crate::ui::search::{SearchAction, SearchState};

const MAX_ANIMATION_FRAMES: usize = 120;
const DEFAULT_FRAME_DELAY: Duration = Duration::from_millis(100);
const MIN_FRAME_DELAY: Duration = Duration::from_millis(20);

#[derive(Clone)]
pub struct AnimationFrame {
    pub protocol: Protocol,
    pub delay: Duration,
}

#[derive(Clone)]
pub struct StaticContent {
    pub protocol: Protocol,
    pub original: image::DynamicImage,
}

#[derive(Clone)]
pub enum FullscreenContent {
    Static(StaticContent),
    Animation(Vec<AnimationFrame>),
}

/// Channel payload for a completed background image load.
pub struct LoadResult {
    idx: usize,
    size: LoadSize,
    content: FullscreenContent,
    dims: Option<(u32, u32)>,
}

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
    pub fullscreen_content: Option<FullscreenContent>,
    fullscreen_frame_idx: usize,
    fullscreen_next_frame_at: Option<Instant>,
    pub fullscreen_pending: bool,
    pub fullscreen_dims: Option<(u32, u32)>,
    pub cache_width: u16,
    pub cache_height: u16,
    pub grid_cols: usize,
    pub thumb_w: u16,
    pub thumb_h: u16,
    pub visible_rows: usize,
    pub requested: HashSet<(usize, LoadSize)>,
    pub search: Option<SearchState>,
    pub zoom: f32,
    pub pan_x: i16,
    pub pan_y: i16,
    pub picker: Picker,
    pub fullscreen_image_w: u16,
    pub fullscreen_image_h: u16,
    pub lang: Lang,
    load_tx: Sender<LoadRequest>,
    load_rx: Receiver<LoadResult>,
}

pub const MIN_CELL: u16 = 24;
pub const LOGO_HEIGHT: u16 = 3;
pub const MIN_LOGO_WIDTH: u16 = 70;
const MAX_CACHE_SIZE: usize = 200;
const ZOOM_STEP: f32 = 1.25;
const ZOOM_MIN: f32 = 0.25;
const ZOOM_MAX: f32 = 10.0;

impl App {
    pub fn new(
        images: Vec<ImageEntry>,
        state: AppState,
        selected: usize,
        load_tx: Sender<LoadRequest>,
        load_rx: Receiver<LoadResult>,
        lang: Lang,
        picker: Picker,
    ) -> Self {
        let selected = selected.min(images.len().saturating_sub(1));
        let fullscreen_pending = state == AppState::Fullscreen;
        let mut app = Self {
            state,
            images,
            selected,
            scroll_row: 0,
            protocol_cache: HashMap::new(),
            fullscreen_content: None,
            fullscreen_frame_idx: 0,
            fullscreen_next_frame_at: None,
            fullscreen_pending,
            fullscreen_dims: None,
            cache_width: 0,
            cache_height: 0,
            grid_cols: 8,
            thumb_w: 0,
            thumb_h: 0,
            visible_rows: 1,
            requested: HashSet::new(),
            search: None,
            zoom: 1.0,
            pan_x: 0,
            pan_y: 0,
            picker,
            fullscreen_image_w: 0,
            fullscreen_image_h: 0,
            lang,
            load_tx,
            load_rx,
        };
        // If launched directly into fullscreen (e.g. "termfoto image.png"),
        // immediately request the original-size load so the image appears.
        if fullscreen_pending {
            app.request_load(selected, LoadSize::Original);
        }
        app
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
            self.reset_fullscreen_content();
            self.fullscreen_pending = true;
            self.request_load(self.selected, LoadSize::Original);
        }
    }

    pub fn exit_fullscreen(&mut self) {
        self.state = AppState::Browser;
        self.reset_fullscreen_content();
        self.zoom = 1.0;
        self.pan_x = 0;
        self.pan_y = 0;
        self.fullscreen_image_w = 0;
        self.fullscreen_image_h = 0;
        self.fullscreen_pending = false;
    }

    pub fn fullscreen_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.reset_fullscreen_content();
            self.zoom = 1.0;
            self.pan_x = 0;
            self.pan_y = 0;
            self.fullscreen_image_w = 0;
            self.fullscreen_image_h = 0;
            self.fullscreen_pending = true;
            self.request_load(self.selected, LoadSize::Original);
        }
    }

    pub fn fullscreen_next(&mut self) {
        if self.selected + 1 < self.images.len() {
            self.selected += 1;
            self.reset_fullscreen_content();
            self.zoom = 1.0;
            self.pan_x = 0;
            self.pan_y = 0;
            self.fullscreen_image_w = 0;
            self.fullscreen_image_h = 0;
            self.fullscreen_pending = true;
            self.request_load(self.selected, LoadSize::Original);
        }
    }

    fn reset_fullscreen_content(&mut self) {
        self.fullscreen_content = None;
        self.fullscreen_frame_idx = 0;
        self.fullscreen_next_frame_at = None;
        self.fullscreen_dims = None;
    }

    pub fn set_fullscreen_content(
        &mut self,
        content: FullscreenContent,
        dims: Option<(u32, u32)>,
        now: Instant,
    ) {
        self.fullscreen_frame_idx = 0;
        self.zoom = 1.0;
        self.pan_x = 0;
        self.pan_y = 0;
        self.fullscreen_next_frame_at = match &content {
            FullscreenContent::Animation(frames) => frames.first().map(|frame| now + frame.delay),
            FullscreenContent::Static(_) => None,
        };
        self.fullscreen_content = Some(content);
        self.fullscreen_dims = dims;
    }

    pub fn current_fullscreen_protocol(&self) -> Option<&Protocol> {
        match self.fullscreen_content.as_ref()? {
            FullscreenContent::Static(sc) => Some(&sc.protocol),
            FullscreenContent::Animation(frames) => frames
                .get(self.fullscreen_frame_idx)
                .or_else(|| frames.first())
                .map(|frame| &frame.protocol),
        }
    }

    #[cfg(test)]
    pub fn fullscreen_frame_index(&self) -> usize {
        self.fullscreen_frame_idx
    }

    pub fn next_animation_deadline(&self) -> Option<Instant> {
        if self.state == AppState::Fullscreen {
            self.fullscreen_next_frame_at
        } else {
            None
        }
    }

    pub fn advance_animation(&mut self, now: Instant) -> bool {
        if self.state != AppState::Fullscreen {
            return false;
        }

        let Some(FullscreenContent::Animation(frames)) = self.fullscreen_content.as_ref() else {
            return false;
        };
        if frames.len() < 2 {
            return false;
        }

        let Some(next_at) = self.fullscreen_next_frame_at else {
            return false;
        };
        if now < next_at {
            return false;
        }

        self.fullscreen_frame_idx = (self.fullscreen_frame_idx + 1) % frames.len();
        self.fullscreen_next_frame_at = Some(now + frames[self.fullscreen_frame_idx].delay);
        true
    }

    /// Check for completed background image loads.
    /// In Browser mode, results go into protocol_cache.
    /// In Fullscreen mode, original-size results for the selected image become fullscreen content.
    pub fn collect_loads(&mut self) {
        let now = Instant::now();
        while let Ok(result) = self.load_rx.try_recv() {
            let LoadResult {
                idx,
                size,
                content,
                dims,
            } = result;
            self.requested.remove(&(idx, size.clone()));
            if self.state == AppState::Fullscreen
                && idx == self.selected
                && matches!(size, LoadSize::Original)
            {
                self.set_fullscreen_content(content, dims, now);
                self.fullscreen_pending = false;
            } else {
                let proto = first_protocol(content);
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
        let key = (idx, size.clone());
        if self.requested.contains(&key) {
            return;
        }
        self.requested.insert(key);
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

    /// 放大，上限 ZOOM_MAX
    pub fn zoom_in(&mut self) {
        if self.state != AppState::Fullscreen {
            return;
        }
        self.set_zoom((self.zoom * ZOOM_STEP).min(ZOOM_MAX));
    }

    /// 缩小，下限 ZOOM_MIN
    pub fn zoom_out(&mut self) {
        if self.state != AppState::Fullscreen {
            return;
        }
        self.set_zoom((self.zoom / ZOOM_STEP).max(ZOOM_MIN));
    }

    /// 重置缩放与平移
    pub fn zoom_reset(&mut self) {
        if self.state != AppState::Fullscreen {
            return;
        }
        self.zoom = 1.0;
        self.pan_x = 0;
        self.pan_y = 0;
        self.regenerate_zoom_protocol();
    }

    fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom;
        self.regenerate_zoom_protocol();
    }

    /// 用缓存原始图像以当前缩放级别重新生成协议
    fn regenerate_zoom_protocol(&mut self) {
        let Some(content) = self.fullscreen_content.as_mut() else {
            return;
        };
        let FullscreenContent::Static(sc) = content else {
            return;
        };
        if self.fullscreen_image_w == 0 || self.fullscreen_image_h == 0 {
            return;
        }
        let new_w = ((self.fullscreen_image_w as f32) * self.zoom).max(1.0) as u16;
        let new_h = ((self.fullscreen_image_h as f32) * self.zoom).max(1.0) as u16;
        let size = Size::new(new_w, new_h);
        if let Ok(protocol) = self.picker.new_protocol(
            sc.original.clone(),
            size,
            Resize::Fit(Some(FilterType::Lanczos3)),
        ) {
            sc.protocol = protocol;
        }
        self.clamp_pan();
    }

    /// 平移后钳制到图片边界
    fn clamp_pan(&mut self) {
        let Some(proto) = self.current_fullscreen_protocol() else {
            return;
        };
        let pw = proto.size().width as i16;
        let ph = proto.size().height as i16;
        let vw = self.fullscreen_image_w as i16;
        let vh = self.fullscreen_image_h as i16;
        let max_x = ((pw - vw).max(0) / 2).max(0);
        let max_y = ((ph - vh).max(0) / 2).max(0);
        self.pan_x = self.pan_x.clamp(-max_x, max_x);
        self.pan_y = self.pan_y.clamp(-max_y, max_y);
    }

    fn pan_step_x(&self) -> i16 {
        ((self.fullscreen_image_w as f32) * 0.1).max(1.0) as i16
    }

    fn pan_step_y(&self) -> i16 {
        ((self.fullscreen_image_h as f32) * 0.1).max(1.0) as i16
    }

    pub fn pan_left(&mut self) {
        self.pan_x -= self.pan_step_x();
        self.clamp_pan();
    }
    pub fn pan_right(&mut self) {
        self.pan_x += self.pan_step_x();
        self.clamp_pan();
    }
    pub fn pan_up(&mut self) {
        self.pan_y -= self.pan_step_y();
        self.clamp_pan();
    }
    pub fn pan_down(&mut self) {
        self.pan_y += self.pan_step_y();
        self.clamp_pan();
    }
}

/// Size mode for background image loading.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

fn first_protocol(content: FullscreenContent) -> Protocol {
    match content {
        FullscreenContent::Static(sc) => sc.protocol,
        FullscreenContent::Animation(mut frames) => frames.remove(0).protocol,
    }
}

fn frame_delay(delay: image::Delay) -> Duration {
    let (numer, denom) = delay.numer_denom_ms();
    if denom == 0 {
        return DEFAULT_FRAME_DELAY;
    }
    let millis = u64::from(numer) / u64::from(denom);
    let duration = if millis == 0 {
        DEFAULT_FRAME_DELAY
    } else {
        Duration::from_millis(millis)
    };
    duration.max(MIN_FRAME_DELAY)
}

fn make_protocol(
    picker: &Picker,
    img: image::DynamicImage,
    size: Size,
    filter: FilterType,
) -> Option<Protocol> {
    picker
        .new_protocol(img, size, Resize::Fit(Some(filter)))
        .ok()
}

fn static_original_content(
    picker: &Picker,
    img: image::DynamicImage,
    size: Size,
) -> Option<FullscreenContent> {
    let protocol = make_protocol(picker, img.clone(), size, FilterType::Lanczos3)?;
    Some(FullscreenContent::Static(StaticContent { protocol, original: img }))
}

fn animation_content_from_frames<I>(
    picker: &Picker,
    frames: I,
    size: Size,
) -> Option<FullscreenContent>
where
    I: IntoIterator<Item = image::ImageResult<image::Frame>>,
{
    let mut animation_frames = Vec::new();
    for frame in frames {
        let frame = frame.ok()?;
        if animation_frames.len() == MAX_ANIMATION_FRAMES {
            return None;
        }
        let delay = frame_delay(frame.delay());
        let img = image::DynamicImage::ImageRgba8(frame.into_buffer());
        let protocol = make_protocol(picker, img, size, FilterType::Lanczos3)?;
        animation_frames.push(AnimationFrame { protocol, delay });
    }

    if animation_frames.len() >= 2 {
        Some(FullscreenContent::Animation(animation_frames))
    } else {
        None
    }
}

fn try_decode_animation(picker: &Picker, path: &Path, size: Size) -> Option<FullscreenContent> {
    let format = image::ImageFormat::from_path(path).ok()?;
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);

    match format {
        image::ImageFormat::Gif => {
            let decoder = image::codecs::gif::GifDecoder::new(reader).ok()?;
            animation_content_from_frames(picker, decoder.into_frames(), size)
        }
        image::ImageFormat::Png => {
            let decoder = image::codecs::png::PngDecoder::new(reader).ok()?;
            let decoder = decoder.apng().ok()?;
            animation_content_from_frames(picker, decoder.into_frames(), size)
        }
        image::ImageFormat::WebP => {
            let decoder = image::codecs::webp::WebPDecoder::new(reader).ok()?;
            animation_content_from_frames(picker, decoder.into_frames(), size)
        }
        _ => None,
    }
}

/// Spawn a background thread pool that loads images and creates Protocols in parallel.
/// Returns (sender, receiver) for App to use.
pub fn spawn_image_loader(
    picker: Picker,
    paths: Vec<std::path::PathBuf>,
) -> (Sender<LoadRequest>, Receiver<LoadResult>) {
    let (load_tx, load_rx) = std::sync::mpsc::channel::<LoadRequest>();
    let (done_tx, done_rx) = std::sync::mpsc::channel::<LoadResult>();
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
                    let dims = (img.width(), img.height());
                    let (img, protocol_size, filter) = match req.size {
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
                    let content = match req.size {
                        LoadSize::Original => try_decode_animation(&picker, path, protocol_size)
                            .or_else(|| static_original_content(&picker, img, protocol_size)),
                        LoadSize::Thumbnail { .. } => {
                            make_protocol(&picker, img.clone(), protocol_size, filter)
                                .map(|protocol| FullscreenContent::Static(StaticContent { protocol, original: img }))
                        }
                    };
                    if let Some(content) = content {
                        let _ = done_tx.send(LoadResult {
                            idx: req.idx,
                            size: req.size,
                            content,
                            dims: Some(dims),
                        });
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
    use std::time::{Duration, Instant};

    fn make_app(count: usize) -> App {
        let images = (0..count)
            .map(|i| ImageEntry {
                path: PathBuf::from(format!("img{:03}.png", i)),
                filename: format!("img{:03}.png", i),
                file_size: 0,
            })
            .collect();
        let (tx, _rx) = std::sync::mpsc::channel::<LoadRequest>();
        let (_tx2, rx2) = std::sync::mpsc::channel::<LoadResult>();
        App::new(images, AppState::Browser, 0, tx, rx2, Lang::Zh, Picker::halfblocks())
    }

    fn make_app_with_load_rx(count: usize) -> (App, Receiver<LoadRequest>) {
        let images = (0..count)
            .map(|i| ImageEntry {
                path: PathBuf::from(format!("img{:03}.png", i)),
                filename: format!("img{:03}.png", i),
                file_size: 0,
            })
            .collect();
        let (tx, rx) = std::sync::mpsc::channel::<LoadRequest>();
        let (_tx2, rx2) = std::sync::mpsc::channel::<LoadResult>();
        (App::new(images, AppState::Browser, 0, tx, rx2, Lang::Zh, Picker::halfblocks()), rx)
    }

    fn make_protocol() -> Protocol {
        let picker = Picker::halfblocks();
        let img = image::DynamicImage::new_rgba8(1, 1);
        picker
            .new_protocol(img, Size::new(1, 1), Resize::Fit(Some(FilterType::Nearest)))
            .unwrap()
    }

    fn make_animation_frame(delay_ms: u64) -> AnimationFrame {
        AnimationFrame {
            protocol: make_protocol(),
            delay: Duration::from_millis(delay_ms),
        }
    }

    fn make_image_frame(delay_ms: u32) -> image::Frame {
        image::Frame::from_parts(
            image::RgbaImage::new(1, 1),
            0,
            0,
            image::Delay::from_numer_denom_ms(delay_ms, 1),
        )
    }

    fn make_colored_image_frame(delay_ms: u32, color: [u8; 4]) -> image::Frame {
        image::Frame::from_parts(
            image::RgbaImage::from_pixel(1, 1, image::Rgba(color)),
            0,
            0,
            image::Delay::from_numer_denom_ms(delay_ms, 1),
        )
    }

    fn install_test_animation(app: &mut App, now: Instant) {
        app.state = AppState::Fullscreen;
        app.set_fullscreen_content(
            FullscreenContent::Animation(vec![
                make_animation_frame(100),
                make_animation_frame(150),
            ]),
            Some((1, 1)),
            now,
        );
    }

    #[test]
    fn animation_does_not_advance_before_delay() {
        let mut app = make_app(1);
        let start = Instant::now();
        install_test_animation(&mut app, start);

        assert!(!app.advance_animation(start + Duration::from_millis(99)));
        assert_eq!(app.fullscreen_frame_index(), 0);
    }

    #[test]
    fn animation_advances_after_delay() {
        let mut app = make_app(1);
        let start = Instant::now();
        install_test_animation(&mut app, start);

        assert!(app.advance_animation(start + Duration::from_millis(100)));
        assert_eq!(app.fullscreen_frame_index(), 1);
    }

    #[test]
    fn animation_loops_from_last_frame_to_first() {
        let mut app = make_app(1);
        let start = Instant::now();
        install_test_animation(&mut app, start);

        app.advance_animation(start + Duration::from_millis(100));
        assert!(app.advance_animation(start + Duration::from_millis(250)));
        assert_eq!(app.fullscreen_frame_index(), 0);
    }

    #[test]
    fn exiting_fullscreen_resets_animation_state() {
        let mut app = make_app(1);
        let start = Instant::now();
        install_test_animation(&mut app, start);
        app.advance_animation(start + Duration::from_millis(100));

        app.exit_fullscreen();

        assert_eq!(app.fullscreen_frame_index(), 0);
        assert!(app.current_fullscreen_protocol().is_none());
    }

    #[test]
    fn thumbnail_request_does_not_block_original_request() {
        let (mut app, rx) = make_app_with_load_rx(1);

        app.request_load(0, LoadSize::Thumbnail { w: 10, h: 5 });
        app.request_load(0, LoadSize::Original);

        assert_eq!(
            rx.try_recv().unwrap().size,
            LoadSize::Thumbnail { w: 10, h: 5 }
        );
        assert_eq!(rx.try_recv().unwrap().size, LoadSize::Original);
    }

    #[test]
    fn animation_content_requires_multiple_frames() {
        let picker = Picker::halfblocks();
        let frames = vec![Ok(make_image_frame(100))];

        let content = animation_content_from_frames(&picker, frames, Size::new(1, 1));

        assert!(content.is_none());
    }

    #[test]
    fn animation_content_accepts_two_to_max_frames() {
        let picker = Picker::halfblocks();
        let frames = vec![Ok(make_image_frame(100)), Ok(make_image_frame(150))];

        let content = animation_content_from_frames(&picker, frames, Size::new(1, 1));

        match content {
            Some(FullscreenContent::Animation(frames)) => {
                assert_eq!(frames.len(), 2);
                assert_eq!(frames[0].delay, Duration::from_millis(100));
                assert_eq!(frames[1].delay, Duration::from_millis(150));
            }
            _ => panic!("expected animation content"),
        }
    }

    #[test]
    fn animation_content_rejects_frames_over_limit() {
        let picker = Picker::halfblocks();
        let frames: Vec<_> = (0..=MAX_ANIMATION_FRAMES)
            .map(|_| Ok(make_image_frame(100)))
            .collect();

        let content = animation_content_from_frames(&picker, frames, Size::new(1, 1));

        assert!(content.is_none());
    }

    #[test]
    fn zero_frame_delay_defaults_to_100ms() {
        assert_eq!(
            frame_delay(image::Delay::from_numer_denom_ms(0, 1)),
            DEFAULT_FRAME_DELAY
        );
    }

    #[test]
    fn tiny_gif_decodes_to_animation_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tiny.gif");
        {
            let file = File::create(&path).unwrap();
            let mut encoder = image::codecs::gif::GifEncoder::new(file);
            encoder
                .encode_frames(vec![
                    make_colored_image_frame(100, [255, 0, 0, 255]),
                    make_colored_image_frame(120, [0, 255, 0, 255]),
                ])
                .unwrap();
        }

        let picker = Picker::halfblocks();
        let content = try_decode_animation(&picker, &path, Size::new(1, 1));

        match content {
            Some(FullscreenContent::Animation(frames)) => assert_eq!(frames.len(), 2),
            _ => panic!("expected animated GIF content"),
        }
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
                file_size: 0,
            })
            .collect();
        let (tx, _rx) = std::sync::mpsc::channel::<LoadRequest>();
        let (_tx2, rx2) = std::sync::mpsc::channel::<LoadResult>();
        App::new(images, AppState::Browser, 0, tx, rx2, Lang::Zh, Picker::halfblocks())
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
