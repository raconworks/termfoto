use crate::app::{App, LoadSize, LOGO_HEIGHT, MIN_LOGO_WIDTH};
use crate::ui::render_logo;
use crate::ui::search::SearchBar;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, Widget},
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

        let show_logo = area.width >= MIN_LOGO_WIDTH;
        let bottom_h = if show_logo { LOGO_HEIGHT } else { 1 };
        let available = area.height.saturating_sub(bottom_h);

        // Grid centered both horizontally and vertically
        let visible_rows = (available / cell_h) as usize;
        let grid_h = (visible_rows as u16) * cell_h;
        let grid_top = area.y + (available.saturating_sub(grid_h)) / 2;

        self.app.clamp_scroll(visible_rows);

        let start = self.app.scroll_row * self.app.grid_cols;
        let end = (start + visible_rows * self.app.grid_cols).min(self.app.images.len());

        // Center the grid horizontally
        let grid_w = self.app.grid_cols as u16 * cell_w;
        let grid_x = area.x + (area.width.saturating_sub(grid_w)) / 2;

        let search_matches: Option<&[usize]> =
            self.app.search.as_ref().map(|s| s.matches.as_slice());

        for slot in start..end {
            let vis_idx = slot - start;
            let col = (vis_idx % self.app.grid_cols) as u16;
            let row = (vis_idx / self.app.grid_cols) as u16;

            let x = grid_x + col * cell_w;
            let y = grid_top + row * cell_h;
            let cell_area = Rect {
                x,
                y,
                width: cell_w,
                height: cell_h,
            };

            if x + cell_w > area.x + area.width || y + cell_h > area.y + area.height {
                continue;
            }

            let is_selected = slot == self.app.selected;
            let in_matches = search_matches.is_some_and(|m| m.contains(&slot));
            let search_query = self.app.search.as_ref().map(|s| s.query.as_str());

            let cell_meta = CellMeta {
                filename: &self.app.images[slot].filename,
                selected: is_selected,
                search_match: in_matches,
                search_query,
                slot,
            };
            render_browser_cell(cell_area, buf, &cell_meta, &self.app.protocol_cache);
        }

        // Logo: bottom-right 6 rows, status bar shares the last row
        if show_logo {
            let logo_area = Rect {
                y: area.y + area.height.saturating_sub(LOGO_HEIGHT),
                height: LOGO_HEIGHT,
                ..area
            };
            render_logo(logo_area, buf);
        }

        // Status bar: bottom row, left-aligned (logo already rendered on right)
        let status_area = Rect {
            y: area.y + area.height.saturating_sub(1),
            height: 1,
            ..area
        };
        if let Some(ref search) = self.app.search {
            SearchBar {
                state: search,
                lang: self.app.lang,
            }
            .render(status_area, buf);
        } else {
            let selected_name = self
                .app
                .images
                .get(self.app.selected)
                .map(|e| e.filename.as_str())
                .unwrap_or("");
            let info = self.app.lang.browser_status(
                selected_name,
                self.app
                    .selected
                    .saturating_add(1)
                    .min(self.app.images.len()),
                self.app.images.len(),
            );
            let span = Span::styled(info, Style::default().fg(Color::White).bg(Color::DarkGray));
            Paragraph::new(span)
                .alignment(Alignment::Left)
                .render(status_area, buf);
        }
    }
}

struct CellMeta<'a> {
    filename: &'a str,
    selected: bool,
    search_match: bool,
    search_query: Option<&'a str>,
    slot: usize,
}

fn render_browser_cell(
    area: Rect,
    buf: &mut Buffer,
    meta: &CellMeta,
    cache: &std::collections::HashMap<usize, Protocol>,
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

    let name_height = 1u16;
    let thumb_area = Rect {
        y: inner.y,
        height: inner.height.saturating_sub(name_height),
        ..inner
    };
    let name_area = Rect {
        y: inner.y + thumb_area.height,
        height: name_height,
        ..inner
    };

    // Render chafa thumbnail centered
    if let Some(proto) = cache.get(&meta.slot) {
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

    let start = app.scroll_row * app.grid_cols;
    let visible_end = (start + app.visible_rows * app.grid_cols).min(app.images.len());

    // Prefetch: extend range by ±1 row
    let prefetch_start = start.saturating_sub(app.grid_cols);
    let prefetch_end = (visible_end + app.grid_cols).min(app.images.len());

    for slot in prefetch_start..prefetch_end {
        if app.protocol_cache.contains_key(&slot) || app.requested.contains(&slot) {
            continue;
        }
        app.request_load(slot, size.clone());
    }
}
