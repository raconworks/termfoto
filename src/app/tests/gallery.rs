use super::*;

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
