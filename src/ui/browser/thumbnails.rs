use crate::app::{App, LoadSize};

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
        let Some(key) = app.image_cache_key_for_slot(slot) else {
            continue;
        };
        if app.protocol_cache.get(&key).is_some() || app.requested.contains(&(key, size.clone())) {
            continue;
        }
        app.request_load(slot, size.clone());
    }
}

pub(super) fn thumbnail_request_order_for_app(app: &App) -> Vec<usize> {
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

pub(super) fn thumbnail_request_order(
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
