use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, BufReader};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{
    mpsc::{Receiver, Sender},
    Arc, Mutex,
};
use std::time::{Duration, Instant, SystemTime};

use crossterm::event::{KeyCode, KeyModifiers};
use fast_image_resize as fir;
use image::AnimationDecoder;
use lru::LruCache;
use ratatui::layout::Size;
use ratatui_image::{
    picker::Picker, protocol::Protocol, FilterType as ProtocolFilterType, FontSize, Resize,
};

use crate::favorites::FavoriteStore;
use crate::lang::Lang;
use crate::scanner::{image_entry_from_path, scan_directory, ImageEntry};
use crate::ui::search::{SearchAction, SearchState};

const MAX_ANIMATION_FRAMES: usize = 120;
const DEFAULT_FRAME_DELAY: Duration = Duration::from_millis(100);
const MIN_FRAME_DELAY: Duration = Duration::from_millis(33);

mod files;
mod fullscreen;
mod gallery;
mod input;
mod loader;
mod render;

pub use files::{DeleteState, RenameState};
#[allow(unused_imports)]
pub use gallery::{DirectoryContextEntry, DirectoryContextKind};
pub use loader::{
    spawn_image_loader, AnimationContent, AnimationFrame, FullscreenContent, ImageCacheKey,
    LoadControl, LoadRequest, LoadResult, LoadSize, OriginalLoadKind, StaticContent,
};

use fullscreen::{AnimationCacheKey, CachedAnimation, CachedOriginal};
use gallery::{default_favorite_store, sort_image_entries};
use loader::{animation_frame_estimated_bytes, load_content_is_terminal, LoadContent};
use render::{
    max_pan_cells, normalized_zoom, spawn_render_worker, zoom_display_geometry, zoom_percent,
    zoom_render_geometry, RenderDirtyReason, RenderKey, RenderQuality, RenderRequest, RenderResult,
    ZoomRenderGeometry,
};

#[cfg(test)]
use fullscreen::animation_cache_key;
#[cfg(test)]
use gallery::{browser_context_parent, browser_directory_context_entries};
#[cfg(test)]
use loader::{
    animation_content_from_frames, frame_delay, process_load_request_with_control,
    process_load_request_with_control_to_sender, process_original_request,
    process_thumbnail_request, static_original_content, try_decode_animation, OriginalRequestParts,
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GalleryMode {
    Directory,
    Favorites,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageSortMode {
    Name,
    Modified,
    Size,
}

impl ImageSortMode {
    fn next(self) -> Self {
        match self {
            ImageSortMode::Name => ImageSortMode::Modified,
            ImageSortMode::Modified => ImageSortMode::Size,
            ImageSortMode::Size => ImageSortMode::Name,
        }
    }
}

pub struct App {
    pub state: AppState,
    pub images: Vec<ImageEntry>,
    directory_images: Vec<ImageEntry>,
    pub sort_mode: ImageSortMode,
    pub gallery_mode: GalleryMode,
    pub image_dir: PathBuf,
    pub(crate) context_dir: PathBuf,
    pub selected: usize,
    pub scroll_row: usize,
    pub browser_focus: BrowserFocus,
    pub context_selected: usize,
    pub context_scroll: usize,
    context_visible_rows: usize,
    pub directory_generation: u64,
    pub protocol_cache: LruCache<ImageCacheKey, Protocol>,
    pub fullscreen_content: Option<FullscreenContent>,
    fullscreen_content_key: Option<ImageCacheKey>,
    fullscreen_frame_idx: usize,
    fullscreen_next_frame_at: Option<Instant>,
    pub fullscreen_pending: bool,
    pub fullscreen_dims: Option<(u32, u32)>,
    pub cache_width: u16,
    pub cache_height: u16,
    pub grid_cols: usize,
    favorite_row_len: usize,
    pub thumb_w: u16,
    pub thumb_h: u16,
    pub visible_rows: usize,
    pub requested: HashSet<(ImageCacheKey, LoadSize)>,
    pub search: Option<SearchState>,
    pub rename: Option<RenameState>,
    pub delete: Option<DeleteState>,
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
    fullscreen_original_cache: LruCache<ImageCacheKey, CachedOriginal>,
    fullscreen_original_cache_bytes: usize,
    animation_cache: LruCache<AnimationCacheKey, CachedAnimation>,
    animation_cache_bytes: usize,
    fullscreen_render_cache: LruCache<RenderKey, Protocol>,
    render_tx: Sender<RenderRequest>,
    render_rx: Receiver<RenderResult>,
    pub lang: Lang,
    load_tx: Sender<LoadRequest>,
    load_rx: Receiver<LoadResult>,
    load_control: LoadControl,
    status_message: Option<(String, Instant)>,
    favorites: FavoriteStore,
    directory_selected_path: Option<PathBuf>,
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
const ANIMATION_CACHE_BYTES: usize = 96 * 1024 * 1024;
const FULLSCREEN_RENDER_CACHE_SIZE: usize = 8;
const INTERACTIVE_SETTLE_DELAY: Duration = Duration::from_millis(120);
const DIRECT_FINAL_RENDER_PIXELS: u64 = 1_000_000;

impl App {
    #[cfg(test)]
    pub fn new(
        start: AppStart,
        load_tx: Sender<LoadRequest>,
        load_rx: Receiver<LoadResult>,
        lang: Lang,
        picker: Picker,
    ) -> Self {
        Self::new_with_load_control(start, load_tx, load_rx, lang, picker, LoadControl::new())
    }

    pub fn new_with_load_control(
        start: AppStart,
        load_tx: Sender<LoadRequest>,
        load_rx: Receiver<LoadResult>,
        lang: Lang,
        picker: Picker,
        load_control: LoadControl,
    ) -> Self {
        let AppStart {
            images,
            image_dir,
            state,
            selected,
        } = start;
        let sort_mode = ImageSortMode::Name;
        let selected_path = images.get(selected).map(|entry| entry.path.clone());
        let mut directory_images = images;
        sort_image_entries(&mut directory_images, sort_mode);
        let fullscreen_pending = state == AppState::Fullscreen;
        let (render_tx, render_rx) = spawn_render_worker(picker.clone());
        load_control.set_generation(0);
        let mut app = Self {
            state,
            images: Vec::new(),
            directory_images,
            sort_mode,
            gallery_mode: GalleryMode::Directory,
            context_dir: image_dir.clone(),
            image_dir,
            selected: 0,
            scroll_row: 0,
            browser_focus: BrowserFocus::Gallery,
            context_selected: 0,
            context_scroll: 0,
            context_visible_rows: 1,
            directory_generation: 0,
            protocol_cache: LruCache::new(NonZeroUsize::new(MAX_CACHE_SIZE).unwrap()),
            fullscreen_content: None,
            fullscreen_content_key: None,
            fullscreen_frame_idx: 0,
            fullscreen_next_frame_at: None,
            fullscreen_pending,
            fullscreen_dims: None,
            cache_width: 0,
            cache_height: 0,
            grid_cols: 8,
            favorite_row_len: 0,
            thumb_w: 0,
            thumb_h: 0,
            visible_rows: 1,
            requested: HashSet::new(),
            search: None,
            rename: None,
            delete: None,
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
            animation_cache: LruCache::unbounded(),
            animation_cache_bytes: 0,
            fullscreen_render_cache: LruCache::new(
                NonZeroUsize::new(FULLSCREEN_RENDER_CACHE_SIZE).unwrap(),
            ),
            render_tx,
            render_rx,
            lang,
            load_tx,
            load_rx,
            load_control,
            status_message: None,
            favorites: default_favorite_store(),
            directory_selected_path: None,
        };
        app.rebuild_directory_gallery(selected_path.clone());
        if app.images.is_empty() {
            app.selected = 0;
        } else if selected_path.is_none() {
            app.selected = selected.min(app.images.len().saturating_sub(1));
        }
        app.reset_context_selection_to_current_folder();
        // If launched directly into fullscreen (e.g. "termfoto image.png"),
        // immediately request the original load so the image appears.
        if fullscreen_pending {
            app.prepare_fullscreen_selection();
        }
        app
    }

    #[cfg(test)]
    pub fn set_favorite_store_path_for_tests(&mut self, path: PathBuf) {
        self.favorites = FavoriteStore::empty_at(path);
        let selected_path = self.current_selected_path();
        self.rebuild_active_gallery(selected_path);
    }

    #[cfg(test)]
    pub fn add_favorite_for_tests(&mut self, path: &Path, added_at_ms: u64) {
        self.favorites.add_at(path, added_at_ms).unwrap();
        let selected_path = self.current_selected_path();
        self.rebuild_active_gallery(selected_path);
    }
}

#[cfg(test)]
mod tests;
