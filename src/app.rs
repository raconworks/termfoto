use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::{
    mpsc::{Receiver, Sender},
    Arc,
};
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyModifiers};
use fast_image_resize as fir;
use image::AnimationDecoder;
use lru::LruCache;
use ratatui::layout::Size;
use ratatui_image::{
    picker::Picker, protocol::Protocol, FilterType as ProtocolFilterType, FontSize, Resize,
};

use crate::lang::Lang;
use crate::scanner::{scan_directory, ImageEntry};
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
    pub protocol: Option<Protocol>,
    pub original: Arc<image::RgbaImage>,
}

#[derive(Clone)]
pub enum FullscreenContent {
    Static(StaticContent),
    Animation(Vec<AnimationFrame>),
}

/// Channel payload for a completed background image load.
pub struct LoadResult {
    idx: usize,
    path: PathBuf,
    size: LoadSize,
    generation: u64,
    content: LoadContent,
    dims: Option<(u32, u32)>,
}

enum LoadContent {
    Thumbnail(Protocol),
    Original(FullscreenContent),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Browser,
    Fullscreen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserFocus {
    Gallery,
    Context,
}

pub struct App {
    pub state: AppState,
    pub images: Vec<ImageEntry>,
    pub image_dir: PathBuf,
    pub(crate) context_dir: PathBuf,
    pub selected: usize,
    pub scroll_row: usize,
    pub browser_focus: BrowserFocus,
    pub context_selected: usize,
    pub context_scroll: usize,
    context_visible_rows: usize,
    pub directory_generation: u64,
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
    pub requested: HashSet<(u64, usize, LoadSize)>,
    pub search: Option<SearchState>,
    pub zoom: f32,
    pub pan_x: i16,
    pub pan_y: i16,
    pub picker: Picker,
    pub fullscreen_image_w: u16,
    pub fullscreen_image_h: u16,
    zoom_dirty: bool,
    render_dirty_reason: Option<RenderDirtyReason>,
    render_generation: u64,
    render_settle_deadline: Option<Instant>,
    fullscreen_protocol_key: Option<RenderKey>,
    fullscreen_original_cache: LruCache<usize, CachedOriginal>,
    fullscreen_original_cache_bytes: usize,
    fullscreen_render_cache: LruCache<RenderKey, Protocol>,
    render_tx: Sender<RenderRequest>,
    render_rx: Receiver<RenderResult>,
    pub lang: Lang,
    load_tx: Sender<LoadRequest>,
    load_rx: Receiver<LoadResult>,
    status_message: Option<(String, Instant)>,
}

pub struct AppStart {
    pub images: Vec<ImageEntry>,
    pub image_dir: PathBuf,
    pub state: AppState,
    pub selected: usize,
}

pub const MIN_CELL: u16 = 24;
pub const LOGO_HEIGHT: u16 = 3;
const MAX_CACHE_SIZE: usize = 200;
const ZOOM_STEP: f32 = 0.10;
const ZOOM_MIN: f32 = 1.0;
const ZOOM_MAX: f32 = 10.0;
const FULLSCREEN_ORIGINAL_CACHE_BYTES: usize = 128 * 1024 * 1024;
const FULLSCREEN_RENDER_CACHE_SIZE: usize = 8;
const INTERACTIVE_SETTLE_DELAY: Duration = Duration::from_millis(120);
const DIRECT_FINAL_RENDER_PIXELS: u64 = 1_000_000;

struct ZoomRenderGeometry {
    source_x: f64,
    source_y: f64,
    source_w: f64,
    source_h: f64,
    target_px_w: u32,
    target_px_h: u32,
}

struct ZoomDisplayGeometry {
    scale: f64,
    display_px_w: f64,
    display_px_h: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RenderQuality {
    Interactive,
    Final,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenderDirtyReason {
    Interaction,
    ContentOrViewport,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RenderKey {
    idx: usize,
    viewport_w: u16,
    viewport_h: u16,
    font_w: u16,
    font_h: u16,
    zoom_percent: u16,
    pan_x: i16,
    pan_y: i16,
    quality: RenderQuality,
}

impl RenderKey {
    fn same_view(&self, other: &Self) -> bool {
        self.idx == other.idx
            && self.viewport_w == other.viewport_w
            && self.viewport_h == other.viewport_h
            && self.font_w == other.font_w
            && self.font_h == other.font_h
            && self.zoom_percent == other.zoom_percent
            && self.pan_x == other.pan_x
            && self.pan_y == other.pan_y
    }
}

#[derive(Clone)]
struct CachedOriginal {
    image: Arc<image::RgbaImage>,
    bytes: usize,
}

struct RenderRequest {
    idx: usize,
    image: Arc<image::RgbaImage>,
    viewport: Size,
    font_size: FontSize,
    zoom: f32,
    pan_x: i16,
    pan_y: i16,
    key: RenderKey,
    generation: u64,
}

struct RenderResult {
    idx: usize,
    protocol: Protocol,
    key: RenderKey,
    generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectoryContextKind {
    Directory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryContextEntry {
    pub name: String,
    pub path: PathBuf,
    pub kind: DirectoryContextKind,
    pub is_current: bool,
    pub depth: usize,
}

fn zoom_render_geometry(
    img_w: u32,
    img_h: u32,
    viewport_px_w: u32,
    viewport_px_h: u32,
    zoom: f32,
    pan_px_x: i32,
    pan_px_y: i32,
) -> ZoomRenderGeometry {
    let img_w = img_w.max(1);
    let img_h = img_h.max(1);
    let viewport_px_w = viewport_px_w.max(1);
    let viewport_px_h = viewport_px_h.max(1);
    let display = zoom_display_geometry(img_w, img_h, viewport_px_w, viewport_px_h, zoom);

    let visible_display_w = display.display_px_w.min(f64::from(viewport_px_w));
    let visible_display_h = display.display_px_h.min(f64::from(viewport_px_h));
    let target_px_w = rounded_px(visible_display_w);
    let target_px_h = rounded_px(visible_display_h);

    let max_display_x = (display.display_px_w - f64::from(viewport_px_w)).max(0.0);
    let max_display_y = (display.display_px_h - f64::from(viewport_px_h)).max(0.0);
    let display_x = if max_display_x > 0.0 {
        (max_display_x / 2.0 + f64::from(pan_px_x)).clamp(0.0, max_display_x)
    } else {
        0.0
    };
    let display_y = if max_display_y > 0.0 {
        (max_display_y / 2.0 + f64::from(pan_px_y)).clamp(0.0, max_display_y)
    } else {
        0.0
    };

    let source_w = (visible_display_w / display.scale).clamp(1.0, f64::from(img_w));
    let source_h = (visible_display_h / display.scale).clamp(1.0, f64::from(img_h));
    let max_source_x = (f64::from(img_w) - source_w).max(0.0);
    let max_source_y = (f64::from(img_h) - source_h).max(0.0);
    let source_x = (display_x / display.scale).clamp(0.0, max_source_x);
    let source_y = (display_y / display.scale).clamp(0.0, max_source_y);

    ZoomRenderGeometry {
        source_x,
        source_y,
        source_w,
        source_h,
        target_px_w,
        target_px_h,
    }
}

fn zoom_display_geometry(
    img_w: u32,
    img_h: u32,
    viewport_px_w: u32,
    viewport_px_h: u32,
    zoom: f32,
) -> ZoomDisplayGeometry {
    let img_w = img_w.max(1);
    let img_h = img_h.max(1);
    let viewport_px_w = viewport_px_w.max(1);
    let viewport_px_h = viewport_px_h.max(1);
    let zoom = normalized_zoom(zoom);
    let fit_scale = (f64::from(viewport_px_w) / f64::from(img_w))
        .min(f64::from(viewport_px_h) / f64::from(img_h))
        .max(f64::EPSILON);
    let scale = fit_scale * f64::from(zoom);

    ZoomDisplayGeometry {
        scale,
        display_px_w: (f64::from(img_w) * scale).max(1.0),
        display_px_h: (f64::from(img_h) * scale).max(1.0),
    }
}

fn normalized_zoom(zoom: f32) -> f32 {
    if zoom.is_finite() {
        zoom.clamp(ZOOM_MIN, ZOOM_MAX)
    } else {
        ZOOM_MIN
    }
}

fn rounded_px(value: f64) -> u32 {
    value.round().clamp(1.0, f64::from(u32::MAX)) as u32
}

fn max_pan_cells(display_px: f64, viewport_px: u32, font_px: u16) -> i16 {
    let overflow = display_px - f64::from(viewport_px.max(1));
    if overflow <= 0.0 {
        return 0;
    }
    let cells = (overflow / 2.0 / f64::from(font_px.max(1))).ceil();
    cells.clamp(0.0, f64::from(i16::MAX)) as i16
}

impl App {
    pub fn new(
        start: AppStart,
        load_tx: Sender<LoadRequest>,
        load_rx: Receiver<LoadResult>,
        lang: Lang,
        picker: Picker,
    ) -> Self {
        let AppStart {
            images,
            image_dir,
            state,
            selected,
        } = start;
        let selected = selected.min(images.len().saturating_sub(1));
        let fullscreen_pending = state == AppState::Fullscreen;
        let (render_tx, render_rx) = spawn_render_worker(picker.clone());
        let mut app = Self {
            state,
            images,
            context_dir: image_dir.clone(),
            image_dir,
            selected,
            scroll_row: 0,
            browser_focus: BrowserFocus::Gallery,
            context_selected: 0,
            context_scroll: 0,
            context_visible_rows: 1,
            directory_generation: 0,
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
            zoom_dirty: false,
            render_dirty_reason: None,
            render_generation: 0,
            render_settle_deadline: None,
            fullscreen_protocol_key: None,
            fullscreen_original_cache: LruCache::unbounded(),
            fullscreen_original_cache_bytes: 0,
            fullscreen_render_cache: LruCache::new(
                NonZeroUsize::new(FULLSCREEN_RENDER_CACHE_SIZE).unwrap(),
            ),
            render_tx,
            render_rx,
            lang,
            load_tx,
            load_rx,
            status_message: None,
        };
        app.reset_context_selection_to_current_folder();
        // If launched directly into fullscreen (e.g. "termfoto image.png"),
        // immediately request the original load so the image appears.
        if fullscreen_pending {
            app.prepare_fullscreen_selection();
        }
        app
    }

    pub fn directory_context_for_browser(&self) -> Vec<DirectoryContextEntry> {
        browser_directory_context_entries(self.context_dir.as_path())
    }

    pub fn clamp_context_selection(&mut self, len: usize, visible_rows: usize) {
        self.context_visible_rows = visible_rows.max(1);
        if len == 0 {
            self.context_selected = 0;
            self.context_scroll = 0;
            return;
        }
        self.context_selected = self.context_selected.min(len - 1);
        self.clamp_context_scroll_bounds(len);
    }

    fn clamp_context_scroll_to_selection(&mut self, len: usize) {
        if len == 0 {
            self.context_scroll = 0;
            return;
        }
        let visible_rows = self.context_visible_rows.max(1);
        if self.context_selected < self.context_scroll {
            self.context_scroll = self.context_selected;
        } else if self.context_selected >= self.context_scroll + visible_rows {
            self.context_scroll = self.context_selected + 1 - visible_rows;
        }
        self.clamp_context_scroll_bounds(len);
    }

    fn clamp_context_scroll_bounds(&mut self, len: usize) {
        let max_scroll = len.saturating_sub(self.context_visible_rows.max(1));
        self.context_scroll = self.context_scroll.min(max_scroll);
    }

    fn context_down(&mut self) {
        let len = self.directory_context_for_browser().len();
        if self.context_selected + 1 < len {
            self.context_selected += 1;
            self.clamp_context_scroll_to_selection(len);
        }
    }

    fn context_up(&mut self) {
        self.context_selected = self.context_selected.saturating_sub(1);
        let len = self.directory_context_for_browser().len();
        self.clamp_context_scroll_to_selection(len);
    }

    fn context_home(&mut self) {
        self.context_selected = 0;
        let len = self.directory_context_for_browser().len();
        self.clamp_context_scroll_to_selection(len);
    }

    fn context_end(&mut self) {
        let len = self.directory_context_for_browser().len();
        self.context_selected = len.saturating_sub(1);
        self.clamp_context_scroll_to_selection(len);
    }

    fn enter_selected_context_directory(&mut self) {
        let entries = self.directory_context_for_browser();
        let Some(entry) = entries.get(self.context_selected) else {
            return;
        };
        self.enter_directory_with_context(entry.path.clone(), entry.path.clone());
    }

    fn enter_parent_directory(&mut self) {
        let Some(new_image_dir) = browser_context_parent(self.image_dir.as_path()) else {
            return;
        };
        self.enter_directory_with_context(new_image_dir.clone(), new_image_dir);
    }

    #[cfg(test)]
    fn enter_directory(&mut self, dir: PathBuf) {
        self.enter_directory_with_context(dir.clone(), dir);
    }

    fn enter_directory_with_context(&mut self, dir: PathBuf, context_dir: PathBuf) {
        let Ok(images) = scan_directory(&dir) else {
            self.status_message = Some((
                format!("{}: {}", self.lang.directory_error(), dir.display()),
                Instant::now() + Duration::from_secs(2),
            ));
            return;
        };

        self.image_dir = dir;
        self.context_dir = context_dir;
        self.images = images;
        self.selected = 0;
        self.scroll_row = 0;
        self.reset_context_selection_to_current_folder();
        self.search = None;
        self.status_message = None;
        self.directory_generation = self.directory_generation.wrapping_add(1);
        self.clear_directory_caches();
    }

    fn reset_context_selection_to_current_folder(&mut self) {
        let entries = self.directory_context_for_browser();
        self.context_selected = if entries.len() > 1 { 1 } else { 0 };
        self.context_scroll = 0;
    }

    fn clear_directory_caches(&mut self) {
        self.clear_protocol_cache();
        self.reset_fullscreen_content();
        self.fullscreen_pending = false;
        self.fullscreen_image_w = 0;
        self.fullscreen_image_h = 0;
        self.zoom = 1.0;
        self.pan_x = 0;
        self.pan_y = 0;
        self.fullscreen_original_cache.clear();
        self.fullscreen_original_cache_bytes = 0;
        self.fullscreen_render_cache.clear();
    }

    pub fn browser_status_message(&mut self) -> Option<String> {
        let (message, expires_at) = self.status_message.as_ref()?;
        if Instant::now() >= *expires_at {
            self.status_message = None;
            return None;
        }
        Some(message.clone())
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
            self.prepare_fullscreen_selection();
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
        self.render_dirty_reason = None;
        self.render_settle_deadline = None;
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
            self.prepare_fullscreen_selection();
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
            self.prepare_fullscreen_selection();
        }
    }

    fn reset_fullscreen_content(&mut self) {
        self.fullscreen_content = None;
        self.fullscreen_frame_idx = 0;
        self.fullscreen_next_frame_at = None;
        self.fullscreen_dims = None;
        self.zoom_dirty = false;
        self.render_dirty_reason = None;
        self.render_settle_deadline = None;
        self.fullscreen_protocol_key = None;
        self.render_generation = self.render_generation.wrapping_add(1);
    }

    fn prepare_fullscreen_selection(&mut self) {
        if self.show_cached_fullscreen_original(Instant::now()) {
            self.fullscreen_pending = self.current_fullscreen_protocol().is_none();
        } else {
            self.fullscreen_pending = true;
            self.request_load(self.selected, LoadSize::Original);
        }
        self.prefetch_fullscreen_neighbors();
    }

    fn show_cached_fullscreen_original(&mut self, now: Instant) -> bool {
        let Some(image) = self.cached_fullscreen_original(self.selected) else {
            return false;
        };
        let dims = Some((image.width(), image.height()));
        self.set_fullscreen_content(
            FullscreenContent::Static(StaticContent {
                protocol: None,
                original: image,
            }),
            dims,
            now,
        );
        true
    }

    fn prefetch_fullscreen_neighbors(&mut self) {
        if self.images.is_empty() {
            return;
        }
        if self.selected > 0 {
            self.request_load(self.selected - 1, LoadSize::Original);
        }
        if self.selected + 1 < self.images.len() {
            self.request_load(self.selected + 1, LoadSize::Original);
        }
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
        let is_static = matches!(&content, FullscreenContent::Static(_));
        self.fullscreen_next_frame_at = match &content {
            FullscreenContent::Animation(frames) => frames.first().map(|frame| now + frame.delay),
            FullscreenContent::Static(_) => None,
        };
        self.fullscreen_content = Some(content);
        self.fullscreen_dims = dims;
        self.fullscreen_protocol_key = None;
        if is_static {
            self.mark_render_dirty(RenderDirtyReason::ContentOrViewport);
        } else {
            self.zoom_dirty = false;
            self.render_dirty_reason = None;
            self.render_settle_deadline = None;
        }
    }

    pub fn set_fullscreen_viewport(&mut self, width: u16, height: u16) {
        let changed = self.fullscreen_image_w != width || self.fullscreen_image_h != height;
        self.fullscreen_image_w = width;
        self.fullscreen_image_h = height;
        if changed && matches!(self.fullscreen_content, Some(FullscreenContent::Static(_))) {
            self.clamp_pan();
            self.mark_render_dirty(RenderDirtyReason::ContentOrViewport);
        }
    }

    pub fn current_fullscreen_protocol(&self) -> Option<&Protocol> {
        match self.fullscreen_content.as_ref()? {
            FullscreenContent::Static(sc) => sc.protocol.as_ref(),
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

    pub fn next_render_deadline(&self) -> Option<Instant> {
        self.render_settle_deadline
    }

    fn mark_render_dirty(&mut self, reason: RenderDirtyReason) {
        self.zoom_dirty = true;
        self.render_dirty_reason = Some(reason);
        self.render_settle_deadline = None;
        self.render_generation = self.render_generation.wrapping_add(1);
    }

    /// Check for completed background image loads.
    /// In Browser mode, results go into protocol_cache.
    /// In Fullscreen mode, original results populate the decoded-original cache.
    pub fn collect_loads(&mut self) {
        let now = Instant::now();
        while let Ok(result) = self.load_rx.try_recv() {
            let LoadResult {
                idx,
                path,
                size,
                generation,
                content,
                dims,
            } = result;
            self.requested.remove(&(generation, idx, size.clone()));
            if generation != self.directory_generation {
                continue;
            }
            let path_is_current = self.images.get(idx).is_some_and(|entry| entry.path == path);
            if !path_is_current {
                continue;
            }
            match content {
                LoadContent::Original(content) => match content {
                    FullscreenContent::Static(sc) => {
                        self.insert_fullscreen_original(idx, Arc::clone(&sc.original));
                        if self.state == AppState::Fullscreen && idx == self.selected {
                            self.set_fullscreen_content(
                                FullscreenContent::Static(StaticContent {
                                    protocol: None,
                                    original: sc.original,
                                }),
                                dims,
                                now,
                            );
                            self.fullscreen_pending = true;
                        }
                    }
                    FullscreenContent::Animation(frames) => {
                        if self.state == AppState::Fullscreen && idx == self.selected {
                            self.set_fullscreen_content(
                                FullscreenContent::Animation(frames),
                                dims,
                                now,
                            );
                            self.fullscreen_pending = false;
                        }
                    }
                },
                LoadContent::Thumbnail(proto) => {
                    // Discard protocols that exceed current cell (stale from terminal resize)
                    let psize = proto.size();
                    if self.thumb_w > 0
                        && (psize.width > self.thumb_w || psize.height > self.thumb_h)
                    {
                        continue;
                    }
                    self.insert_cache(idx, proto);
                }
            }
        }
    }

    pub fn collect_render_results(&mut self) {
        while let Ok(result) = self.render_rx.try_recv() {
            self.apply_render_result(result);
        }
    }

    fn apply_render_result(&mut self, result: RenderResult) {
        if self.state != AppState::Fullscreen
            || self.selected != result.idx
            || self.render_generation != result.generation
        {
            return;
        }

        let Some(current_key) = self.current_render_key(result.key.quality) else {
            return;
        };
        if current_key != result.key {
            return;
        }
        if result.key.quality == RenderQuality::Interactive
            && self.fullscreen_protocol_key.as_ref().is_some_and(|key| {
                key.quality == RenderQuality::Final && key.same_view(&result.key)
            })
        {
            return;
        }

        self.fullscreen_render_cache
            .put(result.key.clone(), result.protocol.clone());
        self.apply_static_protocol(result.key, result.protocol);
    }

    fn apply_static_protocol(&mut self, key: RenderKey, protocol: Protocol) {
        let Some(FullscreenContent::Static(sc)) = self.fullscreen_content.as_mut() else {
            return;
        };
        sc.protocol = Some(protocol);
        self.fullscreen_protocol_key = Some(key);
        self.fullscreen_pending = false;
    }

    pub fn drive_render_queue(&mut self, now: Instant) {
        if self.state != AppState::Fullscreen {
            return;
        }

        let Some(geometry) = self.current_render_geometry() else {
            return;
        };

        if self.zoom_dirty {
            let reason = self
                .render_dirty_reason
                .unwrap_or(RenderDirtyReason::ContentOrViewport);
            let quality = match reason {
                RenderDirtyReason::Interaction => RenderQuality::Interactive,
                RenderDirtyReason::ContentOrViewport => {
                    if u64::from(geometry.target_px_w) * u64::from(geometry.target_px_h)
                        <= DIRECT_FINAL_RENDER_PIXELS
                    {
                        RenderQuality::Final
                    } else {
                        RenderQuality::Interactive
                    }
                }
            };
            if !self.apply_cached_render(quality) {
                self.submit_render_request(quality);
            }
            self.zoom_dirty = false;
            self.render_dirty_reason = None;
            self.render_settle_deadline =
                (quality == RenderQuality::Interactive).then_some(now + INTERACTIVE_SETTLE_DELAY);
            return;
        }

        if self
            .render_settle_deadline
            .is_some_and(|deadline| now >= deadline)
        {
            self.render_settle_deadline = None;
            if !self.apply_cached_render(RenderQuality::Final) {
                self.submit_render_request(RenderQuality::Final);
            }
        }
    }

    /// Compatibility wrapper for tests that still exercise the dirty flag semantics.
    #[cfg(test)]
    pub fn regenerate_if_dirty(&mut self) {
        self.drive_render_queue(Instant::now());
    }

    fn apply_cached_render(&mut self, quality: RenderQuality) -> bool {
        let Some(key) = self.current_render_key(quality) else {
            return false;
        };
        let Some(protocol) = self.fullscreen_render_cache.get(&key).cloned() else {
            return false;
        };
        self.apply_static_protocol(key, protocol);
        true
    }

    fn submit_render_request(&mut self, quality: RenderQuality) -> bool {
        let Some(request) = self.current_render_request(quality) else {
            return false;
        };
        self.render_tx.send(request).is_ok()
    }

    fn current_render_request(&self, quality: RenderQuality) -> Option<RenderRequest> {
        let FullscreenContent::Static(sc) = self.fullscreen_content.as_ref()? else {
            return None;
        };
        if self.fullscreen_image_w == 0 || self.fullscreen_image_h == 0 {
            return None;
        }
        let font_size = self.picker.font_size();
        let key = self.render_key(quality, font_size);
        Some(RenderRequest {
            idx: self.selected,
            image: Arc::clone(&sc.original),
            viewport: Size::new(
                self.fullscreen_image_w.max(1),
                self.fullscreen_image_h.max(1),
            ),
            font_size,
            zoom: self.zoom,
            pan_x: self.pan_x,
            pan_y: self.pan_y,
            key,
            generation: self.render_generation,
        })
    }

    fn current_render_key(&self, quality: RenderQuality) -> Option<RenderKey> {
        if !matches!(
            self.fullscreen_content.as_ref()?,
            FullscreenContent::Static(_)
        ) || self.fullscreen_image_w == 0
            || self.fullscreen_image_h == 0
        {
            return None;
        }
        Some(self.render_key(quality, self.picker.font_size()))
    }

    fn render_key(&self, quality: RenderQuality, font_size: FontSize) -> RenderKey {
        RenderKey {
            idx: self.selected,
            viewport_w: self.fullscreen_image_w.max(1),
            viewport_h: self.fullscreen_image_h.max(1),
            font_w: font_size.width.max(1),
            font_h: font_size.height.max(1),
            zoom_percent: zoom_percent(self.zoom),
            pan_x: self.pan_x,
            pan_y: self.pan_y,
            quality,
        }
    }

    fn current_render_geometry(&self) -> Option<ZoomRenderGeometry> {
        let FullscreenContent::Static(sc) = self.fullscreen_content.as_ref()? else {
            return None;
        };
        if self.fullscreen_image_w == 0 || self.fullscreen_image_h == 0 {
            return None;
        }
        let fs = self.picker.font_size();
        let vp_px_w = (self.fullscreen_image_w as u32).saturating_mul(fs.width as u32);
        let vp_px_h = (self.fullscreen_image_h as u32).saturating_mul(fs.height as u32);
        let pan_px_x = (self.pan_x as f32 * fs.width as f32) as i32;
        let pan_px_y = (self.pan_y as f32 * fs.height as f32) as i32;
        Some(zoom_render_geometry(
            sc.original.width(),
            sc.original.height(),
            vp_px_w,
            vp_px_h,
            self.zoom,
            pan_px_x,
            pan_px_y,
        ))
    }

    fn cached_fullscreen_original(&mut self, idx: usize) -> Option<Arc<image::RgbaImage>> {
        self.fullscreen_original_cache
            .get(&idx)
            .map(|entry| Arc::clone(&entry.image))
    }

    fn insert_fullscreen_original(&mut self, idx: usize, image: Arc<image::RgbaImage>) {
        let bytes = rgba_bytes(&image);
        if let Some(old) = self
            .fullscreen_original_cache
            .put(idx, CachedOriginal { image, bytes })
        {
            self.fullscreen_original_cache_bytes = self
                .fullscreen_original_cache_bytes
                .saturating_sub(old.bytes);
        }
        self.fullscreen_original_cache_bytes =
            self.fullscreen_original_cache_bytes.saturating_add(bytes);
        self.evict_fullscreen_originals(Some(self.selected));
    }

    fn evict_fullscreen_originals(&mut self, protect_idx: Option<usize>) {
        let mut protected = Vec::new();
        while self.fullscreen_original_cache_bytes > FULLSCREEN_ORIGINAL_CACHE_BYTES
            && self.fullscreen_original_cache.len() + protected.len() > 1
        {
            let Some((idx, entry)) = self.fullscreen_original_cache.pop_lru() else {
                break;
            };
            if Some(idx) == protect_idx {
                protected.push((idx, entry));
                continue;
            }
            self.fullscreen_original_cache_bytes = self
                .fullscreen_original_cache_bytes
                .saturating_sub(entry.bytes);
        }
        for (idx, entry) in protected {
            self.fullscreen_original_cache.put(idx, entry);
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
        let Some(entry) = self.images.get(idx) else {
            return;
        };
        if matches!(size, LoadSize::Original) && self.fullscreen_original_cache.contains(&idx) {
            return;
        }
        let key = (self.directory_generation, idx, size.clone());
        if self.requested.contains(&key) {
            return;
        }
        self.requested.insert(key);
        let _ = self.load_tx.send(LoadRequest {
            idx,
            path: entry.path.clone(),
            size,
            generation: self.directory_generation,
        });
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
                    KeyCode::Tab | KeyCode::BackTab => {
                        self.browser_focus = match self.browser_focus {
                            BrowserFocus::Gallery => BrowserFocus::Context,
                            BrowserFocus::Context => BrowserFocus::Gallery,
                        };
                    }
                    _ => match self.browser_focus {
                        BrowserFocus::Gallery => match code {
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
                        BrowserFocus::Context => match code {
                            KeyCode::Left => self.enter_parent_directory(),
                            KeyCode::Right | KeyCode::Enter => {
                                self.enter_selected_context_directory()
                            }
                            KeyCode::Up => self.context_up(),
                            KeyCode::Down => self.context_down(),
                            KeyCode::Home => self.context_home(),
                            KeyCode::End => self.context_end(),
                            _ => {}
                        },
                    },
                }
            }
            AppState::Fullscreen => match code {
                KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => self.exit_fullscreen(),
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return true,
                KeyCode::Char('L') => {
                    self.lang.toggle();
                }
                KeyCode::Char('+') | KeyCode::Char('=') => self.zoom_in(),
                KeyCode::Char('-') => self.zoom_out(),
                KeyCode::Char('0') => self.zoom_reset(),
                KeyCode::Char('h') => self.pan_left(),
                KeyCode::Char('l') => self.pan_right(),
                KeyCode::Char('k') => self.pan_up(),
                KeyCode::Char('j') => self.pan_down(),
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
        self.set_zoom((self.zoom + ZOOM_STEP).min(ZOOM_MAX));
    }

    /// 缩小，下限 ZOOM_MIN
    pub fn zoom_out(&mut self) {
        if self.state != AppState::Fullscreen {
            return;
        }
        self.set_zoom((self.zoom - ZOOM_STEP).max(ZOOM_MIN));
    }

    /// 重置缩放与平移
    pub fn zoom_reset(&mut self) {
        if self.state != AppState::Fullscreen {
            return;
        }
        self.zoom = 1.0;
        self.pan_x = 0;
        self.pan_y = 0;
        self.mark_render_dirty(RenderDirtyReason::Interaction);
    }

    fn set_zoom(&mut self, zoom: f32) {
        self.zoom = normalized_zoom(zoom);
        self.clamp_pan();
        self.mark_render_dirty(RenderDirtyReason::Interaction);
    }

    /// 平移后钳制到缩放后整图超出视口的范围内
    fn clamp_pan(&mut self) {
        let Some(FullscreenContent::Static(sc)) = self.fullscreen_content.as_ref() else {
            return;
        };
        if self.fullscreen_image_w == 0 || self.fullscreen_image_h == 0 {
            self.pan_x = 0;
            self.pan_y = 0;
            return;
        }
        let fs = self.picker.font_size();
        let viewport_px_w = (self.fullscreen_image_w as u32).saturating_mul(fs.width as u32);
        let viewport_px_h = (self.fullscreen_image_h as u32).saturating_mul(fs.height as u32);
        let display = zoom_display_geometry(
            sc.original.width(),
            sc.original.height(),
            viewport_px_w,
            viewport_px_h,
            self.zoom,
        );
        let max_cell_x = max_pan_cells(display.display_px_w, viewport_px_w, fs.width);
        let max_cell_y = max_pan_cells(display.display_px_h, viewport_px_h, fs.height);
        self.pan_x = self.pan_x.clamp(-max_cell_x, max_cell_x);
        self.pan_y = self.pan_y.clamp(-max_cell_y, max_cell_y);
    }

    fn pan_step_x(&self) -> i16 {
        let Some(FullscreenContent::Static(sc)) = self.fullscreen_content.as_ref() else {
            return 1;
        };
        let fs = self.picker.font_size();
        let nat_w = sc.original.width().div_ceil(fs.width as u32) as f32;
        ((nat_w / self.zoom) * 0.1).max(1.0) as i16
    }

    fn pan_step_y(&self) -> i16 {
        let Some(FullscreenContent::Static(sc)) = self.fullscreen_content.as_ref() else {
            return 1;
        };
        let fs = self.picker.font_size();
        let nat_h = sc.original.height().div_ceil(fs.height as u32) as f32;
        ((nat_h / self.zoom) * 0.1).max(1.0) as i16
    }

    pub fn pan_left(&mut self) {
        self.pan_x -= self.pan_step_x();
        self.clamp_pan();
        self.mark_render_dirty(RenderDirtyReason::Interaction);
    }
    pub fn pan_right(&mut self) {
        self.pan_x += self.pan_step_x();
        self.clamp_pan();
        self.mark_render_dirty(RenderDirtyReason::Interaction);
    }
    pub fn pan_up(&mut self) {
        self.pan_y -= self.pan_step_y();
        self.clamp_pan();
        self.mark_render_dirty(RenderDirtyReason::Interaction);
    }
    pub fn pan_down(&mut self) {
        self.pan_y += self.pan_step_y();
        self.clamp_pan();
        self.mark_render_dirty(RenderDirtyReason::Interaction);
    }
}

fn browser_directory_context_entries(context_dir: &Path) -> Vec<DirectoryContextEntry> {
    let mut entries = vec![DirectoryContextEntry {
        name: directory_display_name(context_dir),
        path: context_dir.to_path_buf(),
        kind: DirectoryContextKind::Directory,
        is_current: true,
        depth: 0,
    }];

    let Ok(read_dir) = std::fs::read_dir(context_dir) else {
        return entries;
    };

    let mut children: Vec<_> = read_dir
        .filter_map(|res| res.ok())
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            if !file_type.is_dir() {
                return None;
            }
            Some(DirectoryContextEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                path: entry.path(),
                kind: DirectoryContextKind::Directory,
                is_current: false,
                depth: 1,
            })
        })
        .collect();

    children.sort_by(|a, b| a.name.cmp(&b.name));
    entries.extend(children);
    entries
}

fn directory_display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn browser_context_parent(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    if parent.as_os_str().is_empty() {
        Some(PathBuf::from("."))
    } else {
        Some(parent.to_path_buf())
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
    pub path: PathBuf,
    pub size: LoadSize,
    pub generation: u64,
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
    filter: ProtocolFilterType,
) -> Option<Protocol> {
    picker
        .new_protocol(img, size, Resize::Fit(Some(filter)))
        .ok()
}

#[cfg(test)]
fn static_original_content(img: image::DynamicImage) -> FullscreenContent {
    static_rgba_content(img.into_rgba8())
}

fn static_rgba_content(img: image::RgbaImage) -> FullscreenContent {
    FullscreenContent::Static(StaticContent {
        protocol: None,
        original: Arc::new(img),
    })
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
        let protocol = make_protocol(picker, img, size, ProtocolFilterType::Lanczos3)?;
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

fn zoom_percent(zoom: f32) -> u16 {
    let zoom = normalized_zoom(zoom);
    (zoom * 100.0).round().clamp(1.0, u16::MAX as f32) as u16
}

fn rgba_bytes(image: &image::RgbaImage) -> usize {
    image.len()
}

fn spawn_render_worker(picker: Picker) -> (Sender<RenderRequest>, Receiver<RenderResult>) {
    let (render_tx, render_rx) = std::sync::mpsc::channel::<RenderRequest>();
    let (done_tx, done_rx) = std::sync::mpsc::channel::<RenderResult>();

    std::thread::spawn(move || {
        let mut resizer = fir::Resizer::new();
        while let Ok(mut request) = render_rx.recv() {
            while let Ok(next) = render_rx.try_recv() {
                request = next;
            }
            if let Some(protocol) = render_zoom_protocol(&picker, &mut resizer, &request) {
                let _ = done_tx.send(RenderResult {
                    idx: request.idx,
                    protocol,
                    key: request.key,
                    generation: request.generation,
                });
            }
        }
    });

    (render_tx, done_rx)
}

fn render_zoom_protocol(
    picker: &Picker,
    resizer: &mut fir::Resizer,
    request: &RenderRequest,
) -> Option<Protocol> {
    let viewport_px_w =
        (request.viewport.width as u32).saturating_mul(request.font_size.width as u32);
    let viewport_px_h =
        (request.viewport.height as u32).saturating_mul(request.font_size.height as u32);
    let pan_px_x = (request.pan_x as f32 * request.font_size.width as f32) as i32;
    let pan_px_y = (request.pan_y as f32 * request.font_size.height as f32) as i32;
    let geometry = zoom_render_geometry(
        request.image.width(),
        request.image.height(),
        viewport_px_w,
        viewport_px_h,
        request.zoom,
        pan_px_x,
        pan_px_y,
    );

    let resized =
        resize_with_fast_image_resize(resizer, &request.image, &geometry, request.key.quality)
            .unwrap_or_else(|| {
                resize_with_image_crate(&request.image, &geometry, request.key.quality)
            });
    let resized_img = image::DynamicImage::ImageRgba8(resized);
    picker
        .new_protocol(resized_img, request.viewport, Resize::Fit(None))
        .ok()
}

fn resize_with_fast_image_resize(
    resizer: &mut fir::Resizer,
    image: &image::RgbaImage,
    geometry: &ZoomRenderGeometry,
    quality: RenderQuality,
) -> Option<image::RgbaImage> {
    let mut dst = image::RgbaImage::new(geometry.target_px_w, geometry.target_px_h);
    let algorithm = match quality {
        RenderQuality::Interactive => fir::ResizeAlg::Nearest,
        RenderQuality::Final => fir::ResizeAlg::Convolution(fir::FilterType::Lanczos3),
    };
    let options = fir::ResizeOptions::new().resize_alg(algorithm).crop(
        geometry.source_x,
        geometry.source_y,
        geometry.source_w,
        geometry.source_h,
    );

    resizer.resize(image, &mut dst, Some(&options)).ok()?;
    Some(dst)
}

fn resize_with_image_crate(
    image: &image::RgbaImage,
    geometry: &ZoomRenderGeometry,
    quality: RenderQuality,
) -> image::RgbaImage {
    let filter = match quality {
        RenderQuality::Interactive => image::imageops::FilterType::Nearest,
        RenderQuality::Final => image::imageops::FilterType::Lanczos3,
    };
    let (source_x, source_y, source_w, source_h) = integer_source_rect(image, geometry);
    let cropped =
        image::imageops::crop_imm(image, source_x, source_y, source_w, source_h).to_image();
    image::imageops::resize(&cropped, geometry.target_px_w, geometry.target_px_h, filter)
}

fn integer_source_rect(
    image: &image::RgbaImage,
    geometry: &ZoomRenderGeometry,
) -> (u32, u32, u32, u32) {
    let img_w = image.width().max(1);
    let img_h = image.height().max(1);
    let source_x = geometry
        .source_x
        .floor()
        .clamp(0.0, f64::from(img_w.saturating_sub(1))) as u32;
    let source_y = geometry
        .source_y
        .floor()
        .clamp(0.0, f64::from(img_h.saturating_sub(1))) as u32;
    let max_w = img_w.saturating_sub(source_x).max(1);
    let max_h = img_h.saturating_sub(source_y).max(1);
    let source_w = rounded_px(geometry.source_w).min(max_w).max(1);
    let source_h = rounded_px(geometry.source_h).min(max_h).max(1);

    (source_x, source_y, source_w, source_h)
}

/// Spawn background workers that load thumbnails separately from fullscreen originals.
/// Returns (sender, receiver) for App to use.
pub fn spawn_image_loader(
    picker: Picker,
    _paths: Vec<std::path::PathBuf>,
) -> (Sender<LoadRequest>, Receiver<LoadResult>) {
    let (load_tx, load_rx) = std::sync::mpsc::channel::<LoadRequest>();
    let (done_tx, done_rx) = std::sync::mpsc::channel::<LoadResult>();

    let (thumb_tx, thumb_rx) = std::sync::mpsc::channel::<LoadRequest>();
    let (original_tx, original_rx) = std::sync::mpsc::channel::<LoadRequest>();

    std::thread::spawn(move || {
        while let Ok(req) = load_rx.recv() {
            let routed = match &req.size {
                LoadSize::Thumbnail { .. } => thumb_tx.send(req),
                LoadSize::Original => original_tx.send(req),
            };
            if routed.is_err() {
                break;
            }
        }
    });

    spawn_loader_workers(picker.clone(), done_tx.clone(), thumb_rx, 3);
    spawn_loader_workers(picker, done_tx, original_rx, 1);

    (load_tx, done_rx)
}

fn spawn_loader_workers(
    picker: Picker,
    done_tx: Sender<LoadResult>,
    load_rx: Receiver<LoadRequest>,
    workers: usize,
) {
    let rx = Arc::new(std::sync::Mutex::new(load_rx));
    for _ in 0..workers {
        let picker = picker.clone();
        let done_tx = done_tx.clone();
        let rx = Arc::clone(&rx);

        std::thread::spawn(move || loop {
            let req = {
                let rx = rx.lock().unwrap();
                match rx.recv() {
                    Ok(req) => req,
                    Err(_) => return,
                }
            };

            if let Some(result) = process_load_request(&picker, req) {
                let _ = done_tx.send(result);
            }
        });
    }
}

fn process_load_request(picker: &Picker, req: LoadRequest) -> Option<LoadResult> {
    let LoadRequest {
        idx,
        path,
        size,
        generation,
    } = req;
    match size {
        LoadSize::Thumbnail { w, h } => {
            process_thumbnail_request(picker, path.as_path(), idx, generation, w, h)
        }
        LoadSize::Original => process_original_request(picker, path.as_path(), idx, generation),
    }
}

fn process_thumbnail_request(
    picker: &Picker,
    path: &Path,
    idx: usize,
    generation: u64,
    w: u16,
    h: u16,
) -> Option<LoadResult> {
    let img = image::open(path).ok()?;
    let font_size = picker.font_size();
    let pixel_w = w as u32 * font_size.width as u32 * 2;
    let pixel_h = h as u32 * font_size.height as u32 * 2;
    let dims = Some((img.width(), img.height()));
    let thumb = img.thumbnail(pixel_w, pixel_h);
    let protocol = make_protocol(picker, thumb, Size::new(w, h), ProtocolFilterType::Nearest)?;

    Some(LoadResult {
        idx,
        path: path.to_path_buf(),
        size: LoadSize::Thumbnail { w, h },
        generation,
        content: LoadContent::Thumbnail(protocol),
        dims,
    })
}

fn process_original_request(
    picker: &Picker,
    path: &Path,
    idx: usize,
    generation: u64,
) -> Option<LoadResult> {
    let dims = image::image_dimensions(path).ok()?;
    let font_size = picker.font_size();
    let nat_w = dims.0.div_ceil(font_size.width as u32) as u16;
    let nat_h = dims.1.div_ceil(font_size.height as u32) as u16;
    let protocol_size = Size::new(nat_w.max(1), nat_h.max(1));

    let content = if should_probe_animation(path) {
        try_decode_animation(picker, path, protocol_size)
    } else {
        None
    };
    let content = match content {
        Some(content) => content,
        None => static_rgba_content(image::open(path).ok()?.into_rgba8()),
    };

    Some(LoadResult {
        idx,
        path: path.to_path_buf(),
        size: LoadSize::Original,
        generation,
        content: LoadContent::Original(content),
        dims: Some(dims),
    })
}

fn should_probe_animation(path: &Path) -> bool {
    matches!(
        image::ImageFormat::from_path(path).ok(),
        Some(image::ImageFormat::Gif | image::ImageFormat::Png | image::ImageFormat::WebP)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};
    use tempfile::tempdir;

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
        App::new(
            AppStart {
                images,
                image_dir: PathBuf::from("."),
                state: AppState::Browser,
                selected: 0,
            },
            tx,
            rx2,
            Lang::Zh,
            Picker::halfblocks(),
        )
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
        (
            App::new(
                AppStart {
                    images,
                    image_dir: PathBuf::from("."),
                    state: AppState::Browser,
                    selected: 0,
                },
                tx,
                rx2,
                Lang::Zh,
                Picker::halfblocks(),
            ),
            rx,
        )
    }

    fn make_app_with_load_done(count: usize) -> (App, Sender<LoadResult>) {
        let images = (0..count)
            .map(|i| ImageEntry {
                path: PathBuf::from(format!("img{:03}.png", i)),
                filename: format!("img{:03}.png", i),
                file_size: 0,
            })
            .collect();
        let (tx, _rx) = std::sync::mpsc::channel::<LoadRequest>();
        let (done_tx, done_rx) = std::sync::mpsc::channel::<LoadResult>();
        (
            App::new(
                AppStart {
                    images,
                    image_dir: PathBuf::from("."),
                    state: AppState::Browser,
                    selected: 0,
                },
                tx,
                done_rx,
                Lang::Zh,
                Picker::halfblocks(),
            ),
            done_tx,
        )
    }

    fn make_protocol() -> Protocol {
        let picker = Picker::halfblocks();
        let img = image::DynamicImage::new_rgba8(1, 1);
        picker
            .new_protocol(
                img,
                Size::new(1, 1),
                Resize::Fit(Some(ProtocolFilterType::Nearest)),
            )
            .unwrap()
    }

    fn make_static_content(width: u32, height: u32) -> FullscreenContent {
        FullscreenContent::Static(StaticContent {
            protocol: Some(make_protocol()),
            original: Arc::new(image::RgbaImage::new(width, height)),
        })
    }

    fn write_png(path: &Path) {
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            1,
            1,
            image::Rgba([1, 2, 3, 255]),
        ))
        .save(path)
        .unwrap();
    }

    #[test]
    fn browser_directory_context_lists_current_child_directories_only() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        fs::create_dir(&photos).unwrap();
        fs::create_dir(photos.join("z_album")).unwrap();
        fs::create_dir(photos.join("a_album")).unwrap();
        fs::write(photos.join("note.txt"), b"note").unwrap();
        write_png(&photos.join("photo.png"));

        let mut app = make_app(0);
        app.image_dir = photos.clone();
        app.context_dir = photos;

        let entries = app.directory_context_for_browser();
        let names: Vec<_> = entries.iter().map(|entry| entry.name.as_str()).collect();

        assert_eq!(names, vec!["photos", "a_album", "z_album"]);
        assert!(entries[0].is_current);
        assert_eq!(entries[0].depth, 0);
        assert!(entries[1..].iter().all(|entry| entry.depth == 1));
        assert!(entries[1..].iter().all(|entry| !entry.is_current));
        assert!(!entries.iter().any(|entry| entry.name == ".."));
        assert_eq!(entries[0].kind, DirectoryContextKind::Directory);
        assert!(entries
            .iter()
            .all(|entry| entry.kind == DirectoryContextKind::Directory));
    }

    #[test]
    fn browser_context_starts_at_current_image_directory() {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        fs::create_dir(&photos).unwrap();
        fs::create_dir(photos.join("album")).unwrap();

        let (tx, _rx) = std::sync::mpsc::channel::<LoadRequest>();
        let (_tx2, rx2) = std::sync::mpsc::channel::<LoadResult>();
        let app = App::new(
            AppStart {
                images: Vec::new(),
                image_dir: photos,
                state: AppState::Browser,
                selected: 0,
            },
            tx,
            rx2,
            Lang::Zh,
            Picker::halfblocks(),
        );

        let entries = app.directory_context_for_browser();
        let names: Vec<_> = entries.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(names, vec!["photos", "album"]);
    }

    #[test]
    fn browser_directory_context_missing_directory_keeps_current_entry() {
        let missing = PathBuf::from("/tmp/termfoto-missing-directory-context");

        assert_eq!(browser_directory_context_entries(&missing).len(), 1);
    }

    #[test]
    fn browser_directory_context_handles_relative_single_component_dir() {
        let entries = browser_directory_context_entries(Path::new("."));

        assert!(entries.iter().any(|entry| entry.name == "src"));
        assert!(!entries.iter().any(|entry| entry.name == ".."));
    }

    #[test]
    fn browser_directory_context_omits_parent_for_root() {
        let entries = browser_directory_context_entries(Path::new("/"));

        assert!(!entries.iter().any(|entry| entry.name == ".."));
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

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 0.01,
            "expected {actual} to be close to {expected}"
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

        let thumb = rx.try_recv().unwrap();
        assert_eq!(thumb.idx, 0);
        assert_eq!(thumb.path, PathBuf::from("img000.png"));
        assert_eq!(thumb.generation, app.directory_generation);
        assert_eq!(thumb.size, LoadSize::Thumbnail { w: 10, h: 5 });

        let original = rx.try_recv().unwrap();
        assert_eq!(original.idx, 0);
        assert_eq!(original.path, PathBuf::from("img000.png"));
        assert_eq!(original.generation, app.directory_generation);
        assert_eq!(original.size, LoadSize::Original);
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
    fn process_original_request_returns_animation_for_gif() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("animated.gif");
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
        let result = process_original_request(&picker, &path, 7, 11).unwrap();

        assert_eq!(result.idx, 7);
        assert_eq!(result.path, path);
        assert_eq!(result.size, LoadSize::Original);
        assert_eq!(result.generation, 11);
        assert_eq!(result.dims, Some((1, 1)));
        match result.content {
            LoadContent::Original(FullscreenContent::Animation(frames)) => {
                assert_eq!(frames.len(), 2);
            }
            _ => panic!("expected animated original content"),
        }
    }

    #[test]
    fn process_original_request_decodes_static_jpeg_to_rgba() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("static.jpg");
        image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(3, 2, image::Rgb([10, 20, 30])))
            .save(&path)
            .unwrap();

        let picker = Picker::halfblocks();
        let result = process_original_request(&picker, &path, 2, 13).unwrap();

        assert_eq!(result.idx, 2);
        assert_eq!(result.path, path);
        assert_eq!(result.size, LoadSize::Original);
        assert_eq!(result.generation, 13);
        assert_eq!(result.dims, Some((3, 2)));
        match result.content {
            LoadContent::Original(FullscreenContent::Static(sc)) => {
                assert!(sc.protocol.is_none());
                assert_eq!(sc.original.width(), 3);
                assert_eq!(sc.original.height(), 2);
                assert_eq!(sc.original.len(), 3 * 2 * 4);
            }
            _ => panic!("expected static original content"),
        }
    }

    #[test]
    fn process_thumbnail_request_returns_protocol_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("thumb.png");
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            4,
            3,
            image::Rgba([1, 2, 3, 255]),
        ))
        .save(&path)
        .unwrap();

        let picker = Picker::halfblocks();
        let result = process_thumbnail_request(&picker, &path, 5, 17, 8, 4).unwrap();

        assert_eq!(result.idx, 5);
        assert_eq!(result.path, path);
        assert_eq!(result.size, LoadSize::Thumbnail { w: 8, h: 4 });
        assert_eq!(result.generation, 17);
        assert_eq!(result.dims, Some((4, 3)));
        match result.content {
            LoadContent::Thumbnail(protocol) => assert!(protocol.size().width <= 8),
            LoadContent::Original(_) => panic!("expected thumbnail protocol"),
        }
    }

    #[test]
    fn static_original_content_has_no_protocol_until_rendered() {
        let content = static_original_content(image::DynamicImage::new_rgba8(10, 20));

        match content {
            FullscreenContent::Static(sc) => {
                assert!(sc.protocol.is_none());
                assert_eq!(sc.original.width(), 10);
                assert_eq!(sc.original.height(), 20);
            }
            FullscreenContent::Animation(_) => panic!("expected static content"),
        }
    }

    #[test]
    fn stale_render_result_is_discarded() {
        let mut app = make_app(1);
        app.state = AppState::Fullscreen;
        app.set_fullscreen_content(
            static_original_content(image::DynamicImage::new_rgba8(400, 300)),
            Some((400, 300)),
            Instant::now(),
        );
        app.set_fullscreen_viewport(20, 10);
        let key = app.current_render_key(RenderQuality::Final).unwrap();
        let current_generation = app.render_generation;

        app.apply_render_result(RenderResult {
            idx: app.selected,
            protocol: make_protocol(),
            key,
            generation: current_generation.saturating_sub(1),
        });

        assert!(app.current_fullscreen_protocol().is_none());
        assert!(app.fullscreen_protocol_key.is_none());
    }

    #[test]
    fn protocol_cache_hit_satisfies_dirty_render() {
        let mut app = make_app(1);
        let now = Instant::now();
        app.state = AppState::Fullscreen;
        app.set_fullscreen_content(
            static_original_content(image::DynamicImage::new_rgba8(400, 300)),
            Some((400, 300)),
            now,
        );
        app.set_fullscreen_viewport(20, 10);
        let key = app.current_render_key(RenderQuality::Final).unwrap();
        app.fullscreen_render_cache
            .put(key.clone(), make_protocol());

        app.drive_render_queue(now);

        assert!(!app.zoom_dirty);
        assert!(app.current_fullscreen_protocol().is_some());
        assert_eq!(app.fullscreen_protocol_key, Some(key));
        assert!(app.next_render_deadline().is_none());
    }

    #[test]
    fn interaction_dirty_uses_interactive_even_for_small_viewport() {
        let mut app = make_app(1);
        let now = Instant::now();
        app.state = AppState::Fullscreen;
        app.set_fullscreen_content(
            static_original_content(image::DynamicImage::new_rgba8(400, 300)),
            Some((400, 300)),
            now,
        );
        app.set_fullscreen_viewport(20, 10);
        app.zoom_dirty = false;
        app.render_dirty_reason = None;

        app.zoom_in();
        let interactive_key = app.current_render_key(RenderQuality::Interactive).unwrap();
        let final_key = app.current_render_key(RenderQuality::Final).unwrap();
        app.fullscreen_render_cache
            .put(final_key.clone(), make_protocol());
        app.fullscreen_render_cache
            .put(interactive_key.clone(), make_protocol());

        app.drive_render_queue(now);

        assert!(!app.zoom_dirty);
        assert_eq!(app.fullscreen_protocol_key, Some(interactive_key));
        assert_eq!(
            app.next_render_deadline(),
            Some(now + INTERACTIVE_SETTLE_DELAY)
        );
    }

    #[test]
    fn interaction_settle_renders_final_quality() {
        let mut app = make_app(1);
        let now = Instant::now();
        app.state = AppState::Fullscreen;
        app.set_fullscreen_content(
            static_original_content(image::DynamicImage::new_rgba8(400, 300)),
            Some((400, 300)),
            now,
        );
        app.set_fullscreen_viewport(20, 10);
        app.zoom_dirty = false;
        app.render_dirty_reason = None;

        app.zoom_in();
        let interactive_key = app.current_render_key(RenderQuality::Interactive).unwrap();
        let final_key = app.current_render_key(RenderQuality::Final).unwrap();
        app.fullscreen_render_cache
            .put(interactive_key.clone(), make_protocol());
        app.fullscreen_render_cache
            .put(final_key.clone(), make_protocol());

        app.drive_render_queue(now);
        app.drive_render_queue(now + INTERACTIVE_SETTLE_DELAY);

        assert_eq!(app.fullscreen_protocol_key, Some(final_key));
        assert!(app.next_render_deadline().is_none());
    }

    #[test]
    fn content_dirty_large_viewport_uses_interactive_then_final() {
        let mut app = make_app(1);
        let now = Instant::now();
        app.state = AppState::Fullscreen;
        app.set_fullscreen_content(
            static_original_content(image::DynamicImage::new_rgba8(4000, 3000)),
            Some((4000, 3000)),
            now,
        );
        app.set_fullscreen_viewport(2000, 1000);
        let interactive_key = app.current_render_key(RenderQuality::Interactive).unwrap();
        let final_key = app.current_render_key(RenderQuality::Final).unwrap();
        app.fullscreen_render_cache
            .put(final_key.clone(), make_protocol());
        app.fullscreen_render_cache
            .put(interactive_key.clone(), make_protocol());

        app.drive_render_queue(now);

        assert_eq!(app.fullscreen_protocol_key, Some(interactive_key));
        assert_eq!(
            app.next_render_deadline(),
            Some(now + INTERACTIVE_SETTLE_DELAY)
        );
    }

    #[test]
    fn fullscreen_original_cache_accounts_rgba_bytes() {
        let mut app = make_app(1);

        app.insert_fullscreen_original(0, Arc::new(image::RgbaImage::new(10, 20)));

        assert_eq!(app.fullscreen_original_cache_bytes, 10 * 20 * 4);
        assert_eq!(
            app.cached_fullscreen_original(0).map(|image| image.len()),
            Some(10 * 20 * 4)
        );
    }

    #[test]
    fn fullscreen_original_cache_evicts_to_budget_and_keeps_selected() {
        let mut app = make_app(3);
        app.selected = 0;
        for idx in 0..3 {
            app.insert_fullscreen_original(idx, Arc::new(image::RgbaImage::new(4096, 4096)));
        }

        assert!(app.fullscreen_original_cache_bytes <= FULLSCREEN_ORIGINAL_CACHE_BYTES);
        assert!(app.fullscreen_original_cache.contains(&0));
        assert!(app.fullscreen_original_cache.contains(&2));
        assert!(!app.fullscreen_original_cache.contains(&1));
    }

    #[test]
    fn fullscreen_original_cache_evicts_neighbor_before_selected() {
        let mut app = make_app(2);
        app.selected = 0;
        for idx in 0..2 {
            app.insert_fullscreen_original(idx, Arc::new(image::RgbaImage::new(5000, 5000)));
        }

        assert!(app.fullscreen_original_cache_bytes <= FULLSCREEN_ORIGINAL_CACHE_BYTES);
        assert!(app.fullscreen_original_cache.contains(&0));
        assert!(!app.fullscreen_original_cache.contains(&1));
    }

    #[test]
    fn animation_does_not_enter_static_render_queue() {
        let mut app = make_app(1);
        let now = Instant::now();
        install_test_animation(&mut app, now);
        app.set_fullscreen_viewport(20, 10);
        let generation = app.render_generation;

        app.drive_render_queue(now);

        assert!(!app.zoom_dirty);
        assert_eq!(app.render_generation, generation);
        assert!(app.next_render_deadline().is_none());
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

    #[test]
    fn tab_and_backtab_toggle_browser_focus() {
        let mut app = make_app(1);

        assert_eq!(app.browser_focus, BrowserFocus::Gallery);
        app.handle_key(KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(app.browser_focus, BrowserFocus::Context);
        app.handle_key(KeyCode::BackTab, KeyModifiers::SHIFT);
        assert_eq!(app.browser_focus, BrowserFocus::Gallery);
    }

    #[test]
    fn context_focus_moves_selection_with_arrow_keys() {
        let dir = tempdir().unwrap();
        let current = dir.path().join("current");
        fs::create_dir(&current).unwrap();
        fs::create_dir(current.join("a_child")).unwrap();
        fs::create_dir(current.join("m_child")).unwrap();
        fs::create_dir(current.join("z_child")).unwrap();

        let mut app = make_app(0);
        app.image_dir = current.clone();
        app.context_dir = current;
        app.reset_context_selection_to_current_folder();
        app.browser_focus = BrowserFocus::Context;

        assert_eq!(app.context_selected, 1);
        app.handle_key(KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(app.context_selected, 2);
        app.handle_key(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(app.context_selected, 1);
        app.handle_key(KeyCode::End, KeyModifiers::NONE);
        assert_eq!(app.context_selected, 3);
        app.handle_key(KeyCode::Home, KeyModifiers::NONE);
        assert_eq!(app.context_selected, 0);
    }

    #[test]
    fn context_render_clamp_does_not_auto_scroll_to_selection() {
        let mut app = make_app(0);
        app.context_selected = 8;
        app.context_scroll = 0;

        app.clamp_context_selection(10, 5);

        assert_eq!(app.context_selected, 8);
        assert_eq!(app.context_scroll, 0);
    }

    #[test]
    fn context_scroll_clamp_fills_visible_height_when_possible() {
        let mut app = make_app(0);
        app.context_selected = 9;
        app.context_scroll = 9;

        app.clamp_context_selection(10, 5);

        assert_eq!(app.context_scroll, 5);
    }

    #[test]
    fn context_scroll_resets_when_all_entries_fit() {
        let mut app = make_app(0);
        app.context_selected = 2;
        app.context_scroll = 4;

        app.clamp_context_selection(3, 8);

        assert_eq!(app.context_scroll, 0);
    }

    #[test]
    fn context_enter_switches_to_directory_and_resets_browser_state() {
        let dir = tempdir().unwrap();
        let current = dir.path().join("current");
        let child = current.join("child");
        fs::create_dir(&current).unwrap();
        fs::create_dir(&child).unwrap();
        write_png(&current.join("old.png"));
        write_png(&child.join("new.png"));

        let images = scan_directory(&current).unwrap();
        let (tx, _rx) = std::sync::mpsc::channel::<LoadRequest>();
        let (_done_tx, done_rx) = std::sync::mpsc::channel::<LoadResult>();
        let mut app = App::new(
            AppStart {
                images,
                image_dir: current,
                state: AppState::Browser,
                selected: 0,
            },
            tx,
            done_rx,
            Lang::Zh,
            Picker::halfblocks(),
        );
        app.browser_focus = BrowserFocus::Context;
        app.context_dir = app.image_dir.clone();
        app.context_selected = 1;
        app.scroll_row = 3;
        app.protocol_cache.insert(0, make_protocol());
        app.requested.insert((
            app.directory_generation,
            0,
            LoadSize::Thumbnail { w: 1, h: 1 },
        ));
        let generation = app.directory_generation;

        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

        assert_eq!(app.image_dir, child);
        assert_eq!(app.images.len(), 1);
        assert_eq!(app.images[0].filename, "new.png");
        assert_eq!(app.selected, 0);
        assert_eq!(app.scroll_row, 0);
        assert_eq!(app.context_selected, 0);
        assert_eq!(app.context_scroll, 0);
        assert!(app.protocol_cache.is_empty());
        assert!(app.requested.is_empty());
        assert!(app.directory_generation > generation);
    }

    #[test]
    fn context_right_enters_selected_directory() {
        let dir = tempdir().unwrap();
        let current = dir.path().join("current");
        let child = current.join("child");
        fs::create_dir(&current).unwrap();
        fs::create_dir(&child).unwrap();

        let mut app = make_app(0);
        app.image_dir = current.clone();
        app.context_dir = current;
        app.reset_context_selection_to_current_folder();
        app.browser_focus = BrowserFocus::Context;
        app.context_selected = 1;

        app.handle_key(KeyCode::Right, KeyModifiers::NONE);

        assert_eq!(app.image_dir, child);
    }

    #[test]
    fn context_left_returns_to_parent_directory() {
        let dir = tempdir().unwrap();
        let parent = dir.path().join("parent");
        let current = parent.join("current");
        fs::create_dir(&parent).unwrap();
        fs::create_dir(&current).unwrap();

        let mut app = make_app(0);
        app.image_dir = current.clone();
        app.context_dir = current;
        app.browser_focus = BrowserFocus::Context;

        app.handle_key(KeyCode::Left, KeyModifiers::NONE);

        assert_eq!(app.image_dir, parent);
    }

    #[test]
    fn context_can_enter_child_after_returning_to_parent() {
        let dir = tempdir().unwrap();
        let parent = dir.path().join("parent");
        let child = parent.join("child");
        fs::create_dir(&parent).unwrap();
        fs::create_dir(&child).unwrap();

        let mut app = make_app(0);
        app.image_dir = child.clone();
        app.context_dir = browser_context_parent(child.as_path()).unwrap();
        app.browser_focus = BrowserFocus::Context;

        app.handle_key(KeyCode::Left, KeyModifiers::NONE);
        assert_eq!(app.image_dir, parent);
        let entries = app.directory_context_for_browser();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, parent);
        assert_eq!(entries[1].path, child);

        app.handle_key(KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(app.image_dir, entries[1].path);
    }

    #[test]
    fn entering_directory_resets_search_state() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        let child = root.join("child");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&child).unwrap();
        write_png(&child.join("new.png"));

        let mut app = make_app(0);
        app.image_dir = root.clone();
        app.context_dir = root;
        app.search = Some(SearchState::new(0, '/'));

        app.enter_directory(child);

        assert!(app.search.is_none());
        assert_eq!(app.images.len(), 1);
    }

    #[test]
    fn context_enter_allows_empty_image_directory() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        let empty = root.join("empty");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&empty).unwrap();
        write_png(&root.join("old.png"));

        let mut app = make_app(0);
        app.image_dir = root.clone();
        app.context_dir = root;
        app.images = scan_directory(&app.image_dir).unwrap();
        app.browser_focus = BrowserFocus::Context;
        app.context_selected = 1;

        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(app.image_dir, empty);
        assert!(app.images.is_empty());

        app.browser_focus = BrowserFocus::Gallery;
        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(app.state, AppState::Browser);
    }

    #[test]
    fn failed_directory_scan_keeps_current_directory_and_images() {
        let dir = tempdir().unwrap();
        write_png(&dir.path().join("old.png"));
        let old_images = scan_directory(dir.path()).unwrap();

        let mut app = make_app(0);
        app.image_dir = dir.path().to_path_buf();
        app.context_dir = dir.path().to_path_buf();
        app.images = old_images;
        app.enter_directory(dir.path().join("missing"));

        assert_eq!(app.image_dir, dir.path());
        assert_eq!(app.images.len(), 1);
        assert!(app.browser_status_message().is_some());
    }

    #[test]
    fn stale_load_results_are_discarded() {
        let (mut app, done_tx) = make_app_with_load_done(1);
        let generation = app.directory_generation;

        done_tx
            .send(LoadResult {
                idx: 0,
                path: PathBuf::from("img000.png"),
                size: LoadSize::Thumbnail { w: 1, h: 1 },
                generation: generation.wrapping_add(1),
                content: LoadContent::Thumbnail(make_protocol()),
                dims: Some((1, 1)),
            })
            .unwrap();
        app.collect_loads();
        assert!(app.protocol_cache.is_empty());

        done_tx
            .send(LoadResult {
                idx: 0,
                path: PathBuf::from("other.png"),
                size: LoadSize::Thumbnail { w: 1, h: 1 },
                generation,
                content: LoadContent::Thumbnail(make_protocol()),
                dims: Some((1, 1)),
            })
            .unwrap();
        app.collect_loads();
        assert!(app.protocol_cache.is_empty());
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
        App::new(
            AppStart {
                images,
                image_dir: PathBuf::from("."),
                state: AppState::Browser,
                selected: 0,
            },
            tx,
            rx2,
            Lang::Zh,
            Picker::halfblocks(),
        )
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

    // ---- Zoom / Pan tests ----

    #[test]
    fn zoom_in_increases_zoom() {
        let mut app = make_app(5);
        app.state = AppState::Fullscreen;
        app.zoom = 1.0;
        app.zoom_in();
        assert!((app.zoom - 1.1).abs() < 0.01);
    }

    #[test]
    fn zoom_out_decreases_zoom() {
        let mut app = make_app(5);
        app.state = AppState::Fullscreen;
        app.zoom = 2.0;
        app.zoom_out();
        assert!((app.zoom - 1.9).abs() < 0.01);
    }

    #[test]
    fn zoom_clamped_to_max() {
        let mut app = make_app(5);
        app.state = AppState::Fullscreen;
        app.zoom = 10.0;
        app.zoom_in();
        assert!((app.zoom - 10.0).abs() < 0.01);
    }

    #[test]
    fn zoom_clamped_to_min() {
        let mut app = make_app(5);
        app.state = AppState::Fullscreen;
        app.zoom = 1.0;
        app.zoom_out();
        assert!((app.zoom - 1.0).abs() < 0.01);
    }

    #[test]
    fn zoom_out_recovers_to_min_when_below_100_percent() {
        let mut app = make_app(5);
        app.state = AppState::Fullscreen;
        app.zoom = 0.25;
        app.zoom_out();
        assert!((app.zoom - 1.0).abs() < 0.01);
    }

    #[test]
    fn switching_image_resets_zoom_and_pan() {
        let mut app = make_app(3);
        app.state = AppState::Fullscreen;
        app.zoom = 2.0;
        app.pan_x = 5;
        app.pan_y = 3;
        app.fullscreen_next();
        assert!((app.zoom - 1.0).abs() < 0.01);
        assert_eq!(app.pan_x, 0);
        assert_eq!(app.pan_y, 0);
    }

    #[test]
    fn zoom_reset_sets_defaults() {
        let mut app = make_app(1);
        app.state = AppState::Fullscreen;
        app.zoom = 3.0;
        app.pan_x = 10;
        app.pan_y = 5;
        app.zoom_reset();
        assert!((app.zoom - 1.0).abs() < 0.01);
        assert_eq!(app.pan_x, 0);
        assert_eq!(app.pan_y, 0);
    }

    #[test]
    fn zoom_ignored_in_browser_mode() {
        let mut app = make_app(5);
        app.zoom_in();
        assert!((app.zoom - 1.0).abs() < 0.01);
    }

    #[test]
    fn set_fullscreen_content_static_marks_zoom_dirty_and_resets_zoom_pan() {
        let mut app = make_app(1);
        app.state = AppState::Fullscreen;
        app.zoom = 2.0;
        app.pan_x = 4;
        app.pan_y = -3;
        app.zoom_dirty = false;

        app.set_fullscreen_content(
            make_static_content(400, 300),
            Some((400, 300)),
            Instant::now(),
        );

        assert!((app.zoom - 1.0).abs() < 0.01);
        assert_eq!(app.pan_x, 0);
        assert_eq!(app.pan_y, 0);
        assert!(app.zoom_dirty);
    }

    #[test]
    fn fullscreen_viewport_change_marks_static_content_dirty() {
        let mut app = make_app(1);
        app.state = AppState::Fullscreen;
        app.set_fullscreen_content(
            make_static_content(400, 300),
            Some((400, 300)),
            Instant::now(),
        );
        app.zoom_dirty = false;

        app.set_fullscreen_viewport(80, 40);
        assert!(app.zoom_dirty);

        app.zoom_dirty = false;
        app.set_fullscreen_viewport(80, 40);
        assert!(!app.zoom_dirty);

        app.set_fullscreen_viewport(81, 40);
        assert!(app.zoom_dirty);
    }

    #[test]
    fn fullscreen_viewport_change_does_not_dirty_animation() {
        let mut app = make_app(1);
        install_test_animation(&mut app, Instant::now());

        app.set_fullscreen_viewport(80, 40);

        assert!(!app.zoom_dirty);
    }

    #[test]
    fn regenerate_waits_until_viewport_is_known() {
        let mut app = make_app(1);
        app.state = AppState::Fullscreen;
        app.set_fullscreen_content(
            make_static_content(400, 300),
            Some((400, 300)),
            Instant::now(),
        );

        app.regenerate_if_dirty();
        assert!(app.zoom_dirty);

        app.set_fullscreen_viewport(80, 40);
        app.regenerate_if_dirty();
        assert!(!app.zoom_dirty);
    }

    #[test]
    fn pan_moves_in_correct_direction() {
        let mut app = make_app(1);
        app.state = AppState::Fullscreen;
        app.fullscreen_image_w = 80;
        app.fullscreen_image_h = 40;
        // Match the viewport aspect so both axes have pan room when zoomed.
        let img = image::RgbaImage::new(3000, 3000);
        app.picker = Picker::halfblocks();
        app.fullscreen_content = Some(FullscreenContent::Static(StaticContent {
            protocol: Some(make_protocol()), // placeholder
            original: Arc::new(img),
        }));
        // Zoom in so pan has room (at zoom 1.0, full image visible → no pan room)
        app.zoom_in(); // zoom = 1.1

        app.pan_right();
        assert!(app.pan_x > 0, "pan_right should increase pan_x");
        app.pan_x = 0;
        app.pan_left();
        assert!(app.pan_x < 0, "pan_left should decrease pan_x");
        app.pan_y = 0;
        app.pan_down();
        assert!(app.pan_y > 0, "pan_down should increase pan_y");
        app.pan_y = 0;
        app.pan_up();
        assert!(app.pan_y < 0, "pan_up should decrease pan_y");
    }

    #[test]
    fn zoom_geometry_fits_100_percent_to_viewport() {
        let geometry = zoom_render_geometry(4000, 3000, 800, 400, 1.0, 0, 0);

        assert_eq!(geometry.target_px_w, 533);
        assert_eq!(geometry.target_px_h, 400);
        assert_close(geometry.source_x, 0.0);
        assert_close(geometry.source_y, 0.0);
        assert_close(geometry.source_w, 4000.0);
        assert_close(geometry.source_h, 3000.0);
    }

    #[test]
    fn zoom_geometry_clamps_zoom_below_100_percent_to_fit() {
        let geometry = zoom_render_geometry(4000, 3000, 800, 400, 0.5, 0, 0);

        assert_eq!(geometry.target_px_w, 533);
        assert_eq!(geometry.target_px_h, 400);
        assert_close(geometry.source_w, 4000.0);
        assert_close(geometry.source_h, 3000.0);
    }

    #[test]
    fn zoom_geometry_crops_visible_viewport_from_scaled_whole_image() {
        let geometry = zoom_render_geometry(4000, 3000, 800, 400, 2.0, 0, 0);

        assert_eq!(geometry.target_px_w, 800);
        assert_eq!(geometry.target_px_h, 400);
        assert_close(geometry.source_x, 500.0);
        assert_close(geometry.source_y, 750.0);
        assert_close(geometry.source_w, 3000.0);
        assert_close(geometry.source_h, 1500.0);
        assert!(u64::from(geometry.target_px_w) * u64::from(geometry.target_px_h) <= 800 * 400);
    }

    #[test]
    fn zoom_geometry_does_not_stretch_in_tall_viewport() {
        let geometry = zoom_render_geometry(4000, 3000, 300, 800, 1.0, 0, 0);

        assert_eq!(geometry.target_px_w, 300);
        assert_eq!(geometry.target_px_h, 225);
    }

    #[test]
    fn zoom_geometry_pan_direction_matches_view_movement() {
        let centered = zoom_render_geometry(4000, 3000, 800, 400, 2.0, 0, 0);
        let right = zoom_render_geometry(4000, 3000, 800, 400, 2.0, 100, 0);
        let left = zoom_render_geometry(4000, 3000, 800, 400, 2.0, -100, 0);
        let down = zoom_render_geometry(4000, 3000, 800, 400, 2.0, 0, 100);
        let up = zoom_render_geometry(4000, 3000, 800, 400, 2.0, 0, -100);

        assert!(right.source_x > centered.source_x);
        assert!(left.source_x < centered.source_x);
        assert!(down.source_y > centered.source_y);
        assert!(up.source_y < centered.source_y);
    }

    #[test]
    fn pan_room_exists_only_on_overflow_axes() {
        let font_px = 10;
        let wide = zoom_display_geometry(4000, 1000, 800, 400, 1.5);
        assert!(max_pan_cells(wide.display_px_w, 800, font_px) > 0);
        assert_eq!(max_pan_cells(wide.display_px_h, 400, font_px), 0);

        let tall = zoom_display_geometry(1000, 4000, 800, 400, 1.5);
        assert_eq!(max_pan_cells(tall.display_px_w, 800, font_px), 0);
        assert!(max_pan_cells(tall.display_px_h, 400, font_px) > 0);

        let square = zoom_display_geometry(1000, 1000, 800, 400, 2.0);
        assert_eq!(max_pan_cells(square.display_px_w, 800, font_px), 0);
        assert!(max_pan_cells(square.display_px_h, 400, font_px) > 0);
    }

    #[test]
    fn clamp_pan_zeroes_axes_without_overflow_at_100_percent() {
        let mut app = make_app(1);
        app.state = AppState::Fullscreen;
        app.set_fullscreen_content(
            make_static_content(4000, 3000),
            Some((4000, 3000)),
            Instant::now(),
        );
        app.set_fullscreen_viewport(80, 40);
        app.pan_x = 10;
        app.pan_y = -10;

        app.clamp_pan();

        assert_eq!(app.pan_x, 0);
        assert_eq!(app.pan_y, 0);
    }

    #[test]
    fn clamp_pan_allows_only_axes_that_overflow_after_zoom() {
        let mut wide = make_app(1);
        wide.state = AppState::Fullscreen;
        wide.set_fullscreen_content(
            make_static_content(4000, 1000),
            Some((4000, 1000)),
            Instant::now(),
        );
        wide.set_fullscreen_viewport(80, 40);
        wide.zoom = 1.5;
        wide.pan_x = 1000;
        wide.pan_y = 1000;

        wide.clamp_pan();

        assert!(wide.pan_x > 0);
        assert_eq!(wide.pan_y, 0);

        let mut tall = make_app(1);
        tall.state = AppState::Fullscreen;
        tall.set_fullscreen_content(
            make_static_content(1000, 4000),
            Some((1000, 4000)),
            Instant::now(),
        );
        tall.set_fullscreen_viewport(80, 40);
        tall.zoom = 1.5;
        tall.pan_x = -1000;
        tall.pan_y = -1000;

        tall.clamp_pan();

        assert_eq!(tall.pan_x, 0);
        assert!(tall.pan_y < 0);
    }
}
