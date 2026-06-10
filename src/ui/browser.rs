use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, Widget},
};
use ratatui_image::{protocol::Protocol, Image};
use crate::app::{App, LoadSize, IMAGES_PER_ROW};
use crate::ui::search::SearchBar;

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

        let status_height = 1u16;
        let grid_area = Rect {
            height: area.height.saturating_sub(status_height),
            ..area
        };
        let status_area = Rect {
            y: area.y + grid_area.height,
            height: status_height,
            ..area
        };

        let cell_w = self.cell_w.max(1);
        let cell_h = self.cell_h.max(4);
        let visible_rows = (grid_area.height / cell_h) as usize;

        self.app.clamp_scroll(visible_rows);

        let start = self.app.scroll_row * IMAGES_PER_ROW;
        let end = (start + visible_rows * IMAGES_PER_ROW).min(self.app.images.len());

        let search_matches: Option<&[usize]> = self
            .app
            .search
            .as_ref()
            .map(|s| s.matches.as_slice());

        for slot in start..end {
            let vis_idx = slot - start;
            let col = (vis_idx % IMAGES_PER_ROW) as u16;
            let row = (vis_idx / IMAGES_PER_ROW) as u16;

            let x = grid_area.x + col * cell_w;
            let y = grid_area.y + row * cell_h;
            let cell_area = Rect { x, y, width: cell_w, height: cell_h };

            if x + cell_w > grid_area.x + grid_area.width || y + cell_h > grid_area.y + grid_area.height {
                continue;
            }

            let is_selected = slot == self.app.selected;
            let in_matches = search_matches.map_or(false, |m| m.contains(&slot));
            let search_query = self.app.search.as_ref().map(|s| s.query.as_str());

            render_browser_cell(
                cell_area,
                buf,
                &self.app.images[slot].filename,
                is_selected,
                in_matches,
                &self.app.protocol_cache,
                slot,
                search_query,
            );
        }

        // Status bar: search bar or normal status
        if let Some(ref search) = self.app.search {
            SearchBar {
                state: search,
                total: self.app.images.len(),
            }.render(status_area, buf);
        } else {
            let selected_name = self
                .app
                .images
                .get(self.app.selected)
                .map(|e| e.filename.as_str())
                .unwrap_or("");
            let info = format!(
                " {} [{}/{}]  ←→↑↓ 导航  PgUp/PgDown/Space翻页  Home/End首尾  Enter全屏  q退出",
                selected_name,
                self.app.selected.saturating_add(1).min(self.app.images.len()),
                self.app.images.len(),
            );
            let span = Span::styled(info, Style::default().fg(Color::White).bg(Color::DarkGray));
            Paragraph::new(span)
                .alignment(Alignment::Left)
                .render(status_area, buf);
        }
    }
}

fn render_browser_cell(
    area: Rect,
    buf: &mut Buffer,
    filename: &str,
    selected: bool,
    search_match: bool,
    cache: &std::collections::HashMap<usize, Protocol>,
    slot: usize,
    search_query: Option<&str>,
) {
    let border_style = if selected {
        // Both selected and search match: bright yellow
        if search_match {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Cyan)
        }
    } else if search_match {
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
    if let Some(proto) = cache.get(&slot) {
        let proto_size = proto.size();
        let offset_x = thumb_area
            .width
            .saturating_sub(proto_size.width)
            / 2;
        let offset_y = thumb_area
            .height
            .saturating_sub(proto_size.height)
            / 2;
        let centered = Rect {
            x: thumb_area.x + offset_x,
            y: thumb_area.y + offset_y,
            width: proto_size.width.min(thumb_area.width),
            height: proto_size.height.min(thumb_area.height),
        };
        Image::new(proto).allow_clipping(true).render(centered, buf);
    }

    // Render filename with match highlighting if in search mode
    let matched_char_style = if selected || search_match {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Rgb(200, 200, 0))
    };
    let normal_style = if selected {
        Style::default().fg(Color::Cyan)
    } else if search_match {
        Style::default().fg(Color::Rgb(200, 200, 200))
    } else {
        Style::default().fg(Color::White)
    };

    if let Some(query) = search_query {
        if !query.is_empty() {
            render_filename_with_highlight(
                name_area, buf, filename, query,
                matched_char_style, normal_style,
            );
            return;
        }
    }

    // No search / empty query: centered single-span filename
    let span: Span;
    if selected {
        span = Span::styled(filename.to_string(), Style::default().fg(Color::Cyan));
    } else if search_match {
        span = Span::styled(filename.to_string(), Style::default().fg(Color::Rgb(200, 200, 200)));
    } else {
        span = Span::styled(filename.to_string(), Style::default().fg(Color::White));
    }
    let name_width = filename.chars().count() as u16;
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
    terminal_width: u16,
    visible_rows: usize,
) {
    if cell_w < 2 || cell_h < 2 {
        return;
    }

    // Clear cache if terminal width changed (cell size changed, old protocols invalid)
    if app.cache_width != terminal_width {
        app.clear_protocol_cache();
        app.cache_width = terminal_width;
    }

    let thumb_w = cell_w.saturating_sub(2);
    let thumb_h = cell_h.saturating_sub(3); // minus border + filename row
    let size = LoadSize::Thumbnail { w: thumb_w, h: thumb_h };

    let start = app.scroll_row * IMAGES_PER_ROW;
    let visible_end = (start + visible_rows * IMAGES_PER_ROW).min(app.images.len());

    // Prefetch: extend range by ±1 row
    let prefetch_start = start.saturating_sub(IMAGES_PER_ROW);
    let prefetch_end = (visible_end + IMAGES_PER_ROW).min(app.images.len());

    for slot in prefetch_start..prefetch_end {
        if app.protocol_cache.contains_key(&slot) || app.requested.contains(&slot) {
            continue;
        }
        app.request_load(slot, size.clone());
    }
}
