use super::thumbnails::{thumbnail_request_order, thumbnail_request_order_for_app};
use super::*;
use crate::app::{AppStart, AppState, LoadRequest, LoadResult};
use crate::lang::Lang;
use crate::scanner::ImageEntry;
use crate::ui::layout::three_panel_areas;
use ratatui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};
use ratatui_image::picker::Picker;
use std::fs;
use tempfile::{tempdir, TempDir};

fn buffer_text(buf: &Buffer) -> String {
    buf.content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>()
}

fn row_text(buf: &Buffer, area: Rect, y: u16) -> String {
    (area.x..area.x + area.width)
        .map(|x| buf.cell((x, y)).unwrap().symbol())
        .collect()
}

fn render_test_app() -> (TempDir, App) {
    let dir = tempdir().unwrap();
    let photos = dir.path().join("photos");
    fs::create_dir(&photos).unwrap();
    let image_path = photos.join("sample.png");
    fs::write(&image_path, b"sample").unwrap();

    let images = vec![ImageEntry {
        path: image_path,
        filename: "sample.png".to_string(),
        file_size: 6,
        modified_at: None,
    }];
    let (tx, _rx) = std::sync::mpsc::channel::<LoadRequest>();
    let (_tx2, rx2) = std::sync::mpsc::channel::<LoadResult>();
    (
        dir,
        App::new(
            AppStart {
                images,
                image_dir: photos,
                state: AppState::Browser,
                selected: 0,
            },
            tx,
            rx2,
            Lang::En,
            Picker::halfblocks(),
        ),
    )
}

#[test]
fn thumbnail_request_order_prioritizes_visible_slots() {
    let slots = thumbnail_request_order(8, 24, 8, 40);

    assert_eq!(
        slots,
        vec![
            8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 0, 1, 2, 3, 4, 5, 6, 7,
            24, 25, 26, 27, 28, 29, 30, 31,
        ]
    );
}

#[test]
fn thumbnail_request_order_clamps_edges() {
    assert_eq!(thumbnail_request_order(0, 6, 8, 6), vec![0, 1, 2, 3, 4, 5]);
    assert_eq!(
        thumbnail_request_order(16, 24, 8, 20),
        vec![16, 17, 18, 19, 8, 9, 10, 11, 12, 13, 14, 15]
    );
}

#[test]
fn thumbnail_request_order_includes_pinned_row_and_visible_normal_rows() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("img2.png"), b"favorite").unwrap();
    let images = (0..5)
        .map(|idx| ImageEntry {
            path: dir.path().join(format!("img{idx}.png")),
            filename: format!("img{idx}.png"),
            file_size: 0,
            modified_at: None,
        })
        .collect();
    let (tx, _rx) = std::sync::mpsc::channel::<LoadRequest>();
    let (_tx2, rx2) = std::sync::mpsc::channel::<LoadResult>();
    let mut app = App::new(
        AppStart {
            images,
            image_dir: dir.path().to_path_buf(),
            state: AppState::Browser,
            selected: 0,
        },
        tx,
        rx2,
        Lang::En,
        Picker::halfblocks(),
    );
    app.set_favorite_store_path_for_tests(dir.path().join("favorites.tsv"));
    app.set_grid_layout(2, 2);
    app.add_favorite_for_tests(&dir.path().join("img2.png"), 10);
    app.scroll_row = 1;

    assert_eq!(thumbnail_request_order_for_app(&app), vec![0, 3, 4, 1, 2]);
}

#[test]
fn browser_render_includes_three_panel_titles_and_prompt_row() {
    let (_dir, mut app) = render_test_app();
    app.grid_cols = 2;
    app.visible_rows = 1;
    let area = Rect::new(0, 0, 100, 20);
    let mut buf = Buffer::empty(area);

    BrowserView {
        app: &mut app,
        cell_w: 24,
        cell_h: 8,
    }
    .render(area, &mut buf);

    let text = buffer_text(&buf);
    assert!(text.contains("Context"));
    assert!(text.contains("Gallery"));
    assert!(text.contains("Info"));
    let areas = three_panel_areas(area);
    assert!(!row_text(&buf, areas.gallery, areas.gallery.y).contains("Favorites"));

    let prompt_text_row = area.height - 3;
    let prompt_row: String = (0..area.width)
        .map(|x| buf.cell((x, prompt_text_row)).unwrap().symbol())
        .collect();
    assert!(prompt_row.contains("sample.png"));
}

#[test]
fn browser_render_marks_favorite_cells() {
    let (dir, mut app) = render_test_app();
    app.set_favorite_store_path_for_tests(dir.path().join("favorites.tsv"));
    let path = app.images[0].path.clone();
    app.add_favorite_for_tests(&path, 10);
    let area = Rect::new(0, 0, 100, 20);
    let mut buf = Buffer::empty(area);

    BrowserView {
        app: &mut app,
        cell_w: 24,
        cell_h: 8,
    }
    .render(area, &mut buf);

    assert!(buffer_text(&buf).contains("Favorite"));
}

#[test]
fn browser_render_shows_favorites_panel_above_gallery() {
    let dir = tempdir().unwrap();
    let favorite_path = dir.path().join("favorite.png");
    let normal_path = dir.path().join("normal.png");
    fs::write(&favorite_path, b"favorite").unwrap();
    fs::write(&normal_path, b"normal").unwrap();
    let images = vec![
        ImageEntry {
            path: favorite_path.clone(),
            filename: "favorite.png".to_string(),
            file_size: 8,
            modified_at: None,
        },
        ImageEntry {
            path: normal_path,
            filename: "normal.png".to_string(),
            file_size: 6,
            modified_at: None,
        },
    ];
    let (tx, _rx) = std::sync::mpsc::channel::<LoadRequest>();
    let (_tx2, rx2) = std::sync::mpsc::channel::<LoadResult>();
    let mut app = App::new(
        AppStart {
            images,
            image_dir: dir.path().to_path_buf(),
            state: AppState::Browser,
            selected: 0,
        },
        tx,
        rx2,
        Lang::En,
        Picker::halfblocks(),
    );
    app.set_grid_layout(2, 3);
    app.set_favorite_store_path_for_tests(dir.path().join("favorites.tsv"));
    app.add_favorite_for_tests(&favorite_path, 10);

    let area = Rect::new(0, 0, 100, 24);
    let cell_w: u16 = 20;
    let cell_h: u16 = 6;
    let areas = three_panel_areas(area);
    let favorites_height = cell_h.saturating_add(2).min(areas.gallery.height);
    let mut buf = Buffer::empty(area);

    BrowserView {
        app: &mut app,
        cell_w,
        cell_h,
    }
    .render(area, &mut buf);

    assert!(row_text(&buf, areas.gallery, areas.gallery.y).contains("Favorites"));
    assert!(row_text(&buf, areas.gallery, areas.gallery.y + favorites_height).contains("Gallery"));
}

#[test]
fn browser_focus_changes_panel_border_style() {
    let (_dir, mut app) = render_test_app();
    app.browser_focus = BrowserFocus::Context;
    let area = Rect::new(0, 0, 100, 20);
    let areas = three_panel_areas(area);
    let mut buf = Buffer::empty(area);

    BrowserView {
        app: &mut app,
        cell_w: 24,
        cell_h: 8,
    }
    .render(area, &mut buf);

    assert_eq!(
        buf.cell((areas.context.x, areas.context.y)).unwrap().fg,
        Color::Cyan
    );
    assert_eq!(
        buf.cell((areas.gallery.x, areas.gallery.y)).unwrap().fg,
        Color::DarkGray
    );
}

#[test]
fn browser_context_selection_is_highlighted() {
    let (_dir, mut app) = render_test_app();
    fs::create_dir(app.image_dir.join("album")).unwrap();
    app.browser_focus = BrowserFocus::Context;
    app.context_dir = app.image_dir.clone();
    app.context_selected = 1;
    let area = Rect::new(0, 0, 100, 20);
    let areas = three_panel_areas(area);
    let mut buf = Buffer::empty(area);

    BrowserView {
        app: &mut app,
        cell_w: 24,
        cell_h: 8,
    }
    .render(area, &mut buf);

    let highlighted = buf
        .content()
        .iter()
        .any(|cell| cell.bg == Color::Cyan && cell.symbol() != " ");
    assert!(highlighted);

    let context_text: String = (areas.context.y..areas.context.y + areas.context.height)
        .flat_map(|y| (areas.context.x..areas.context.x + areas.context.width).map(move |x| (x, y)))
        .map(|pos| buf.cell(pos).unwrap().symbol())
        .collect();
    assert!(context_text.contains("    > album/"));
}

#[test]
fn browser_render_with_no_images_leaves_gallery_info_and_filename_empty() {
    let dir = tempdir().unwrap();
    let (tx, _rx) = std::sync::mpsc::channel::<LoadRequest>();
    let (_tx2, rx2) = std::sync::mpsc::channel::<LoadResult>();
    let mut app = App::new(
        AppStart {
            images: Vec::new(),
            image_dir: dir.path().to_path_buf(),
            state: AppState::Browser,
            selected: 0,
        },
        tx,
        rx2,
        Lang::En,
        Picker::halfblocks(),
    );
    let area = Rect::new(0, 0, 100, 20);
    let mut buf = Buffer::empty(area);

    BrowserView {
        app: &mut app,
        cell_w: 24,
        cell_h: 8,
    }
    .render(area, &mut buf);

    let text = buffer_text(&buf);
    assert!(!text.contains("old.png"));
    assert!(text.contains("File      [0/0]"));
    assert!(text.contains("Sort Name"));
    assert!(text.contains("s Sort"));
}

#[test]
fn browser_prompt_changes_for_context_and_search_modes() {
    let (_dir, mut app) = render_test_app();
    app.browser_focus = BrowserFocus::Context;
    let area = Rect::new(0, 0, 100, 20);
    let mut buf = Buffer::empty(area);

    BrowserView {
        app: &mut app,
        cell_w: 24,
        cell_h: 8,
    }
    .render(area, &mut buf);

    let text = buffer_text(&buf);
    assert!(text.contains("Folder"));
    assert!(text.contains("Open Folder"));
    assert!(text.contains("Sort Name"));
    assert!(text.contains("s Sort"));

    app.handle_key(
        crossterm::event::KeyCode::Char('/'),
        crossterm::event::KeyModifiers::NONE,
    );
    let mut buf = Buffer::empty(area);
    BrowserView {
        app: &mut app,
        cell_w: 24,
        cell_h: 8,
    }
    .render(area, &mut buf);

    let text = buffer_text(&buf);
    assert!(text.contains("Cycle"));
    assert!(!text.contains("Open Folder"));
    assert!(!text.contains("s Sort"));
}
