use super::*;

#[derive(Clone)]
pub struct AnimationFrame {
    pub protocol: Protocol,
    pub delay: Duration,
}

#[derive(Clone)]
pub struct AnimationContent {
    pub frames: Vec<AnimationFrame>,
    pub complete: bool,
    pub estimated_bytes: usize,
}

impl AnimationContent {
    pub(in crate::app) fn empty() -> Self {
        Self {
            frames: Vec::new(),
            complete: false,
            estimated_bytes: 0,
        }
    }

    #[cfg(test)]
    pub(in crate::app) fn complete(frames: Vec<AnimationFrame>, font_size: FontSize) -> Self {
        let estimated_bytes = animation_frames_estimated_bytes(&frames, font_size);
        Self {
            frames,
            complete: true,
            estimated_bytes,
        }
    }
}

#[derive(Clone)]
pub struct StaticContent {
    pub protocol: Option<Protocol>,
    pub original: Arc<image::RgbaImage>,
}

#[derive(Clone)]
pub enum FullscreenContent {
    Static(StaticContent),
    Animation(AnimationContent),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageCacheKey {
    path: PathBuf,
    file_size: u64,
    modified_at: Option<SystemTime>,
}

impl ImageCacheKey {
    pub(in crate::app) fn from_entry(entry: &ImageEntry) -> Self {
        Self {
            path: FavoriteStore::normalize_path(&entry.path),
            file_size: entry.file_size,
            modified_at: entry.modified_at,
        }
    }
}

/// Channel payload for a completed background image load.
pub struct LoadResult {
    pub(in crate::app) key: ImageCacheKey,
    pub(in crate::app) size: LoadSize,
    pub(in crate::app) generation: u64,
    pub(in crate::app) content: LoadContent,
    pub(in crate::app) dims: Option<(u32, u32)>,
}

pub(in crate::app) enum LoadContent {
    Thumbnail(Protocol),
    Original(FullscreenContent),
    AnimationStarted { dims: (u32, u32) },
    AnimationFrame { index: usize, frame: AnimationFrame },
    AnimationFinished { complete: bool },
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LoadSize {
    /// Browser thumbnail at fixed cell dimensions.
    Thumbnail { w: u16, h: u16 },
    /// Fullscreen original load for a known viewport.
    Original {
        w: u16,
        h: u16,
        kind: OriginalLoadKind,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OriginalLoadKind {
    Selected,
    Prefetch,
}

/// A request sent to the background loader.
#[derive(Debug, Clone)]
pub struct LoadRequest {
    pub key: ImageCacheKey,
    pub path: PathBuf,
    pub size: LoadSize,
    pub generation: u64,
}
