use super::*;

pub(super) fn make_app(count: usize) -> App {
    let images = (0..count)
        .map(|i| ImageEntry {
            path: PathBuf::from(format!("img{:03}.png", i)),
            filename: format!("img{:03}.png", i),
            file_size: 0,
            modified_at: None,
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

pub(super) fn make_app_with_load_rx(count: usize) -> (App, Receiver<LoadRequest>) {
    let images = (0..count)
        .map(|i| ImageEntry {
            path: PathBuf::from(format!("img{:03}.png", i)),
            filename: format!("img{:03}.png", i),
            file_size: 0,
            modified_at: None,
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

pub(super) fn make_app_with_load_done(count: usize) -> (App, Sender<LoadResult>) {
    let images = (0..count)
        .map(|i| ImageEntry {
            path: PathBuf::from(format!("img{:03}.png", i)),
            filename: format!("img{:03}.png", i),
            file_size: 0,
            modified_at: None,
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

pub(super) fn test_entry(name: &str, file_size: u64, modified_secs: Option<u64>) -> ImageEntry {
    ImageEntry {
        path: PathBuf::from(name),
        filename: name.to_string(),
        file_size,
        modified_at: modified_secs.map(|secs| UNIX_EPOCH + Duration::from_secs(secs)),
    }
}

pub(super) fn test_key(name: &str) -> ImageCacheKey {
    ImageCacheKey::from_entry(&test_entry(name, 0, None))
}

pub(super) fn app_key(app: &App, idx: usize) -> ImageCacheKey {
    app.image_cache_key_for_slot(idx).unwrap()
}

pub(super) fn path_key(path: &Path) -> ImageCacheKey {
    let entry = image_entry_from_path(path).unwrap_or_else(|| ImageEntry {
        path: path.to_path_buf(),
        filename: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        file_size: 0,
        modified_at: None,
    });
    ImageCacheKey::from_entry(&entry)
}

pub(super) fn make_app_with_entries(images: Vec<ImageEntry>) -> App {
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

pub(super) fn image_names(app: &App) -> Vec<&str> {
    app.images
        .iter()
        .map(|entry| entry.filename.as_str())
        .collect()
}

pub(super) fn make_protocol() -> Protocol {
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

pub(super) fn make_static_content(width: u32, height: u32) -> FullscreenContent {
    FullscreenContent::Static(StaticContent {
        protocol: Some(make_protocol()),
        original: Arc::new(image::RgbaImage::new(width, height)),
    })
}

pub(super) fn write_png(path: &Path) {
    image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        1,
        1,
        image::Rgba([1, 2, 3, 255]),
    ))
    .save(path)
    .unwrap();
}

pub(super) fn make_app_for_dir(
    dir: &Path,
    selected: usize,
    state: AppState,
) -> (App, Receiver<LoadRequest>) {
    let images = scan_directory(dir).unwrap();
    let (tx, rx) = std::sync::mpsc::channel::<LoadRequest>();
    let (_tx2, rx2) = std::sync::mpsc::channel::<LoadResult>();
    (
        App::new(
            AppStart {
                images,
                image_dir: dir.to_path_buf(),
                state,
                selected,
            },
            tx,
            rx2,
            Lang::En,
            Picker::halfblocks(),
        ),
        rx,
    )
}

pub(super) fn set_rename_input(app: &mut App, value: &str) {
    let current_len = app.rename.as_ref().unwrap().input.chars().count();
    for _ in 0..current_len {
        app.handle_key(KeyCode::Backspace, KeyModifiers::NONE);
    }
    for ch in value.chars() {
        app.handle_key(KeyCode::Char(ch), KeyModifiers::NONE);
    }
}

pub(super) fn missing_load_request(_idx: usize, size: LoadSize, generation: u64) -> LoadRequest {
    let path = PathBuf::from("missing.png");
    LoadRequest {
        key: path_key(&path),
        path,
        size,
        generation,
    }
}

pub(super) fn assert_skipped(result: LoadResult, _idx: usize, size: LoadSize, generation: u64) {
    assert_eq!(result.size, size);
    assert_eq!(result.generation, generation);
    assert!(matches!(result.content, LoadContent::Skipped));
    assert!(result.dims.is_none());
}

pub(super) fn isolate_favorites(app: &mut App, dir: &Path) {
    app.set_favorite_store_path_for_tests(dir.join("favorites.tsv"));
}

pub(super) fn make_animation_frame(delay_ms: u64) -> AnimationFrame {
    AnimationFrame {
        protocol: make_protocol(),
        delay: Duration::from_millis(delay_ms),
    }
}

pub(super) fn animation_content(frames: Vec<AnimationFrame>) -> AnimationContent {
    AnimationContent::complete(frames, Picker::halfblocks().font_size())
}

pub(super) fn selected_original_size(w: u16, h: u16) -> LoadSize {
    LoadSize::Original {
        w,
        h,
        kind: OriginalLoadKind::Selected,
    }
}

pub(super) fn prefetch_original_size(w: u16, h: u16) -> LoadSize {
    LoadSize::Original {
        w,
        h,
        kind: OriginalLoadKind::Prefetch,
    }
}

pub(super) fn make_image_frame(delay_ms: u32) -> image::Frame {
    image::Frame::from_parts(
        image::RgbaImage::new(1, 1),
        0,
        0,
        image::Delay::from_numer_denom_ms(delay_ms, 1),
    )
}

pub(super) fn make_colored_image_frame(delay_ms: u32, color: [u8; 4]) -> image::Frame {
    image::Frame::from_parts(
        image::RgbaImage::from_pixel(1, 1, image::Rgba(color)),
        0,
        0,
        image::Delay::from_numer_denom_ms(delay_ms, 1),
    )
}

pub(super) fn write_gif(path: &Path, frames: Vec<image::Frame>) {
    let file = File::create(path).unwrap();
    let mut encoder = image::codecs::gif::GifEncoder::new(file);
    encoder.encode_frames(frames).unwrap();
}

pub(super) fn install_test_animation(app: &mut App, now: Instant) {
    app.state = AppState::Fullscreen;
    app.set_fullscreen_content(
        FullscreenContent::Animation(animation_content(vec![
            make_animation_frame(100),
            make_animation_frame(150),
        ])),
        Some((1, 1)),
        now,
    );
}

pub(super) fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 0.01,
        "expected {actual} to be close to {expected}"
    );
}

pub(super) fn make_app_with_names(names: &[&str]) -> App {
    let images: Vec<ImageEntry> = names
        .iter()
        .map(|name| ImageEntry {
            path: PathBuf::from(name),
            filename: name.to_string(),
            file_size: 0,
            modified_at: None,
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
