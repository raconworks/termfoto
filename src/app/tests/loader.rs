use super::*;

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
