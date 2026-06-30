use crate::app::{App, BrowserFocus, LoadSize};
use crate::ui::layout::three_panel_areas;
use crate::ui::search::SearchBar;
use crate::ui::{
    render_directory_context, render_info_panel, render_panel, render_prompt_base,
    render_prompt_lines,
};
use lru::LruCache;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Widget},
};
use ratatui_image::{protocol::Protocol, Image};

/// Truncate filename to fit cell width, appending "…" if needed.
fn truncate_filename(name: &str, max_width: u16) -> String {
    let max = max_width as usize;
    if name.chars().count() <= max {
        name.to_string()
    } else {
        let mut s: String = name.chars().take(max.saturating_sub(1)).collect();
        s.push('…');
        s
    }
}

pub struct BrowserView<'a> {
    pub app: &'a mut App,
    pub cell_w: u16,
    pub cell_h: u16,
}

impl<'a> Widget for BrowserView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let cell_w = self.cell_w.max(1);
        let cell_h = self.cell_h.max(1);

        let areas = three_panel_areas(area);
        let context_inner = render_panel(
            areas.context,
            self.app.lang.title_context(),
            self.app.browser_focus == BrowserFocus::Context,
            buf,
        );
        let gallery_inner = render_panel(
            areas.gallery,
            self.app.lang.title_gallery(),
            self.app.browser_focus == BrowserFocus::Gallery,
            buf,
        );
        let info_inner = render_panel(areas.info, self.app.lang.title_info(), false, buf);

        let context_entries = self.app.directory_context_for_browser();
        self.app
            .clamp_context_selection(context_entries.len(), context_inner.height as usize);
        render_directory_context(
            context_inner,
            &context_entries,
            self.app.lang.empty_folder_context(),
            Some(self.app.context_selected),
            self.app.context_scroll,
            buf,
        );
        render_info_panel(
            info_inner,
            self.app.images.get(self.app.selected),
            None,
            self.app,
            buf,
        );

        let available = gallery_inner.height;

        // Grid centered both horizontally and vertically
        let visible_rows = (available / cell_h).max(1) as usize;
        self.app.visible_rows = visible_rows;
        let grid_h = (visible_rows as u16) * cell_h;
        let grid_top = gallery_inner.y + (available.saturating_sub(grid_h)) / 2;

        self.app.clamp_scroll(visible_rows);

        // Center the grid horizontally
        let grid_w = self.app.grid_cols as u16 * cell_w;
        let grid_x = gallery_inner.x + (gallery_inner.width.saturating_sub(grid_w)) / 2;
        let grid = GalleryGrid {
            x: grid_x,
            top: grid_top,
            cell_w,
            cell_h,
            bounds: gallery_inner,
        };

        let search_matches: Option<&[usize]> =
            self.app.search.as_ref().map(|s| s.matches.as_slice());

        let favorite_row_len = self.app.favorite_row_len();
        let normal_visible_rows = self.app.normal_visible_rows(visible_rows);
        let first_normal_row = if favorite_row_len > 0 { 1 } else { 0 };

        if visible_rows > 0 {
            for slot in 0..favorite_row_len {
                let col = slot as u16;
                render_gallery_slot(self.app, slot, col, 0, &grid, search_matches, buf);
            }
        }

        let start = favorite_row_len + self.app.scroll_row * self.app.grid_cols;
        let end = (start + normal_visible_rows * self.app.grid_cols).min(self.app.images.len());

        for slot in start..end {
            let vis_idx = slot - start;
            let col = (vis_idx % self.app.grid_cols) as u16;
            let row = first_normal_row as u16 + (vis_idx / self.app.grid_cols) as u16;
            render_gallery_slot(self.app, slot, col, row, &grid, search_matches, buf);
        }

        if let Some(lines) = self.app.rename_prompt_lines() {
            render_prompt_lines(areas.prompt, &lines, buf);
        } else if let Some(ref search) = self.app.search {
            render_prompt_base(areas.prompt, buf);
            SearchBar {
                state: search,
                lang: self.app.lang,
            }
            .render(areas.prompt, buf);
        } else {
            let selected_name = self
                .app
                .images
                .get(self.app.selected)
                .map(|e| e.filename.as_str())
                .unwrap_or("");
            let mut lines = if self.app.is_favorites_view() {
                self.app.lang.favorites_prompt_lines(
                    selected_name,
                    self.app
                        .selected
                        .saturating_add(1)
                        .min(self.app.images.len()),
                    self.app.images.len(),
                )
            } else {
                match self.app.browser_focus {
                    BrowserFocus::Gallery => self.app.lang.browser_prompt_lines(
                        selected_name,
                        self.app
                            .selected
                            .saturating_add(1)
                            .min(self.app.images.len()),
                        self.app.images.len(),
                        self.app.sort_label(),
                    ),
                    BrowserFocus::Context => {
                        let context_name = context_entries
                            .get(self.app.context_selected)
                            .map(|entry| entry.name.as_str())
                            .unwrap_or("");
                        self.app.lang.context_prompt_lines(
                            context_name,
                            self.app
                                .context_selected
                                .saturating_add(1)
                                .min(context_entries.len()),
                            context_entries.len(),
                            self.app.sort_label(),
                        )
                    }
                }
            };
            if let Some(message) = self.app.browser_status_message() {
                lines[0] = self.app.lang.status_prompt_line(&message);
            }
            render_prompt_lines(areas.prompt, &lines, buf);
        }
    }
}

struct GalleryGrid {
    x: u16,
    top: u16,
    cell_w: u16,
    cell_h: u16,
    bounds: Rect,
}

fn render_gallery_slot(
    app: &App,
    slot: usize,
    col: u16,
    row: u16,
    grid: &GalleryGrid,
    search_matches: Option<&[usize]>,
    buf: &mut Buffer,
) {
    let x = grid.x + col * grid.cell_w;
    let y = grid.top + row * grid.cell_h;
    let cell_area = Rect {
        x,
        y,
        width: grid.cell_w,
        height: grid.cell_h,
    };

    if x + grid.cell_w > grid.bounds.x + grid.bounds.width
        || y + grid.cell_h > grid.bounds.y + grid.bounds.height
    {
        return;
    }

    let is_selected = slot == app.selected;
    let in_matches = search_matches.is_some_and(|m| m.contains(&slot));
    let search_query = app.search.as_ref().map(|s| s.query.as_str());
    let is_favorite = app.is_favorite_index(slot);

    let Some(entry) = app.images.get(slot) else {
        return;
    };
    let cell_meta = CellMeta {
        filename: &entry.filename,
        selected: is_selected,
        search_match: in_matches,
        search_query,
        favorite: is_favorite,
        favorite_label: is_favorite.then(|| app.lang.favorite_badge()),
        slot,
    };
    render_browser_cell(cell_area, buf, &cell_meta, &app.protocol_cache);
}

struct CellMeta<'a> {
    filename: &'a str,
    selected: bool,
    search_match: bool,
    search_query: Option<&'a str>,
    favorite: bool,
    favorite_label: Option<&'a str>,
    slot: usize,
}

fn render_browser_cell(
    area: Rect,
    buf: &mut Buffer,
    meta: &CellMeta,
    cache: &LruCache<usize, Protocol>,
) {
    let border_style = if meta.selected {
        // Both selected and search match: bright yellow
        if meta.search_match {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Cyan)
        }
    } else if meta.search_match {
        // Search match but not selected: dim yellow
        Style::default().fg(Color::Rgb(128, 128, 0))
    } else if meta.favorite {
        Style::default().fg(Color::Magenta)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .render(area, buf);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    // Truncate filename to fit cell width
    let name = truncate_filename(meta.filename, inner.width);

    let badge_height = meta.favorite_label.is_some() as u16;
    let name_height = 1u16;
    let badge_area = Rect {
        y: inner.y,
        height: badge_height,
        ..inner
    };
    let thumb_area = Rect {
        y: inner.y + badge_height,
        height: inner.height.saturating_sub(name_height + badge_height),
        ..inner
    };
    let name_area = Rect {
        y: inner.y + badge_height + thumb_area.height,
        height: name_height,
        ..inner
    };

    if let Some(label) = meta.favorite_label {
        let label = truncate_filename(label, badge_area.width);
        let style = if meta.selected {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::Magenta)
        };
        let label_width = label.chars().count() as u16;
        let label_x = badge_area.x + (badge_area.width.saturating_sub(label_width)) / 2;
        buf.set_span(
            label_x,
            badge_area.y,
            &Span::styled(label, style),
            label_width,
        );
    }

    // Render chafa thumbnail centered
    if let Some(proto) = cache.peek(&meta.slot) {
        let proto_size = proto.size();
        let offset_x = thumb_area.width.saturating_sub(proto_size.width) / 2;
        let offset_y = thumb_area.height.saturating_sub(proto_size.height) / 2;
        let centered = Rect {
            x: thumb_area.x + offset_x,
            y: thumb_area.y + offset_y,
            width: proto_size.width.min(thumb_area.width),
            height: proto_size.height.min(thumb_area.height),
        };
        Image::new(proto).allow_clipping(true).render(centered, buf);
    }

    // Render filename with match highlighting if in search mode
    let matched_char_style = if meta.selected || meta.search_match {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Rgb(200, 200, 0))
    };
    let normal_style = if meta.selected {
        Style::default().fg(Color::Cyan)
    } else if meta.search_match {
        Style::default().fg(Color::Rgb(200, 200, 200))
    } else if meta.favorite {
        Style::default().fg(Color::Magenta)
    } else {
        Style::default().fg(Color::White)
    };

    if let Some(query) = meta.search_query {
        if !query.is_empty() {
            render_filename_with_highlight(
                name_area,
                buf,
                &name,
                query,
                matched_char_style,
                normal_style,
            );
            return;
        }
    }

    // No search / empty query: centered single-span filename
    let span: Span;
    if meta.selected {
        span = Span::styled(name.clone(), Style::default().fg(Color::Cyan));
    } else if meta.search_match {
        span = Span::styled(name.clone(), Style::default().fg(Color::Rgb(200, 200, 200)));
    } else if meta.favorite {
        span = Span::styled(name.clone(), Style::default().fg(Color::Magenta));
    } else {
        span = Span::styled(name.clone(), Style::default().fg(Color::White));
    }
    let name_width = name.chars().count() as u16;
    let name_x = name_area.x + (name_area.width.saturating_sub(name_width)) / 2;
    buf.set_span(name_x, name_area.y, &span, name_width);
}

/// Render filename with matched characters highlighted in `match_style`.
fn render_filename_with_highlight(
    area: Rect,
    buf: &mut Buffer,
    filename: &str,
    query: &str,
    match_style: Style,
    normal_style: Style,
) {
    let mut spans: Vec<Span> = Vec::new();
    let filename_lower = filename.to_lowercase();
    let query_chars: Vec<char> = query.to_lowercase().chars().collect();
    let mut qi = 0;

    let filename_chars: Vec<char> = filename.chars().collect();
    let filename_lower_chars: Vec<char> = filename_lower.chars().collect();

    for (i, ch) in filename_chars.iter().enumerate() {
        if qi < query_chars.len() && filename_lower_chars[i] == query_chars[qi] {
            spans.push(Span::styled(ch.to_string(), match_style));
            qi += 1;
        } else {
            spans.push(Span::styled(ch.to_string(), normal_style));
        }
    }

    let total_width: usize = spans.iter().map(|s| s.width()).sum();
    let start_x = area.x + area.width.saturating_sub(total_width as u16) / 2;
    let mut x = start_x;
    for span in &spans {
        let w = span.width() as u16;
        buf.set_span(x, area.y, span, w);
        x += w;
    }
}

/// Request chafa protocol generation for visible images + prefetch adjacent rows.
pub fn populate_protocol_cache(
    app: &mut App,
    cell_w: u16,
    cell_h: u16,
    term_size: ratatui::layout::Size,
) {
    if cell_w < 2 || cell_h < 2 {
        app.clear_thumbnail_interest();
        return;
    }

    // Clear cache if terminal size changed (cell dimensions invalid)
    if app.cache_width != term_size.width || app.cache_height != term_size.height {
        app.clear_protocol_cache();
        app.cache_width = term_size.width;
        app.cache_height = term_size.height;
    }

    let thumb_w = cell_w.saturating_sub(2);
    let thumb_h = cell_h.saturating_sub(3); // minus border + filename row
    app.thumb_w = thumb_w;
    app.thumb_h = thumb_h;
    let size = LoadSize::Thumbnail {
        w: thumb_w,
        h: thumb_h,
    };

    let request_order = thumbnail_request_order_for_app(app);
    app.update_thumbnail_interest(thumb_w, thumb_h, request_order.iter().copied());

    for slot in request_order {
        if app.protocol_cache.get(&slot).is_some()
            || app
                .requested
                .contains(&(app.directory_generation, slot, size.clone()))
        {
            continue;
        }
        app.request_load(slot, size.clone());
    }
}

fn thumbnail_request_order_for_app(app: &App) -> Vec<usize> {
    let favorite_row_len = app.favorite_row_len();
    if favorite_row_len == 0 {
        let start = app.scroll_row * app.grid_cols;
        let visible_end = (start + app.visible_rows * app.grid_cols).min(app.images.len());
        return thumbnail_request_order(start, visible_end, app.grid_cols, app.images.len());
    }

    let grid_cols = app.grid_cols.max(1);
    let normal_visible_rows = app.normal_visible_rows(app.visible_rows);
    let normal_start = favorite_row_len + app.scroll_row * grid_cols;
    let normal_visible_end = (normal_start + normal_visible_rows * grid_cols).min(app.images.len());
    let previous_start = favorite_row_len + app.scroll_row.saturating_sub(1) * grid_cols;
    let previous_end = normal_start;
    let next_start = normal_visible_end;
    let next_end = (next_start + grid_cols).min(app.images.len());

    let mut slots = Vec::new();
    slots.extend(0..favorite_row_len);
    slots.extend(normal_start..normal_visible_end);
    if app.scroll_row > 0 {
        slots.extend(previous_start..previous_end);
    }
    slots.extend(next_start..next_end);
    slots
}

fn thumbnail_request_order(
    visible_start: usize,
    visible_end: usize,
    grid_cols: usize,
    total_images: usize,
) -> Vec<usize> {
    let visible_start = visible_start.min(total_images);
    let visible_end = visible_end.min(total_images).max(visible_start);
    let prefetch_start = visible_start.saturating_sub(grid_cols);
    let prefetch_end = (visible_end + grid_cols).min(total_images);

    let mut slots = Vec::with_capacity(prefetch_end.saturating_sub(prefetch_start));
    slots.extend(visible_start..visible_end);
    slots.extend(prefetch_start..visible_start);
    slots.extend(visible_end..prefetch_end);
    slots
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppStart, AppState, LoadRequest, LoadResult};
    use crate::lang::Lang;
    use crate::scanner::ImageEntry;
    use ratatui_image::picker::Picker;
    use std::fs;
    use tempfile::{tempdir, TempDir};

    fn buffer_text(buf: &Buffer) -> String {
        buf.content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
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
                8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 0, 1, 2, 3, 4, 5, 6,
                7, 24, 25, 26, 27, 28, 29, 30, 31,
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
            .flat_map(|y| {
                (areas.context.x..areas.context.x + areas.context.width).map(move |x| (x, y))
            })
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
}
