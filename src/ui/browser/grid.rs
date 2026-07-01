use crate::app::App;
use ratatui::{buffer::Buffer, layout::Rect};

use super::cell::{render_browser_cell, CellMeta};

pub(super) struct GalleryGrid {
    x: u16,
    top: u16,
    cell_w: u16,
    cell_h: u16,
    bounds: Rect,
}

pub(super) fn gallery_grid(
    bounds: Rect,
    grid_cols: usize,
    cell_w: u16,
    cell_h: u16,
    visible_rows: usize,
) -> GalleryGrid {
    let grid_h = (visible_rows as u16) * cell_h;
    let top = bounds.y + (bounds.height.saturating_sub(grid_h)) / 2;
    let grid_w = grid_cols as u16 * cell_w;
    let x = bounds.x + (bounds.width.saturating_sub(grid_w)) / 2;
    GalleryGrid {
        x,
        top,
        cell_w,
        cell_h,
        bounds,
    }
}

pub(super) fn render_gallery_slot(
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
        cache_key: app.image_cache_key_for_slot(slot),
    };
    render_browser_cell(cell_area, buf, &cell_meta, &app.protocol_cache);
}
