use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};
use ratatui_image::{protocol::Protocol, Image, Resize, FilterType};
use crate::app::{App, IMAGES_PER_ROW};

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
        let cell_h = self.cell_h.max(4);
        let visible_rows = (area.height / cell_h) as usize;

        self.app.clamp_scroll(visible_rows);

        let start = self.app.scroll_row * IMAGES_PER_ROW;
        let end = (start + visible_rows * IMAGES_PER_ROW).min(self.app.images.len());

        for slot in start..end {
            let vis_idx = slot - start;
            let col = (vis_idx % IMAGES_PER_ROW) as u16;
            let row = (vis_idx / IMAGES_PER_ROW) as u16;

            let x = area.x + col * cell_w;
            let y = area.y + row * cell_h;
            let cell_area = Rect { x, y, width: cell_w, height: cell_h };

            if x + cell_w > area.x + area.width || y + cell_h > area.y + area.height {
                continue;
            }

            let is_selected = slot == self.app.selected;
            render_browser_cell(
                cell_area,
                buf,
                &self.app.images[slot].filename,
                is_selected,
                &self.app.protocol_cache,
                slot,
            );
        }
    }
}

fn render_browser_cell(
    area: Rect,
    buf: &mut Buffer,
    filename: &str,
    selected: bool,
    cache: &std::collections::HashMap<usize, Protocol>,
    slot: usize,
) {
    let border_style = if selected {
        Style::default().fg(Color::Cyan)
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

    // Render filename centered
    let max_chars = inner.width as usize;
    let truncated = if max_chars == 0 {
        String::new()
    } else {
        let chars: Vec<char> = filename.chars().collect();
        if chars.len() <= max_chars {
            filename.to_string()
        } else {
            let truncated: String = chars[..max_chars.saturating_sub(1)].iter().collect();
            format!("{}…", truncated)
        }
    };

    let name_style = if selected {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };
    let name_width = truncated.chars().count() as u16;
    let name_x = name_area.x + (name_area.width.saturating_sub(name_width)) / 2;
    buf.set_string(name_x, name_area.y, &truncated, name_style);
}

/// Create chafa protocols for images in the visible range that aren't cached yet.
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

    // Clear cache if terminal width changed (cell size changed)
    if app.cache_width != terminal_width {
        app.clear_protocol_cache();
        app.cache_width = terminal_width;
    }

    let start = app.scroll_row * IMAGES_PER_ROW;
    let end = (start + visible_rows * IMAGES_PER_ROW).min(app.images.len());

    let thumb_w = cell_w.saturating_sub(2);
    let thumb_h = cell_h.saturating_sub(3); // minus border + filename row

    for slot in start..end {
        if app.protocol_cache.contains_key(&slot) {
            continue;
        }
        if let Ok(img) = image::open(&app.images[slot].path) {
            let size = ratatui::layout::Size::new(thumb_w, thumb_h);
            if let Ok(proto) = app.picker.new_protocol(
                img,
                size,
                Resize::Fit(Some(FilterType::Lanczos3)),
            ) {
                app.protocol_cache.insert(slot, proto);
            }
        }
    }
}
