use super::*;

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
