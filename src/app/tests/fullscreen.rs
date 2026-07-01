use super::*;

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
