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
    pub(super) fn empty() -> Self {
        Self {
            frames: Vec::new(),
            complete: false,
            estimated_bytes: 0,
        }
    }

    #[cfg(test)]
    pub(super) fn complete(frames: Vec<AnimationFrame>, font_size: FontSize) -> Self {
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
    pub(super) fn from_entry(entry: &ImageEntry) -> Self {
        Self {
            path: FavoriteStore::normalize_path(&entry.path),
            file_size: entry.file_size,
            modified_at: entry.modified_at,
        }
    }
}

/// Channel payload for a completed background image load.
pub struct LoadResult {
    pub(super) key: ImageCacheKey,
    pub(super) size: LoadSize,
    pub(super) generation: u64,
    pub(super) content: LoadContent,
    pub(super) dims: Option<(u32, u32)>,
}

pub(super) enum LoadContent {
    Thumbnail(Protocol),
    Original(FullscreenContent),
    AnimationStarted { dims: (u32, u32) },
    AnimationFrame { index: usize, frame: AnimationFrame },
    AnimationFinished { complete: bool },
    Skipped,
}

#[derive(Clone)]
pub struct LoadControl {
    inner: Arc<Mutex<LoadControlState>>,
}

#[derive(Default)]
struct LoadControlState {
    generation: u64,
    thumbnail_interest: Option<ThumbnailInterest>,
    original_interest: Option<OriginalInterest>,
}

struct ThumbnailInterest {
    w: u16,
    h: u16,
    keys: HashSet<ImageCacheKey>,
}

struct OriginalInterest {
    w: u16,
    h: u16,
    selected: Option<ImageCacheKey>,
    prefetch: HashSet<ImageCacheKey>,
}

impl LoadControl {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(LoadControlState::default())),
        }
    }

    pub(super) fn set_generation(&self, generation: u64) {
        let mut state = self.inner.lock().unwrap();
        state.generation = generation;
        state.thumbnail_interest = None;
        state.original_interest = None;
    }

    pub(super) fn update_thumbnail_interest<I>(&self, generation: u64, w: u16, h: u16, keys: I)
    where
        I: IntoIterator<Item = ImageCacheKey>,
    {
        let mut state = self.inner.lock().unwrap();
        ensure_load_generation(&mut state, generation);
        state.thumbnail_interest = Some(ThumbnailInterest {
            w,
            h,
            keys: keys.into_iter().collect(),
        });
    }

    pub(super) fn clear_thumbnail_interest(&self, generation: u64) {
        let mut state = self.inner.lock().unwrap();
        ensure_load_generation(&mut state, generation);
        state.thumbnail_interest = None;
    }

    pub(super) fn update_original_interest<I>(
        &self,
        generation: u64,
        w: u16,
        h: u16,
        selected: Option<ImageCacheKey>,
        prefetch: I,
    ) where
        I: IntoIterator<Item = ImageCacheKey>,
    {
        let mut state = self.inner.lock().unwrap();
        ensure_load_generation(&mut state, generation);
        state.original_interest = Some(OriginalInterest {
            w,
            h,
            selected,
            prefetch: prefetch.into_iter().collect(),
        });
    }

    pub(super) fn clear_original_interest(&self, generation: u64) {
        let mut state = self.inner.lock().unwrap();
        ensure_load_generation(&mut state, generation);
        state.original_interest = None;
    }

    pub(super) fn remove_interest_key(&self, generation: u64, key: &ImageCacheKey) {
        let mut state = self.inner.lock().unwrap();
        ensure_load_generation(&mut state, generation);
        if let Some(interest) = state.thumbnail_interest.as_mut() {
            interest.keys.remove(key);
        }
        if let Some(interest) = state.original_interest.as_mut() {
            if interest.selected.as_ref() == Some(key) {
                interest.selected = None;
            }
            interest.prefetch.remove(key);
        }
    }

    pub(super) fn allows(&self, req: &LoadRequest) -> bool {
        let state = self.inner.lock().unwrap();
        if req.generation != state.generation {
            return false;
        }

        match &req.size {
            LoadSize::Thumbnail { w, h } => {
                state.thumbnail_interest.as_ref().is_some_and(|interest| {
                    interest.w == *w && interest.h == *h && interest.keys.contains(&req.key)
                })
            }
            LoadSize::Original { w, h, kind } => {
                state.original_interest.as_ref().is_some_and(|interest| {
                    interest.w == *w
                        && interest.h == *h
                        && match kind {
                            OriginalLoadKind::Selected => {
                                interest.selected.as_ref() == Some(&req.key)
                            }
                            OriginalLoadKind::Prefetch => interest.prefetch.contains(&req.key),
                        }
                })
            }
        }
    }
}

impl Default for LoadControl {
    fn default() -> Self {
        Self::new()
    }
}

fn ensure_load_generation(state: &mut LoadControlState, generation: u64) {
    if state.generation != generation {
        state.generation = generation;
        state.thumbnail_interest = None;
        state.original_interest = None;
    }
}

/// Size mode for background image loading.
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

pub(super) struct OriginalRequestParts<'a> {
    pub(super) path: &'a Path,
    pub(super) key: ImageCacheKey,
    pub(super) generation: u64,
    pub(super) w: u16,
    pub(super) h: u16,
    pub(super) kind: OriginalLoadKind,
}

pub(super) fn load_content_is_terminal(content: &LoadContent) -> bool {
    matches!(
        content,
        LoadContent::Thumbnail(_)
            | LoadContent::Original(_)
            | LoadContent::AnimationFinished { .. }
            | LoadContent::Skipped
    )
}

pub(super) fn frame_delay(delay: image::Delay) -> Duration {
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
pub(super) fn static_original_content(img: image::DynamicImage) -> FullscreenContent {
    static_rgba_content(img.into_rgba8())
}

fn static_rgba_content(img: image::RgbaImage) -> FullscreenContent {
    FullscreenContent::Static(StaticContent {
        protocol: None,
        original: Arc::new(img),
    })
}

fn animation_frame_from_image_frame(
    picker: &Picker,
    frame: image::Frame,
    size: Size,
) -> Option<AnimationFrame> {
    let delay = frame_delay(frame.delay());
    let img = image::DynamicImage::ImageRgba8(frame.into_buffer());
    let protocol = make_protocol(picker, img, size, ProtocolFilterType::Nearest)?;
    Some(AnimationFrame { protocol, delay })
}

#[cfg(test)]
pub(super) fn animation_content_from_frames<I>(
    picker: &Picker,
    frames: I,
    size: Size,
) -> Option<AnimationContent>
where
    I: IntoIterator<Item = image::ImageResult<image::Frame>>,
{
    let mut animation_frames = Vec::new();
    for frame in frames {
        if animation_frames.len() == MAX_ANIMATION_FRAMES {
            return None;
        }
        let frame = frame.ok()?;
        animation_frames.push(animation_frame_from_image_frame(picker, frame, size)?);
    }

    if animation_frames.len() >= 2 {
        Some(AnimationContent {
            estimated_bytes: animation_frames_estimated_bytes(
                &animation_frames,
                picker.font_size(),
            ),
            frames: animation_frames,
            complete: true,
        })
    } else {
        None
    }
}

#[cfg(test)]
pub(super) fn try_decode_animation(
    picker: &Picker,
    path: &Path,
    size: Size,
) -> Option<AnimationContent> {
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

enum AnimationProbeOutcome {
    StaticFallback,
    Handled,
}

struct AnimationRequestContext<'a> {
    picker: &'a Picker,
    load_control: &'a LoadControl,
    done_tx: &'a Sender<LoadResult>,
    req: &'a LoadRequest,
    dims: (u32, u32),
    size: Size,
    kind: OriginalLoadKind,
}

fn try_handle_animation_original(
    ctx: &AnimationRequestContext<'_>,
    path: &Path,
) -> AnimationProbeOutcome {
    let Some(format) = image::ImageFormat::from_path(path).ok() else {
        return AnimationProbeOutcome::StaticFallback;
    };
    let Ok(file) = File::open(path) else {
        return AnimationProbeOutcome::StaticFallback;
    };
    let reader = BufReader::new(file);

    match format {
        image::ImageFormat::Gif => {
            let Ok(decoder) = image::codecs::gif::GifDecoder::new(reader) else {
                return AnimationProbeOutcome::StaticFallback;
            };
            handle_animation_frames(ctx, decoder.into_frames())
        }
        image::ImageFormat::Png => {
            let Ok(decoder) = image::codecs::png::PngDecoder::new(reader) else {
                return AnimationProbeOutcome::StaticFallback;
            };
            let Ok(decoder) = decoder.apng() else {
                return AnimationProbeOutcome::StaticFallback;
            };
            handle_animation_frames(ctx, decoder.into_frames())
        }
        image::ImageFormat::WebP => {
            let Ok(decoder) = image::codecs::webp::WebPDecoder::new(reader) else {
                return AnimationProbeOutcome::StaticFallback;
            };
            handle_animation_frames(ctx, decoder.into_frames())
        }
        _ => AnimationProbeOutcome::StaticFallback,
    }
}

fn handle_animation_frames<I>(ctx: &AnimationRequestContext<'_>, frames: I) -> AnimationProbeOutcome
where
    I: IntoIterator<Item = image::ImageResult<image::Frame>>,
{
    let mut frames = frames.into_iter();
    let Some(first) = frames.next() else {
        return AnimationProbeOutcome::StaticFallback;
    };
    let Some(first) = first
        .ok()
        .and_then(|frame| animation_frame_from_image_frame(ctx.picker, frame, ctx.size))
    else {
        return AnimationProbeOutcome::StaticFallback;
    };
    if !ctx.load_control.allows(ctx.req) {
        let _ = ctx.done_tx.send(skipped_load_result(ctx.req.clone()));
        return AnimationProbeOutcome::Handled;
    }

    let Some(second) = frames.next() else {
        return AnimationProbeOutcome::StaticFallback;
    };
    let Some(second) = second
        .ok()
        .and_then(|frame| animation_frame_from_image_frame(ctx.picker, frame, ctx.size))
    else {
        return AnimationProbeOutcome::StaticFallback;
    };
    if !ctx.load_control.allows(ctx.req) {
        let _ = ctx.done_tx.send(skipped_load_result(ctx.req.clone()));
        return AnimationProbeOutcome::Handled;
    }

    if ctx.kind == OriginalLoadKind::Prefetch {
        let _ = ctx.done_tx.send(skipped_load_result(ctx.req.clone()));
        return AnimationProbeOutcome::Handled;
    }

    if !send_animation_started(ctx.done_tx, ctx.req, ctx.dims) {
        return AnimationProbeOutcome::Handled;
    }
    if !send_animation_frame(ctx.done_tx, ctx.req, 0, first) {
        return AnimationProbeOutcome::Handled;
    }
    if !ctx.load_control.allows(ctx.req) {
        let _ = send_animation_finished(ctx.done_tx, ctx.req, false);
        return AnimationProbeOutcome::Handled;
    }
    if !send_animation_frame(ctx.done_tx, ctx.req, 1, second) {
        return AnimationProbeOutcome::Handled;
    }

    for (frame_count, frame) in (2usize..).zip(frames) {
        if !ctx.load_control.allows(ctx.req) {
            let _ = send_animation_finished(ctx.done_tx, ctx.req, false);
            return AnimationProbeOutcome::Handled;
        }
        if frame_count == MAX_ANIMATION_FRAMES {
            let _ = send_animation_finished(ctx.done_tx, ctx.req, false);
            return AnimationProbeOutcome::Handled;
        }
        let Some(frame) = frame
            .ok()
            .and_then(|frame| animation_frame_from_image_frame(ctx.picker, frame, ctx.size))
        else {
            let _ = send_animation_finished(ctx.done_tx, ctx.req, false);
            return AnimationProbeOutcome::Handled;
        };
        if !send_animation_frame(ctx.done_tx, ctx.req, frame_count, frame) {
            return AnimationProbeOutcome::Handled;
        }
    }

    if !ctx.load_control.allows(ctx.req) {
        let _ = send_animation_finished(ctx.done_tx, ctx.req, false);
        return AnimationProbeOutcome::Handled;
    }
    let _ = send_animation_finished(ctx.done_tx, ctx.req, true);
    AnimationProbeOutcome::Handled
}

fn send_animation_started(
    done_tx: &Sender<LoadResult>,
    req: &LoadRequest,
    dims: (u32, u32),
) -> bool {
    done_tx
        .send(LoadResult {
            key: req.key.clone(),
            size: req.size.clone(),
            generation: req.generation,
            content: LoadContent::AnimationStarted { dims },
            dims: None,
        })
        .is_ok()
}

fn send_animation_frame(
    done_tx: &Sender<LoadResult>,
    req: &LoadRequest,
    index: usize,
    frame: AnimationFrame,
) -> bool {
    done_tx
        .send(LoadResult {
            key: req.key.clone(),
            size: req.size.clone(),
            generation: req.generation,
            content: LoadContent::AnimationFrame { index, frame },
            dims: None,
        })
        .is_ok()
}

fn send_animation_finished(
    done_tx: &Sender<LoadResult>,
    req: &LoadRequest,
    complete: bool,
) -> bool {
    done_tx
        .send(LoadResult {
            key: req.key.clone(),
            size: req.size.clone(),
            generation: req.generation,
            content: LoadContent::AnimationFinished { complete },
            dims: None,
        })
        .is_ok()
}

#[cfg(test)]
pub(super) fn animation_frames_estimated_bytes(
    frames: &[AnimationFrame],
    font_size: FontSize,
) -> usize {
    frames
        .iter()
        .map(|frame| animation_frame_estimated_bytes(frame, font_size))
        .sum()
}

pub(super) fn animation_frame_estimated_bytes(
    frame: &AnimationFrame,
    font_size: FontSize,
) -> usize {
    let size = frame.protocol.size();
    usize::from(size.width.max(1))
        .saturating_mul(usize::from(size.height.max(1)))
        .saturating_mul(usize::from(font_size.width.max(1)))
        .saturating_mul(usize::from(font_size.height.max(1)))
        .saturating_mul(4)
}

/// Spawn background workers that load thumbnails separately from fullscreen originals.
/// Returns (sender, receiver) for App to use.
pub fn spawn_image_loader(
    picker: Picker,
    _paths: Vec<std::path::PathBuf>,
    load_control: LoadControl,
) -> (Sender<LoadRequest>, Receiver<LoadResult>) {
    let (load_tx, load_rx) = std::sync::mpsc::channel::<LoadRequest>();
    let (done_tx, done_rx) = std::sync::mpsc::channel::<LoadResult>();

    let (thumb_tx, thumb_rx) = std::sync::mpsc::channel::<LoadRequest>();
    let (original_tx, original_rx) = std::sync::mpsc::channel::<LoadRequest>();

    std::thread::spawn(move || {
        while let Ok(req) = load_rx.recv() {
            let routed = match &req.size {
                LoadSize::Thumbnail { .. } => thumb_tx.send(req),
                LoadSize::Original { .. } => original_tx.send(req),
            };
            if routed.is_err() {
                break;
            }
        }
    });

    spawn_loader_workers(
        picker.clone(),
        done_tx.clone(),
        thumb_rx,
        load_control.clone(),
        3,
    );
    spawn_loader_workers(picker, done_tx, original_rx, load_control, 1);

    (load_tx, done_rx)
}

fn spawn_loader_workers(
    picker: Picker,
    done_tx: Sender<LoadResult>,
    load_rx: Receiver<LoadRequest>,
    load_control: LoadControl,
    workers: usize,
) {
    let rx = Arc::new(std::sync::Mutex::new(load_rx));
    for _ in 0..workers {
        let picker = picker.clone();
        let done_tx = done_tx.clone();
        let rx = Arc::clone(&rx);
        let load_control = load_control.clone();

        std::thread::spawn(move || loop {
            let req = {
                let rx = rx.lock().unwrap();
                match rx.recv() {
                    Ok(req) => req,
                    Err(_) => return,
                }
            };

            process_load_request_with_control_to_sender(&picker, &load_control, req, &done_tx);
        });
    }
}

#[cfg(test)]
pub(super) fn process_load_request_with_control(
    picker: &Picker,
    load_control: &LoadControl,
    req: LoadRequest,
) -> Option<LoadResult> {
    let (done_tx, done_rx) = std::sync::mpsc::channel::<LoadResult>();
    process_load_request_with_control_to_sender(picker, load_control, req, &done_tx);
    done_rx.try_recv().ok()
}

pub(super) fn process_load_request_with_control_to_sender(
    picker: &Picker,
    load_control: &LoadControl,
    req: LoadRequest,
    done_tx: &Sender<LoadResult>,
) {
    if !load_control.allows(&req) {
        let _ = done_tx.send(skipped_load_result(req));
        return;
    }

    if let Some(result) = process_load_request(picker, load_control, req, done_tx) {
        let _ = done_tx.send(result);
    }
}

fn skipped_load_result(req: LoadRequest) -> LoadResult {
    let LoadRequest {
        key,
        size,
        generation,
        ..
    } = req;

    LoadResult {
        key,
        size,
        generation,
        content: LoadContent::Skipped,
        dims: None,
    }
}

fn process_load_request(
    picker: &Picker,
    load_control: &LoadControl,
    req: LoadRequest,
    done_tx: &Sender<LoadResult>,
) -> Option<LoadResult> {
    let LoadRequest {
        key,
        path,
        size,
        generation,
        ..
    } = req;
    match size {
        LoadSize::Thumbnail { w, h } => {
            process_thumbnail_request(picker, path.as_path(), key, generation, w, h)
        }
        LoadSize::Original { w, h, kind } => process_original_request(
            picker,
            load_control,
            done_tx,
            OriginalRequestParts {
                path: path.as_path(),
                key,
                generation,
                w,
                h,
                kind,
            },
        ),
    }
}

pub(super) fn process_thumbnail_request(
    picker: &Picker,
    path: &Path,
    key: ImageCacheKey,
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
        key,
        size: LoadSize::Thumbnail { w, h },
        generation,
        content: LoadContent::Thumbnail(protocol),
        dims,
    })
}

pub(super) fn process_original_request(
    picker: &Picker,
    load_control: &LoadControl,
    done_tx: &Sender<LoadResult>,
    parts: OriginalRequestParts<'_>,
) -> Option<LoadResult> {
    let OriginalRequestParts {
        path,
        key,
        generation,
        w,
        h,
        kind,
    } = parts;
    let size = LoadSize::Original { w, h, kind };
    let req = LoadRequest {
        key: key.clone(),
        path: path.to_path_buf(),
        size: size.clone(),
        generation,
    };
    let dims = image::image_dimensions(path).ok()?;
    let protocol_size = Size::new(w.max(1), h.max(1));

    if should_probe_animation(path) {
        let animation_ctx = AnimationRequestContext {
            picker,
            load_control,
            done_tx,
            dims,
            size: protocol_size,
            kind,
            req: &req,
        };
        match try_handle_animation_original(&animation_ctx, path) {
            AnimationProbeOutcome::Handled => return None,
            AnimationProbeOutcome::StaticFallback => {}
        }
    }

    if !load_control.allows(&req) {
        return Some(skipped_load_result(req));
    }
    let content = static_rgba_content(image::open(path).ok()?.into_rgba8());
    if !load_control.allows(&req) {
        return Some(skipped_load_result(req));
    }

    Some(LoadResult {
        key,
        size,
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
