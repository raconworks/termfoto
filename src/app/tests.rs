use super::*;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant, UNIX_EPOCH};
use tempfile::tempdir;

fn make_app(count: usize) -> App {
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

fn make_app_with_load_rx(count: usize) -> (App, Receiver<LoadRequest>) {
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

fn make_app_with_load_done(count: usize) -> (App, Sender<LoadResult>) {
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

fn test_entry(name: &str, file_size: u64, modified_secs: Option<u64>) -> ImageEntry {
    ImageEntry {
        path: PathBuf::from(name),
        filename: name.to_string(),
        file_size,
        modified_at: modified_secs.map(|secs| UNIX_EPOCH + Duration::from_secs(secs)),
    }
}

fn test_key(name: &str) -> ImageCacheKey {
    ImageCacheKey::from_entry(&test_entry(name, 0, None))
}

fn app_key(app: &App, idx: usize) -> ImageCacheKey {
    app.image_cache_key_for_slot(idx).unwrap()
}

fn path_key(path: &Path) -> ImageCacheKey {
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

fn make_app_with_entries(images: Vec<ImageEntry>) -> App {
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

fn image_names(app: &App) -> Vec<&str> {
    app.images
        .iter()
        .map(|entry| entry.filename.as_str())
        .collect()
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

fn make_app_for_dir(dir: &Path, selected: usize, state: AppState) -> (App, Receiver<LoadRequest>) {
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

fn set_rename_input(app: &mut App, value: &str) {
    let current_len = app.rename.as_ref().unwrap().input.chars().count();
    for _ in 0..current_len {
        app.handle_key(KeyCode::Backspace, KeyModifiers::NONE);
    }
    for ch in value.chars() {
        app.handle_key(KeyCode::Char(ch), KeyModifiers::NONE);
    }
}

fn missing_load_request(_idx: usize, size: LoadSize, generation: u64) -> LoadRequest {
    let path = PathBuf::from("missing.png");
    LoadRequest {
        key: path_key(&path),
        path,
        size,
        generation,
    }
}

fn assert_skipped(result: LoadResult, _idx: usize, size: LoadSize, generation: u64) {
    assert_eq!(result.size, size);
    assert_eq!(result.generation, generation);
    assert!(matches!(result.content, LoadContent::Skipped));
    assert!(result.dims.is_none());
}

fn isolate_favorites(app: &mut App, dir: &Path) {
    app.set_favorite_store_path_for_tests(dir.join("favorites.tsv"));
}

#[test]
fn sort_mode_defaults_to_name_order() {
    let app = make_app_with_entries(vec![
        test_entry("zebra.png", 1, Some(1)),
        test_entry("apple.png", 1, Some(3)),
        test_entry("mango.png", 1, Some(2)),
    ]);

    assert_eq!(app.sort_mode, ImageSortMode::Name);
    assert_eq!(
        image_names(&app),
        vec!["apple.png", "mango.png", "zebra.png"]
    );
}

#[test]
fn sort_key_cycles_modes() {
    let mut app = make_app(0);

    assert_eq!(app.sort_mode, ImageSortMode::Name);
    app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);
    assert_eq!(app.sort_mode, ImageSortMode::Modified);
    app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);
    assert_eq!(app.sort_mode, ImageSortMode::Size);
    app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);
    assert_eq!(app.sort_mode, ImageSortMode::Name);
}

#[test]
fn modified_sort_is_newest_first_and_missing_last() {
    let mut app = make_app_with_entries(vec![
        test_entry("missing.png", 1, None),
        test_entry("old.png", 1, Some(10)),
        test_entry("new.png", 1, Some(30)),
    ]);

    app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);

    assert_eq!(app.sort_mode, ImageSortMode::Modified);
    assert_eq!(image_names(&app), vec!["new.png", "old.png", "missing.png"]);
}

#[test]
fn size_sort_is_largest_first() {
    let mut app = make_app_with_entries(vec![
        test_entry("small.png", 5, Some(10)),
        test_entry("large.png", 50, Some(30)),
        test_entry("medium.png", 25, Some(20)),
    ]);

    app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);

    assert_eq!(app.sort_mode, ImageSortMode::Size);
    assert_eq!(
        image_names(&app),
        vec!["large.png", "medium.png", "small.png"]
    );
}

#[test]
fn image_cache_key_includes_file_metadata() {
    let original = ImageCacheKey::from_entry(&test_entry("same.png", 10, Some(1)));
    let same = ImageCacheKey::from_entry(&test_entry("same.png", 10, Some(1)));
    let resized = ImageCacheKey::from_entry(&test_entry("same.png", 11, Some(1)));
    let modified = ImageCacheKey::from_entry(&test_entry("same.png", 10, Some(2)));

    assert_eq!(original, same);
    assert_ne!(original, resized);
    assert_ne!(original, modified);
}

#[test]
fn sorting_preserves_selected_image_and_image_caches() {
    let mut app = make_app_with_entries(vec![
        test_entry("a.png", 1, Some(10)),
        test_entry("b.png", 1, Some(30)),
        test_entry("c.png", 1, Some(20)),
    ]);
    app.grid_cols = 1;
    app.visible_rows = 1;
    app.selected = 0;
    app.scroll_row = 0;
    let first_key = app_key(&app, 0);
    app.protocol_cache.put(first_key.clone(), make_protocol());
    app.requested
        .insert((first_key, LoadSize::Thumbnail { w: 1, h: 1 }));
    let generation = app.directory_generation;

    app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);

    assert_eq!(image_names(&app), vec!["b.png", "c.png", "a.png"]);
    assert_eq!(app.images[app.selected].filename, "a.png");
    assert_eq!(app.selected, 2);
    assert_eq!(app.scroll_row, 2);
    assert!(!app.protocol_cache.is_empty());
    assert!(!app.requested.is_empty());
    assert_eq!(app.directory_generation, generation);
}

#[test]
fn sort_key_works_from_context_focus() {
    let mut app = make_app(1);
    app.browser_focus = BrowserFocus::Context;

    app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);

    assert_eq!(app.sort_mode, ImageSortMode::Modified);
}

#[test]
fn search_mode_keeps_s_as_query_text() {
    let mut app = make_app_with_names(&["sample.png"]);

    app.handle_key(KeyCode::Char('/'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);

    assert_eq!(app.sort_mode, ImageSortMode::Name);
    assert_eq!(app.search.as_ref().unwrap().query, "s");
}

#[test]
fn empty_directory_can_cycle_sort_mode() {
    let mut app = make_app(0);

    app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);

    assert_eq!(app.sort_mode, ImageSortMode::Modified);
    assert_eq!(app.selected, 0);
    assert_eq!(app.sort_label(), "修改时间");
}

#[test]
fn entering_directory_keeps_current_sort_mode() {
    let dir = tempdir().unwrap();
    let child = dir.path().join("child");
    fs::create_dir(&child).unwrap();
    fs::write(child.join("small.png"), b"1").unwrap();
    fs::write(child.join("large.png"), b"123456789").unwrap();

    let mut app = make_app(0);
    app.sort_mode = ImageSortMode::Size;
    app.enter_directory(child);

    assert_eq!(app.sort_mode, ImageSortMode::Size);
    assert_eq!(image_names(&app), vec!["large.png", "small.png"]);
}

#[test]
fn thumbnail_load_results_cache_after_sort_by_key() {
    let (mut app, done_tx) = make_app_with_load_done(2);
    let generation = app.directory_generation;
    let key = app_key(&app, 0);
    app.thumb_w = 1;
    app.thumb_h = 1;

    app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);
    done_tx
        .send(LoadResult {
            key: key.clone(),
            size: LoadSize::Thumbnail { w: 1, h: 1 },
            generation,
            content: LoadContent::Thumbnail(make_protocol()),
            dims: Some((1, 1)),
        })
        .unwrap();
    app.collect_loads();

    assert!(app.protocol_cache.contains(&key));
}

#[test]
fn browser_directory_context_lists_current_child_directories_only() {
    let dir = tempdir().unwrap();
    let photos = dir.path().join("photos");
    fs::create_dir(&photos).unwrap();
    fs::create_dir(photos.join("z_album")).unwrap();
    fs::create_dir(photos.join("a_album")).unwrap();
    fs::create_dir(photos.join(".hidden")).unwrap();
    fs::write(photos.join("note.txt"), b"note").unwrap();
    write_png(&photos.join("photo.png"));

    let mut app = make_app(0);
    app.image_dir = photos.clone();
    app.context_dir = photos;

    let entries = app.directory_context_for_browser();
    let names: Vec<_> = entries.iter().map(|entry| entry.name.as_str()).collect();

    assert_eq!(names, vec!["photos", "a_album", "z_album"]);
    assert!(!names.contains(&".hidden"));
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

fn animation_content(frames: Vec<AnimationFrame>) -> AnimationContent {
    AnimationContent::complete(frames, Picker::halfblocks().font_size())
}

fn selected_original_size(w: u16, h: u16) -> LoadSize {
    LoadSize::Original {
        w,
        h,
        kind: OriginalLoadKind::Selected,
    }
}

fn prefetch_original_size(w: u16, h: u16) -> LoadSize {
    LoadSize::Original {
        w,
        h,
        kind: OriginalLoadKind::Prefetch,
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

fn write_gif(path: &Path, frames: Vec<image::Frame>) {
    let file = File::create(path).unwrap();
    let mut encoder = image::codecs::gif::GifEncoder::new(file);
    encoder.encode_frames(frames).unwrap();
}

fn install_test_animation(app: &mut App, now: Instant) {
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
fn exiting_fullscreen_clears_pending_original_request_for_reentry() {
    let (mut app, rx) = make_app_with_load_rx(1);

    app.enter_fullscreen();
    app.set_fullscreen_viewport(80, 40);
    let first = rx.try_recv().unwrap();
    let original_size = selected_original_size(80, 40);
    assert_eq!(first.size, original_size);
    let key = app_key(&app, 0);
    assert!(app
        .requested
        .contains(&(key.clone(), original_size.clone())));

    app.exit_fullscreen();
    assert!(!app.requested.contains(&(key, original_size)));

    app.enter_fullscreen();
    app.set_fullscreen_viewport(80, 40);
    let second = rx.try_recv().unwrap();
    assert_eq!(second.size, selected_original_size(80, 40));
    assert_eq!(second.generation, app.directory_generation);
    assert!(rx.try_recv().is_err());
}

#[test]
fn thumbnail_request_does_not_block_original_request() {
    let (mut app, rx) = make_app_with_load_rx(1);

    app.request_load(0, LoadSize::Thumbnail { w: 10, h: 5 });
    app.request_load(0, selected_original_size(80, 40));

    let thumb = rx.try_recv().unwrap();
    assert_eq!(thumb.path, PathBuf::from("img000.png"));
    assert_eq!(thumb.generation, app.directory_generation);
    assert_eq!(thumb.size, LoadSize::Thumbnail { w: 10, h: 5 });

    let original = rx.try_recv().unwrap();
    assert_eq!(original.path, PathBuf::from("img000.png"));
    assert_eq!(original.generation, app.directory_generation);
    assert_eq!(original.size, selected_original_size(80, 40));
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
        Some(animation) => {
            assert!(animation.complete);
            assert_eq!(animation.frames.len(), 2);
            assert_eq!(animation.frames[0].delay, Duration::from_millis(100));
            assert_eq!(animation.frames[1].delay, Duration::from_millis(150));
        }
        None => panic!("expected animation content"),
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
fn tiny_frame_delay_clamps_to_33ms() {
    assert_eq!(
        frame_delay(image::Delay::from_numer_denom_ms(1, 1)),
        Duration::from_millis(33)
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
        Some(animation) => assert_eq!(animation.frames.len(), 2),
        None => panic!("expected animated GIF content"),
    }
}

#[test]
fn selected_original_request_streams_animation_for_gif() {
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
    let control = LoadControl::new();
    let size = selected_original_size(4, 2);
    control.update_original_interest(11, 4, 2, Some(path_key(&path)), Vec::<ImageCacheKey>::new());
    let (done_tx, done_rx) = std::sync::mpsc::channel::<LoadResult>();
    process_load_request_with_control_to_sender(
        &picker,
        &control,
        LoadRequest {
            key: path_key(&path),
            path: path.clone(),
            size: size.clone(),
            generation: 11,
        },
        &done_tx,
    );

    let started = done_rx.try_recv().unwrap();
    assert_eq!(started.size, size);
    assert_eq!(started.generation, 11);
    assert!(matches!(
        started.content,
        LoadContent::AnimationStarted { dims: (1, 1) }
    ));
    let first = done_rx.try_recv().unwrap();
    assert!(matches!(
        first.content,
        LoadContent::AnimationFrame { index: 0, .. }
    ));
    let second = done_rx.try_recv().unwrap();
    assert!(matches!(
        second.content,
        LoadContent::AnimationFrame { index: 1, .. }
    ));
    let finished = done_rx.try_recv().unwrap();
    assert!(matches!(
        finished.content,
        LoadContent::AnimationFinished { complete: true }
    ));
}

#[test]
fn single_frame_gif_falls_back_to_static_original() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("single.gif");
    write_gif(&path, vec![make_colored_image_frame(100, [255, 0, 0, 255])]);

    let picker = Picker::halfblocks();
    let control = LoadControl::new();
    let size = selected_original_size(4, 2);
    control.update_original_interest(21, 4, 2, Some(path_key(&path)), Vec::<ImageCacheKey>::new());
    let (done_tx, done_rx) = std::sync::mpsc::channel::<LoadResult>();
    process_load_request_with_control_to_sender(
        &picker,
        &control,
        LoadRequest {
            key: path_key(&path),
            path: path.clone(),
            size: size.clone(),
            generation: 21,
        },
        &done_tx,
    );

    let result = done_rx.try_recv().unwrap();
    assert_eq!(result.size, size);
    assert!(matches!(
        result.content,
        LoadContent::Original(FullscreenContent::Static(_))
    ));
    assert!(done_rx.try_recv().is_err());
}

#[test]
fn prefetch_skips_real_animation_after_probe() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("animated.gif");
    write_gif(
        &path,
        vec![
            make_colored_image_frame(100, [255, 0, 0, 255]),
            make_colored_image_frame(120, [0, 255, 0, 255]),
        ],
    );

    let picker = Picker::halfblocks();
    let control = LoadControl::new();
    let key = path_key(&path);
    let size = prefetch_original_size(4, 2);
    control.update_original_interest(31, 4, 2, None, [key.clone()]);
    let (done_tx, done_rx) = std::sync::mpsc::channel::<LoadResult>();
    process_load_request_with_control_to_sender(
        &picker,
        &control,
        LoadRequest {
            key,
            path: path.clone(),
            size: size.clone(),
            generation: 31,
        },
        &done_tx,
    );

    let result = done_rx.try_recv().unwrap();
    assert_skipped(result, 0, size, 31);
    assert!(done_rx.try_recv().is_err());
}

#[test]
fn prefetch_static_original_decodes_to_rgba() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("static.png");
    write_png(&path);

    let picker = Picker::halfblocks();
    let control = LoadControl::new();
    let key = path_key(&path);
    let size = prefetch_original_size(4, 2);
    control.update_original_interest(41, 4, 2, None, [key.clone()]);
    let (done_tx, done_rx) = std::sync::mpsc::channel::<LoadResult>();
    process_load_request_with_control_to_sender(
        &picker,
        &control,
        LoadRequest {
            key,
            path: path.clone(),
            size: size.clone(),
            generation: 41,
        },
        &done_tx,
    );

    let result = done_rx.try_recv().unwrap();
    assert_eq!(result.size, size);
    assert!(matches!(
        result.content,
        LoadContent::Original(FullscreenContent::Static(_))
    ));
}

#[test]
fn streaming_animation_first_frame_displays_before_finish() {
    let (mut app, done_tx) = make_app_with_load_done(1);
    let key = app_key(&app, 0);
    let size = selected_original_size(80, 40);
    app.state = AppState::Fullscreen;
    app.fullscreen_image_w = 80;
    app.fullscreen_image_h = 40;
    app.fullscreen_pending = true;
    app.requested.insert((key.clone(), size.clone()));

    done_tx
        .send(LoadResult {
            key: key.clone(),
            size: size.clone(),
            generation: app.directory_generation,
            content: LoadContent::AnimationStarted { dims: (1, 1) },
            dims: None,
        })
        .unwrap();
    done_tx
        .send(LoadResult {
            key: key.clone(),
            size: size.clone(),
            generation: app.directory_generation,
            content: LoadContent::AnimationFrame {
                index: 0,
                frame: make_animation_frame(100),
            },
            dims: None,
        })
        .unwrap();

    app.collect_loads();

    assert!(app.current_fullscreen_protocol().is_some());
    assert!(!app.fullscreen_pending);
    assert!(app.requested.contains(&(key, size)));
}

#[test]
fn complete_streaming_animation_writes_cache_and_clears_request() {
    let (mut app, done_tx) = make_app_with_load_done(1);
    let key = app_key(&app, 0);
    let size = selected_original_size(80, 40);
    app.state = AppState::Fullscreen;
    app.fullscreen_image_w = 80;
    app.fullscreen_image_h = 40;
    app.requested.insert((key.clone(), size.clone()));

    for content in [
        LoadContent::AnimationStarted { dims: (1, 1) },
        LoadContent::AnimationFrame {
            index: 0,
            frame: make_animation_frame(100),
        },
        LoadContent::AnimationFrame {
            index: 1,
            frame: make_animation_frame(120),
        },
        LoadContent::AnimationFinished { complete: true },
    ] {
        done_tx
            .send(LoadResult {
                key: key.clone(),
                size: size.clone(),
                generation: app.directory_generation,
                content,
                dims: None,
            })
            .unwrap();
    }

    app.collect_loads();

    let cache_key = app.current_animation_cache_key().unwrap();
    assert!(app.animation_cache.contains(&cache_key));
    assert!(!app.requested.contains(&(key, size)));
}

#[test]
fn incomplete_streaming_animation_does_not_write_cache() {
    let (mut app, done_tx) = make_app_with_load_done(1);
    let key = app_key(&app, 0);
    let size = selected_original_size(80, 40);
    app.state = AppState::Fullscreen;
    app.fullscreen_image_w = 80;
    app.fullscreen_image_h = 40;
    app.requested.insert((key.clone(), size.clone()));

    for content in [
        LoadContent::AnimationStarted { dims: (1, 1) },
        LoadContent::AnimationFrame {
            index: 0,
            frame: make_animation_frame(100),
        },
        LoadContent::AnimationFrame {
            index: 1,
            frame: make_animation_frame(120),
        },
        LoadContent::AnimationFinished { complete: false },
    ] {
        done_tx
            .send(LoadResult {
                key: key.clone(),
                size: size.clone(),
                generation: app.directory_generation,
                content,
                dims: None,
            })
            .unwrap();
    }

    app.collect_loads();

    assert!(app.animation_cache.is_empty());
    assert!(!app.requested.contains(&(key, size)));
}

#[test]
fn stale_animation_frame_after_viewport_change_is_discarded() {
    let (mut app, done_tx) = make_app_with_load_done(1);
    let key = app_key(&app, 0);
    app.state = AppState::Fullscreen;
    app.fullscreen_image_w = 81;
    app.fullscreen_image_h = 40;

    done_tx
        .send(LoadResult {
            key,
            size: selected_original_size(80, 40),
            generation: app.directory_generation,
            content: LoadContent::AnimationFrame {
                index: 0,
                frame: make_animation_frame(100),
            },
            dims: None,
        })
        .unwrap();

    app.collect_loads();

    assert!(app.current_fullscreen_protocol().is_none());
}

#[test]
fn process_original_request_decodes_static_jpeg_to_rgba() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("static.jpg");
    image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(3, 2, image::Rgb([10, 20, 30])))
        .save(&path)
        .unwrap();

    let picker = Picker::halfblocks();
    let control = LoadControl::new();
    let size = selected_original_size(4, 2);
    control.update_original_interest(13, 4, 2, Some(path_key(&path)), Vec::<ImageCacheKey>::new());
    let (done_tx, _done_rx) = std::sync::mpsc::channel::<LoadResult>();
    let result = process_original_request(
        &picker,
        &control,
        &done_tx,
        OriginalRequestParts {
            path: &path,
            key: path_key(&path),
            generation: 13,
            w: 4,
            h: 2,
            kind: OriginalLoadKind::Selected,
        },
    )
    .unwrap();

    assert_eq!(result.size, size);
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
    let result = process_thumbnail_request(&picker, &path, path_key(&path), 17, 8, 4).unwrap();

    assert_eq!(result.size, LoadSize::Thumbnail { w: 8, h: 4 });
    assert_eq!(result.generation, 17);
    assert_eq!(result.dims, Some((4, 3)));
    match result.content {
        LoadContent::Thumbnail(protocol) => assert!(protocol.size().width <= 8),
        LoadContent::Original(_) => panic!("expected thumbnail protocol"),
        LoadContent::AnimationStarted { .. }
        | LoadContent::AnimationFrame { .. }
        | LoadContent::AnimationFinished { .. } => panic!("expected thumbnail protocol"),
        LoadContent::Skipped => panic!("expected thumbnail protocol"),
    }
}

#[test]
fn load_control_skips_stale_generation_without_decoding() {
    let picker = Picker::halfblocks();
    let control = LoadControl::new();
    control.set_generation(2);
    let thumb_size = LoadSize::Thumbnail { w: 8, h: 4 };

    let thumb = process_load_request_with_control(
        &picker,
        &control,
        missing_load_request(0, thumb_size.clone(), 1),
    )
    .unwrap();
    let original = process_load_request_with_control(
        &picker,
        &control,
        missing_load_request(0, selected_original_size(8, 4), 1),
    )
    .unwrap();

    assert_skipped(thumb, 0, thumb_size, 1);
    assert_skipped(original, 0, selected_original_size(8, 4), 1);
}

#[test]
fn load_control_skips_thumbnail_outside_interest() {
    let picker = Picker::halfblocks();
    let control = LoadControl::new();
    let size = LoadSize::Thumbnail { w: 8, h: 4 };
    control.update_thumbnail_interest(0, 8, 4, [test_key("img001.png")]);

    let result = process_load_request_with_control(
        &picker,
        &control,
        missing_load_request(0, size.clone(), 0),
    )
    .unwrap();

    assert_skipped(result, 0, size, 0);
}

#[test]
fn load_control_skips_stale_thumbnail_size() {
    let picker = Picker::halfblocks();
    let control = LoadControl::new();
    let size = LoadSize::Thumbnail { w: 7, h: 4 };
    control.update_thumbnail_interest(0, 8, 4, [test_key("missing.png")]);

    let result = process_load_request_with_control(
        &picker,
        &control,
        missing_load_request(0, size.clone(), 0),
    )
    .unwrap();

    assert_skipped(result, 0, size, 0);
}

#[test]
fn load_control_skips_stale_original_viewport() {
    let picker = Picker::halfblocks();
    let control = LoadControl::new();
    let size = selected_original_size(7, 4);
    control.update_original_interest(
        0,
        8,
        4,
        Some(test_key("missing.png")),
        Vec::<ImageCacheKey>::new(),
    );

    let result = process_load_request_with_control(
        &picker,
        &control,
        missing_load_request(0, size.clone(), 0),
    )
    .unwrap();

    assert_skipped(result, 0, size, 0);
}

#[test]
fn load_control_skips_original_outside_current_neighbors() {
    let picker = Picker::halfblocks();
    let control = LoadControl::new();
    control.update_original_interest(
        0,
        8,
        4,
        Some(test_key("img004.png")),
        Vec::<ImageCacheKey>::new(),
    );
    let size = selected_original_size(8, 4);

    let result = process_load_request_with_control(
        &picker,
        &control,
        missing_load_request(2, size.clone(), 0),
    )
    .unwrap();

    assert_skipped(result, 2, size, 0);
}

#[test]
fn load_control_allows_current_original_request() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("current.png");
    write_png(&path);
    let picker = Picker::halfblocks();
    let control = LoadControl::new();
    control.update_original_interest(0, 8, 4, Some(path_key(&path)), Vec::<ImageCacheKey>::new());

    let result = process_load_request_with_control(
        &picker,
        &control,
        LoadRequest {
            key: path_key(&path),
            path: path.clone(),
            size: selected_original_size(8, 4),
            generation: 0,
        },
    )
    .unwrap();

    assert_eq!(result.size, selected_original_size(8, 4));
    assert_eq!(result.generation, 0);
    assert!(matches!(result.content, LoadContent::Original(_)));
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
        image_key: app_key(&app, app.selected),
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
    let key = app_key(&app, 0);

    app.insert_fullscreen_original(key.clone(), Arc::new(image::RgbaImage::new(10, 20)));

    assert_eq!(app.fullscreen_original_cache_bytes, 10 * 20 * 4);
    assert_eq!(
        app.cached_fullscreen_original(&key)
            .map(|image| image.len()),
        Some(10 * 20 * 4)
    );
}

#[test]
fn fullscreen_original_cache_evicts_to_budget_and_keeps_selected() {
    let mut app = make_app(3);
    app.selected = 0;
    for idx in 0..3 {
        app.insert_fullscreen_original(
            app_key(&app, idx),
            Arc::new(image::RgbaImage::new(4096, 4096)),
        );
    }

    assert!(app.fullscreen_original_cache_bytes <= FULLSCREEN_ORIGINAL_CACHE_BYTES);
    assert!(app.fullscreen_original_cache.contains(&app_key(&app, 0)));
    assert!(app.fullscreen_original_cache.contains(&app_key(&app, 2)));
    assert!(!app.fullscreen_original_cache.contains(&app_key(&app, 1)));
}

#[test]
fn fullscreen_original_cache_evicts_neighbor_before_selected() {
    let mut app = make_app(2);
    app.selected = 0;
    for idx in 0..2 {
        app.insert_fullscreen_original(
            app_key(&app, idx),
            Arc::new(image::RgbaImage::new(5000, 5000)),
        );
    }

    assert!(app.fullscreen_original_cache_bytes <= FULLSCREEN_ORIGINAL_CACHE_BYTES);
    assert!(app.fullscreen_original_cache.contains(&app_key(&app, 0)));
    assert!(!app.fullscreen_original_cache.contains(&app_key(&app, 1)));
}

#[test]
fn animation_cache_hit_satisfies_fullscreen_without_request() {
    let (mut app, rx) = make_app_with_load_rx(1);
    let key = app_key(&app, 0);
    app.state = AppState::Fullscreen;
    app.fullscreen_image_w = 80;
    app.fullscreen_image_h = 40;
    let cache_key = app.current_animation_cache_key().unwrap();
    app.insert_animation_cache(
        cache_key,
        animation_content(vec![make_animation_frame(100), make_animation_frame(120)]),
        Some((1, 1)),
    );

    app.prepare_fullscreen_selection();

    assert!(app.current_fullscreen_protocol().is_some());
    assert!(!app.fullscreen_pending);
    assert!(rx.try_recv().is_err());
    assert_eq!(app.fullscreen_content_key, Some(key));
}

#[test]
fn oversized_animation_is_not_cached() {
    let mut app = make_app(1);
    app.state = AppState::Fullscreen;
    app.fullscreen_image_w = 80;
    app.fullscreen_image_h = 40;
    let cache_key = app.current_animation_cache_key().unwrap();
    let mut content = animation_content(vec![make_animation_frame(100), make_animation_frame(120)]);
    content.estimated_bytes = ANIMATION_CACHE_BYTES + 1;

    app.insert_animation_cache(cache_key, content, Some((1, 1)));

    assert!(app.animation_cache.is_empty());
    assert_eq!(app.animation_cache_bytes, 0);
}

#[test]
fn animation_cache_evicts_lru_entries_to_budget() {
    let mut app = make_app(3);
    app.state = AppState::Fullscreen;
    app.selected = 2;
    app.fullscreen_image_w = 80;
    app.fullscreen_image_h = 40;

    for idx in 0..3 {
        let key = animation_cache_key(app_key(&app, idx), 80, 40, app.picker.font_size());
        let mut content =
            animation_content(vec![make_animation_frame(100), make_animation_frame(120)]);
        content.estimated_bytes = ANIMATION_CACHE_BYTES / 2;
        app.insert_animation_cache(key, content, Some((1, 1)));
    }

    assert!(app.animation_cache_bytes <= ANIMATION_CACHE_BYTES);
    assert_eq!(app.animation_cache.len(), 2);
    assert!(!app.animation_cache.contains(&animation_cache_key(
        app_key(&app, 0),
        80,
        40,
        app.picker.font_size()
    )));
}

#[test]
fn fullscreen_viewport_change_requests_new_animation_size() {
    let (mut app, rx) = make_app_with_load_rx(1);
    app.state = AppState::Fullscreen;
    app.fullscreen_image_w = 80;
    app.fullscreen_image_h = 40;
    app.set_fullscreen_content(
        FullscreenContent::Animation(animation_content(vec![
            make_animation_frame(100),
            make_animation_frame(120),
        ])),
        Some((1, 1)),
        Instant::now(),
    );

    app.set_fullscreen_viewport(81, 40);

    let request = rx.try_recv().unwrap();
    assert_eq!(request.size, selected_original_size(81, 40));
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
fn thumbnail_lru_evicts_least_recently_used_entry() {
    let mut app = make_app(MAX_CACHE_SIZE + 1);
    for idx in 0..MAX_CACHE_SIZE {
        app.insert_cache(app_key(&app, idx), make_protocol());
    }

    app.insert_cache(app_key(&app, MAX_CACHE_SIZE), make_protocol());

    assert_eq!(app.protocol_cache.len(), MAX_CACHE_SIZE);
    assert!(!app.protocol_cache.contains(&app_key(&app, 0)));
    assert!(app.protocol_cache.contains(&app_key(&app, MAX_CACHE_SIZE)));
}

#[test]
fn thumbnail_lru_touch_keeps_visible_cache_entry() {
    let mut app = make_app(MAX_CACHE_SIZE + 1);
    for idx in 0..MAX_CACHE_SIZE {
        app.insert_cache(app_key(&app, idx), make_protocol());
    }
    app.grid_cols = 1;
    app.visible_rows = 1;
    app.scroll_row = 0;
    app.cache_width = 80;
    app.cache_height = 24;

    crate::ui::browser::populate_protocol_cache(&mut app, 6, 6, Size::new(80, 24));
    app.insert_cache(app_key(&app, MAX_CACHE_SIZE), make_protocol());

    assert_eq!(app.protocol_cache.len(), MAX_CACHE_SIZE);
    assert!(app.protocol_cache.contains(&app_key(&app, 0)));
    assert!(!app.protocol_cache.contains(&app_key(&app, 2)));
}

#[test]
fn skipped_load_results_clear_requested_and_allow_rerequest() {
    let images = vec![ImageEntry {
        path: PathBuf::from("img000.png"),
        filename: "img000.png".to_string(),
        file_size: 0,
        modified_at: None,
    }];
    let (load_tx, load_rx) = std::sync::mpsc::channel::<LoadRequest>();
    let (done_tx, done_rx) = std::sync::mpsc::channel::<LoadResult>();
    let mut app = App::new(
        AppStart {
            images,
            image_dir: PathBuf::from("."),
            state: AppState::Browser,
            selected: 0,
        },
        load_tx,
        done_rx,
        Lang::Zh,
        Picker::halfblocks(),
    );
    let size = LoadSize::Thumbnail { w: 1, h: 1 };
    let key = app_key(&app, 0);
    app.requested.insert((key.clone(), size.clone()));

    done_tx
        .send(LoadResult {
            key: key.clone(),
            size: size.clone(),
            generation: app.directory_generation,
            content: LoadContent::Skipped,
            dims: None,
        })
        .unwrap();
    app.collect_loads();
    app.request_load(0, size.clone());

    assert!(!app.requested.is_empty());
    assert!(!app.protocol_cache.contains(&key));
    assert_eq!(load_rx.try_recv().unwrap().size, size);
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
    let cached_key = app_key(&app, 0);
    app.protocol_cache.put(cached_key.clone(), make_protocol());
    app.requested
        .insert((cached_key, LoadSize::Thumbnail { w: 1, h: 1 }));
    let generation = app.directory_generation;

    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert_eq!(app.image_dir, child);
    assert_eq!(app.images.len(), 1);
    assert_eq!(app.images[0].filename, "new.png");
    assert_eq!(app.selected, 0);
    assert_eq!(app.scroll_row, 0);
    assert_eq!(app.context_selected, 0);
    assert_eq!(app.context_scroll, 0);
    assert!(!app.protocol_cache.is_empty());
    assert!(!app.requested.is_empty());
    assert_eq!(app.directory_generation, generation);
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
fn stale_generation_is_discarded_but_path_mismatch_uses_key() {
    let (mut app, done_tx) = make_app_with_load_done(1);
    let generation = app.directory_generation;
    let key = app_key(&app, 0);
    app.thumb_w = 1;
    app.thumb_h = 1;

    done_tx
        .send(LoadResult {
            key: key.clone(),
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
            key: key.clone(),
            size: LoadSize::Thumbnail { w: 1, h: 1 },
            generation,
            content: LoadContent::Thumbnail(make_protocol()),
            dims: Some((1, 1)),
        })
        .unwrap();
    app.collect_loads();
    assert!(app.protocol_cache.contains(&key));
}

#[test]
fn favorite_toggle_works_in_browser_and_fullscreen_but_not_text_modes() {
    let dir = tempdir().unwrap();
    let mut app = make_app_with_names(&["sample.png"]);
    isolate_favorites(&mut app, dir.path());

    app.handle_key(KeyCode::Char('f'), KeyModifiers::NONE);
    assert!(app.favorites.is_favorite(&app.images[0].path));

    app.handle_key(KeyCode::Char('/'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('F'), KeyModifiers::NONE);
    assert_eq!(app.search.as_ref().unwrap().query, "F");
    assert_eq!(app.gallery_mode, GalleryMode::Directory);
    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);

    app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('f'), KeyModifiers::NONE);
    assert!(app.rename.as_ref().unwrap().input.ends_with('f'));
    assert!(app.favorites.is_favorite(&app.images[0].path));
    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);

    app.enter_fullscreen();
    app.handle_key(KeyCode::Char('f'), KeyModifiers::NONE);
    assert!(!app.favorites.is_favorite(&app.images[0].path));
}

#[test]
fn directory_gallery_pins_global_favorites_by_newest_first() {
    let dir = tempdir().unwrap();
    for name in ["a.png", "b.png", "c.png", "d.png"] {
        write_png(&dir.path().join(name));
    }
    let images = vec![
        ImageEntry {
            path: dir.path().join("a.png"),
            filename: "a.png".to_string(),
            file_size: 0,
            modified_at: None,
        },
        ImageEntry {
            path: dir.path().join("b.png"),
            filename: "b.png".to_string(),
            file_size: 0,
            modified_at: None,
        },
        ImageEntry {
            path: dir.path().join("c.png"),
            filename: "c.png".to_string(),
            file_size: 0,
            modified_at: None,
        },
        ImageEntry {
            path: dir.path().join("d.png"),
            filename: "d.png".to_string(),
            file_size: 0,
            modified_at: None,
        },
    ];
    let mut app = make_app_with_entries(images);
    app.set_grid_layout(2, 3);
    isolate_favorites(&mut app, dir.path());
    app.add_favorite_for_tests(&dir.path().join("d.png"), 10);
    app.add_favorite_for_tests(&dir.path().join("a.png"), 20);
    app.add_favorite_for_tests(&dir.path().join("c.png"), 30);

    assert_eq!(app.favorite_row_len(), 2);
    assert_eq!(image_names(&app), vec!["c.png", "a.png", "b.png", "d.png"]);

    app.selected = 0;
    app.navigate_down();
    assert_eq!(app.images[app.selected].filename, "b.png");
    app.navigate_up();
    assert_eq!(app.images[app.selected].filename, "c.png");

    app.selected = 3;
    app.scroll_row = 2;
    app.navigate_home();
    assert_eq!(app.images[app.selected].filename, "b.png");
    assert_eq!(app.scroll_row, 0);
}

#[test]
fn up_from_first_directory_row_jumps_to_nearest_favorite() {
    let dir = tempdir().unwrap();
    for name in ["a.png", "b.png", "c.png", "d.png"] {
        write_png(&dir.path().join(name));
    }
    let images = ["a.png", "b.png", "c.png", "d.png"]
        .into_iter()
        .map(|name| ImageEntry {
            path: dir.path().join(name),
            filename: name.to_string(),
            file_size: 0,
            modified_at: None,
        })
        .collect();
    let mut app = make_app_with_entries(images);
    app.set_grid_layout(4, 3);
    isolate_favorites(&mut app, dir.path());
    app.add_favorite_for_tests(&dir.path().join("a.png"), 10);
    app.selected = app
        .images
        .iter()
        .position(|entry| entry.filename == "d.png")
        .unwrap();

    app.navigate_up();

    assert_eq!(app.images[app.selected].filename, "a.png");
}

#[test]
fn directory_gallery_keeps_global_favorite_row_after_switching_folder() {
    let dir = tempdir().unwrap();
    let current = dir.path().join("current");
    let child = current.join("child");
    let other = dir.path().join("other");
    fs::create_dir_all(&child).unwrap();
    fs::create_dir_all(&other).unwrap();
    write_png(&current.join("a.png"));
    write_png(&child.join("new.png"));
    write_png(&other.join("favorite.png"));
    let (mut app, _rx) = make_app_for_dir(&current, 0, AppState::Browser);
    app.set_grid_layout(3, 3);
    isolate_favorites(&mut app, dir.path());
    app.add_favorite_for_tests(&other.join("favorite.png"), 10);

    app.enter_directory(child);

    assert_eq!(app.favorite_row_len(), 1);
    assert_eq!(image_names(&app), vec!["favorite.png", "new.png"]);
}

#[test]
fn favorites_view_shows_all_existing_favorites_newest_first_and_toggles_back() {
    let dir = tempdir().unwrap();
    let other = tempdir().unwrap();
    write_png(&dir.path().join("a.png"));
    write_png(&dir.path().join("b.png"));
    write_png(&other.path().join("c.png"));
    let (mut app, _rx) = make_app_for_dir(dir.path(), 1, AppState::Browser);
    isolate_favorites(&mut app, dir.path());
    app.add_favorite_for_tests(&dir.path().join("a.png"), 20);
    app.add_favorite_for_tests(&other.path().join("c.png"), 30);
    app.selected = app
        .images
        .iter()
        .position(|entry| entry.filename == "b.png")
        .unwrap();

    app.handle_key(KeyCode::Char('F'), KeyModifiers::NONE);

    assert_eq!(app.gallery_mode, GalleryMode::Favorites);
    assert_eq!(image_names(&app), vec!["c.png", "a.png"]);
    assert_eq!(app.selected, 0);

    app.handle_key(KeyCode::Char('F'), KeyModifiers::NONE);

    assert_eq!(app.gallery_mode, GalleryMode::Directory);
    assert_eq!(image_names(&app), vec!["c.png", "a.png", "b.png"]);
    assert_eq!(app.images[app.selected].filename, "b.png");
}

#[test]
fn unfavorite_in_favorites_view_removes_current_and_exits_empty_fullscreen() {
    let dir = tempdir().unwrap();
    write_png(&dir.path().join("a.png"));
    write_png(&dir.path().join("b.png"));
    let (mut app, _rx) = make_app_for_dir(dir.path(), 0, AppState::Browser);
    isolate_favorites(&mut app, dir.path());
    app.add_favorite_for_tests(&dir.path().join("a.png"), 10);
    app.add_favorite_for_tests(&dir.path().join("b.png"), 20);
    app.handle_key(KeyCode::Char('F'), KeyModifiers::NONE);
    app.selected = 0;

    app.handle_key(KeyCode::Char('f'), KeyModifiers::NONE);

    assert_eq!(image_names(&app), vec!["a.png"]);
    assert_eq!(app.selected, 0);
    assert_eq!(app.images[app.selected].filename, "a.png");

    app.enter_fullscreen();
    app.handle_key(KeyCode::Char('f'), KeyModifiers::NONE);

    assert_eq!(app.gallery_mode, GalleryMode::Favorites);
    assert_eq!(app.state, AppState::Browser);
    assert!(app.images.is_empty());
}

#[test]
fn favorite_changes_preserve_image_caches_and_generation() {
    let dir = tempdir().unwrap();
    let mut app = make_app(2);
    isolate_favorites(&mut app, dir.path());
    let key = app_key(&app, 0);
    app.protocol_cache.put(key.clone(), make_protocol());
    app.requested
        .insert((key.clone(), LoadSize::Thumbnail { w: 1, h: 1 }));
    app.insert_fullscreen_original(key.clone(), Arc::new(image::RgbaImage::new(1, 1)));
    let generation = app.directory_generation;

    app.handle_key(KeyCode::Char('f'), KeyModifiers::NONE);

    assert_eq!(app.directory_generation, generation);
    assert!(app.protocol_cache.contains(&key));
    assert!(!app.requested.is_empty());
    assert!(app.fullscreen_original_cache.contains(&key));
}

#[test]
fn favorite_toggle_reuses_cached_thumbnail_after_reorder() {
    let dir = tempdir().unwrap();
    write_png(&dir.path().join("sample.png"));
    let (mut app, rx) = make_app_for_dir(dir.path(), 0, AppState::Browser);
    isolate_favorites(&mut app, dir.path());
    app.set_grid_layout(3, 3);
    app.cache_width = 80;
    app.cache_height = 24;
    let key = app_key(&app, 0);
    app.protocol_cache.put(key.clone(), make_protocol());

    app.handle_key(KeyCode::Char('f'), KeyModifiers::NONE);
    crate::ui::browser::populate_protocol_cache(&mut app, 6, 6, Size::new(80, 24));

    assert_eq!(app.images[app.selected].filename, "sample.png");
    assert_eq!(app_key(&app, app.selected), key);
    assert!(app.protocol_cache.contains(&key));
    assert!(rx.try_recv().is_err());
}

#[test]
fn favorites_view_toggle_keeps_current_fullscreen_original_cache() {
    let dir = tempdir().unwrap();
    write_png(&dir.path().join("sample.png"));
    let (mut app, _rx) = make_app_for_dir(dir.path(), 0, AppState::Browser);
    isolate_favorites(&mut app, dir.path());
    app.add_favorite_for_tests(&dir.path().join("sample.png"), 10);
    let key = app_key(&app, app.selected);
    app.insert_fullscreen_original(key.clone(), Arc::new(image::RgbaImage::new(2, 2)));

    app.enter_fullscreen();
    let render_generation = app.render_generation;
    app.handle_key(KeyCode::Char('F'), KeyModifiers::NONE);

    assert_eq!(app.gallery_mode, GalleryMode::Favorites);
    assert_eq!(app.state, AppState::Fullscreen);
    assert_eq!(app.fullscreen_content_key, Some(key.clone()));
    assert!(app.fullscreen_original_cache.contains(&key));
    assert_eq!(app.render_generation, render_generation);
}

#[test]
fn delete_confirmation_can_cancel_in_browser() {
    let dir = tempdir().unwrap();
    write_png(&dir.path().join("a.png"));
    write_png(&dir.path().join("b.png"));
    let (mut app, _rx) = make_app_for_dir(dir.path(), 0, AppState::Browser);

    app.handle_key(KeyCode::Char('d'), KeyModifiers::NONE);
    assert!(app.delete.is_some());
    app.handle_key(KeyCode::Char('n'), KeyModifiers::NONE);

    assert!(app.delete.is_none());
    assert!(dir.path().join("a.png").exists());
    assert_eq!(image_names(&app), vec!["a.png", "b.png"]);

    app.handle_key(KeyCode::Char('d'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);

    assert!(app.delete.is_none());
    assert!(dir.path().join("a.png").exists());
    assert_eq!(image_names(&app), vec!["a.png", "b.png"]);
}

#[test]
fn delete_confirm_removes_file_and_selects_adjacent_image() {
    let dir = tempdir().unwrap();
    for name in ["a.png", "b.png", "c.png"] {
        write_png(&dir.path().join(name));
    }
    let (mut app, _rx) = make_app_for_dir(dir.path(), 1, AppState::Browser);

    app.handle_key(KeyCode::Char('d'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);

    assert!(!dir.path().join("b.png").exists());
    assert_eq!(image_names(&app), vec!["a.png", "c.png"]);
    assert_eq!(app.images[app.selected].filename, "c.png");

    app.handle_key(KeyCode::Char('d'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert!(!dir.path().join("c.png").exists());
    assert_eq!(image_names(&app), vec!["a.png"]);
    assert_eq!(app.images[app.selected].filename, "a.png");
}

#[test]
fn delete_shortcut_starts_only_from_gallery_or_fullscreen() {
    let mut app = make_app_with_names(&["sample.png"]);
    app.browser_focus = BrowserFocus::Context;
    app.handle_key(KeyCode::Char('d'), KeyModifiers::NONE);
    assert!(app.delete.is_none());

    app.browser_focus = BrowserFocus::Gallery;
    app.handle_key(KeyCode::Char('/'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('d'), KeyModifiers::NONE);
    assert!(app.delete.is_none());
    assert_eq!(app.search.as_ref().unwrap().query, "d");
    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);

    app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('d'), KeyModifiers::NONE);
    assert!(app.delete.is_none());
    assert!(app.rename.as_ref().unwrap().input.ends_with('d'));
    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);

    app.state = AppState::Fullscreen;
    app.handle_key(KeyCode::Char('d'), KeyModifiers::NONE);
    assert!(app.delete.is_some());
    assert_eq!(app.delete.as_ref().unwrap().origin, AppState::Fullscreen);
}

#[test]
fn fullscreen_delete_keeps_fullscreen_until_last_image() {
    let dir = tempdir().unwrap();
    write_png(&dir.path().join("a.png"));
    write_png(&dir.path().join("b.png"));
    let (mut app, rx) = make_app_for_dir(dir.path(), 0, AppState::Fullscreen);
    while rx.try_recv().is_ok() {}

    app.handle_key(KeyCode::Char('d'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);

    assert!(!dir.path().join("a.png").exists());
    assert_eq!(app.state, AppState::Fullscreen);
    assert_eq!(image_names(&app), vec!["b.png"]);
    assert_eq!(app.selected, 0);

    app.handle_key(KeyCode::Char('d'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert!(!dir.path().join("b.png").exists());
    assert_eq!(app.state, AppState::Browser);
    assert!(app.images.is_empty());
}

#[test]
fn delete_favorite_removes_favorite_record_and_empty_favorites_status() {
    let dir = tempdir().unwrap();
    write_png(&dir.path().join("a.png"));
    let (mut app, _rx) = make_app_for_dir(dir.path(), 0, AppState::Browser);
    isolate_favorites(&mut app, dir.path());
    app.add_favorite_for_tests(&dir.path().join("a.png"), 10);
    app.handle_key(KeyCode::Char('F'), KeyModifiers::NONE);

    app.handle_key(KeyCode::Char('d'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);

    assert!(!dir.path().join("a.png").exists());
    assert!(app.favorites.entries().is_empty());
    assert_eq!(app.gallery_mode, GalleryMode::Favorites);
    assert!(app.images.is_empty());
    assert_eq!(app.browser_status_message().unwrap(), "No favorites");
}

#[test]
fn delete_failure_keeps_gallery_and_selection() {
    let dir = tempdir().unwrap();
    write_png(&dir.path().join("a.png"));
    write_png(&dir.path().join("b.png"));
    let (mut app, _rx) = make_app_for_dir(dir.path(), 0, AppState::Browser);

    app.handle_key(KeyCode::Char('d'), KeyModifiers::NONE);
    fs::remove_file(dir.path().join("a.png")).unwrap();
    app.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);

    assert!(app.delete.is_none());
    assert_eq!(image_names(&app), vec!["a.png", "b.png"]);
    assert_eq!(app.selected, 0);
    assert!(app
        .browser_status_message()
        .unwrap()
        .starts_with("Delete failed:"));
}

#[test]
fn delete_removes_only_deleted_image_caches_and_requests() {
    let dir = tempdir().unwrap();
    write_png(&dir.path().join("a.png"));
    write_png(&dir.path().join("b.png"));
    let (mut app, _rx) = make_app_for_dir(dir.path(), 0, AppState::Browser);
    let deleted_key = app_key(&app, 0);
    let kept_key = app_key(&app, 1);
    let deleted_render_key = RenderKey {
        image_key: deleted_key.clone(),
        viewport_w: 10,
        viewport_h: 10,
        font_w: 1,
        font_h: 1,
        zoom_percent: 100,
        pan_x: 0,
        pan_y: 0,
        quality: RenderQuality::Final,
    };
    let kept_render_key = RenderKey {
        image_key: kept_key.clone(),
        viewport_w: 10,
        viewport_h: 10,
        font_w: 1,
        font_h: 1,
        zoom_percent: 100,
        pan_x: 0,
        pan_y: 0,
        quality: RenderQuality::Final,
    };
    let deleted_animation_key =
        animation_cache_key(deleted_key.clone(), 80, 40, app.picker.font_size());
    let kept_animation_key = animation_cache_key(kept_key.clone(), 80, 40, app.picker.font_size());

    app.protocol_cache.put(deleted_key.clone(), make_protocol());
    app.protocol_cache.put(kept_key.clone(), make_protocol());
    app.requested
        .insert((deleted_key.clone(), LoadSize::Thumbnail { w: 1, h: 1 }));
    app.requested
        .insert((kept_key.clone(), LoadSize::Thumbnail { w: 1, h: 1 }));
    app.insert_fullscreen_original(deleted_key.clone(), Arc::new(image::RgbaImage::new(1, 1)));
    app.insert_fullscreen_original(kept_key.clone(), Arc::new(image::RgbaImage::new(1, 1)));
    app.insert_animation_cache(
        deleted_animation_key.clone(),
        animation_content(vec![make_animation_frame(100), make_animation_frame(120)]),
        Some((1, 1)),
    );
    app.insert_animation_cache(
        kept_animation_key.clone(),
        animation_content(vec![make_animation_frame(100), make_animation_frame(120)]),
        Some((1, 1)),
    );
    app.fullscreen_render_cache
        .put(deleted_render_key.clone(), make_protocol());
    app.fullscreen_render_cache
        .put(kept_render_key.clone(), make_protocol());

    app.handle_key(KeyCode::Char('d'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);

    assert!(!app.protocol_cache.contains(&deleted_key));
    assert!(app.protocol_cache.contains(&kept_key));
    assert!(!app.requested.iter().any(|(key, _)| key == &deleted_key));
    assert!(app.requested.iter().any(|(key, _)| key == &kept_key));
    assert!(!app.fullscreen_original_cache.contains(&deleted_key));
    assert!(app.fullscreen_original_cache.contains(&kept_key));
    assert!(!app.animation_cache.contains(&deleted_animation_key));
    assert!(app.animation_cache.contains(&kept_animation_key));
    assert!(!app.fullscreen_render_cache.contains(&deleted_render_key));
    assert!(app.fullscreen_render_cache.contains(&kept_render_key));
}

#[test]
fn rename_updates_favorite_path_and_preserves_added_time() {
    let dir = tempdir().unwrap();
    write_png(&dir.path().join("old.png"));
    let (mut app, _rx) = make_app_for_dir(dir.path(), 0, AppState::Browser);
    isolate_favorites(&mut app, dir.path());
    app.add_favorite_for_tests(&dir.path().join("old.png"), 77);

    app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
    set_rename_input(&mut app, "new");
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    let new_path = FavoriteStore::normalize_path(&dir.path().join("new.png"));
    assert_eq!(app.favorites.entries().len(), 1);
    assert_eq!(app.favorites.entries()[0].path, new_path);
    assert_eq!(app.favorites.entries()[0].added_at_ms, 77);
    assert_eq!(app.images[app.selected].filename, "new.png");
}

// ---- Rename tests ----

#[test]
fn rename_shortcut_starts_only_from_gallery_or_fullscreen() {
    let mut app = make_app(1);
    app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
    assert!(app.rename.is_some());
    assert_eq!(app.rename.as_ref().unwrap().input, "img000");

    let mut app = make_app(1);
    app.browser_focus = BrowserFocus::Context;
    app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
    assert!(app.rename.is_none());

    let mut app = make_app(1);
    app.state = AppState::Fullscreen;
    app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
    assert!(app.rename.is_some());
    assert_eq!(app.rename.as_ref().unwrap().origin, AppState::Fullscreen);
}

#[test]
fn search_mode_keeps_r_as_query_text() {
    let mut app = make_app_with_names(&["sample.png"]);

    app.handle_key(KeyCode::Char('/'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);

    assert!(app.rename.is_none());
    assert_eq!(app.search.as_ref().unwrap().query, "r");
}

#[test]
fn rename_input_backspace_and_escape_work() {
    let mut app = make_app(1);

    app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('x'), KeyModifiers::NONE);
    assert_eq!(app.rename.as_ref().unwrap().input, "img000x");

    app.handle_key(KeyCode::Backspace, KeyModifiers::NONE);
    assert_eq!(app.rename.as_ref().unwrap().input, "img000");

    app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
    assert!(app.rename.is_none());
    assert!(app.browser_status_message().is_some());
}

#[test]
fn rename_rejects_empty_separator_and_unchanged_names() {
    let dir = tempdir().unwrap();
    write_png(&dir.path().join("old.png"));
    let (mut app, _rx) = make_app_for_dir(dir.path(), 0, AppState::Browser);

    app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
    set_rename_input(&mut app, "");
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    assert!(app.rename.is_some());
    assert!(app.rename.as_ref().unwrap().message.is_some());
    assert!(dir.path().join("old.png").exists());

    set_rename_input(&mut app, "bad/name");
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    assert!(app.rename.is_some());
    assert!(app.rename.as_ref().unwrap().message.is_some());
    assert!(!dir.path().join("bad.png").exists());

    set_rename_input(&mut app, "old");
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    assert!(app.rename.is_none());
    assert!(dir.path().join("old.png").exists());
}

#[test]
fn rename_preserves_extension_selects_new_file_and_preserves_caches() {
    let dir = tempdir().unwrap();
    write_png(&dir.path().join("old.jpg"));
    write_png(&dir.path().join("z.png"));
    let (mut app, _rx) = make_app_for_dir(dir.path(), 0, AppState::Browser);
    let old_key = app_key(&app, 0);
    app.protocol_cache.put(old_key.clone(), make_protocol());
    app.requested
        .insert((old_key.clone(), LoadSize::Thumbnail { w: 1, h: 1 }));
    app.insert_fullscreen_original(old_key.clone(), Arc::new(image::RgbaImage::new(1, 1)));
    let generation = app.directory_generation;

    app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
    set_rename_input(&mut app, "new");
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert!(!dir.path().join("old.jpg").exists());
    assert!(dir.path().join("new.jpg").exists());
    assert_eq!(app.images[app.selected].filename, "new.jpg");
    assert_eq!(app.sort_mode, ImageSortMode::Name);
    assert!(app.protocol_cache.contains(&old_key));
    assert!(!app.requested.is_empty());
    assert!(app.fullscreen_original_cache.contains(&old_key));
    assert_eq!(app.directory_generation, generation);
}

#[test]
fn rename_existing_target_requires_confirmation_and_can_cancel() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("old.png"), b"source").unwrap();
    fs::write(dir.path().join("target.png"), b"target").unwrap();
    let (mut app, _rx) = make_app_for_dir(dir.path(), 0, AppState::Browser);

    app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
    set_rename_input(&mut app, "target");
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert!(app.rename.as_ref().unwrap().pending_overwrite);
    app.handle_key(KeyCode::Char('n'), KeyModifiers::NONE);

    assert!(app.rename.is_none());
    assert_eq!(fs::read(dir.path().join("old.png")).unwrap(), b"source");
    assert_eq!(fs::read(dir.path().join("target.png")).unwrap(), b"target");
}

#[test]
fn rename_existing_target_overwrites_after_confirmation() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("old.png"), b"source").unwrap();
    fs::write(dir.path().join("target.png"), b"target").unwrap();
    let (mut app, _rx) = make_app_for_dir(dir.path(), 0, AppState::Browser);

    app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
    set_rename_input(&mut app, "target");
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
    app.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);

    assert!(app.rename.is_none());
    assert!(!dir.path().join("old.png").exists());
    assert_eq!(fs::read(dir.path().join("target.png")).unwrap(), b"source");
    assert_eq!(image_names(&app), vec!["target.png"]);
    assert_eq!(app.images[app.selected].filename, "target.png");
}

#[test]
fn rename_refresh_keeps_size_sort_order_and_selected_image() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("large.png"), b"123456789").unwrap();
    fs::write(dir.path().join("small.png"), b"1").unwrap();
    let (mut app, _rx) = make_app_for_dir(dir.path(), 0, AppState::Browser);
    app.sort_mode = ImageSortMode::Size;
    sort_image_entries(&mut app.images, app.sort_mode);
    app.selected = app
        .images
        .iter()
        .position(|entry| entry.filename == "small.png")
        .unwrap();

    app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
    set_rename_input(&mut app, "renamed");
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert_eq!(app.sort_mode, ImageSortMode::Size);
    assert_eq!(image_names(&app), vec!["large.png", "renamed.png"]);
    assert_eq!(app.images[app.selected].filename, "renamed.png");
}

#[test]
fn fullscreen_rename_keeps_fullscreen_and_requests_new_original() {
    let dir = tempdir().unwrap();
    write_png(&dir.path().join("old.png"));
    let (mut app, rx) = make_app_for_dir(dir.path(), 0, AppState::Fullscreen);
    app.set_fullscreen_viewport(80, 40);
    while rx.try_recv().is_ok() {}

    app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
    set_rename_input(&mut app, "new");
    app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    assert_eq!(app.state, AppState::Fullscreen);
    assert_eq!(app.images[app.selected].filename, "new.png");
    assert!(app.fullscreen_pending);

    let request = rx.try_recv().unwrap();
    assert_eq!(request.path, dir.path().join("new.png"));
    assert_eq!(request.size, selected_original_size(80, 40));
    assert_eq!(request.generation, app.directory_generation);
}

// ---- Search tests ----

fn make_app_with_names(names: &[&str]) -> App {
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
