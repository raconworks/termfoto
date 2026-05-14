use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};
use image::DynamicImage;
use crate::app::{App, CELL_WIDTH, CELL_HEIGHT};

pub struct GridView<'a> {
    pub app: &'a App,
}

impl<'a> Widget for GridView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let cell_w = CELL_WIDTH as u16;
        let cell_h = CELL_HEIGHT as u16;
        let visible_rows = (area.height / cell_h) as usize;

        for (slot, entry) in self
            .app
            .images
            .iter()
            .enumerate()
            .skip(self.app.scroll_offset * self.app.grid_cols)
            .take(visible_rows * self.app.grid_cols)
        {
            let vis_idx = slot - self.app.scroll_offset * self.app.grid_cols;
            let col = (vis_idx % self.app.grid_cols) as u16;
            let row = (vis_idx / self.app.grid_cols) as u16;

            let x = area.x + col * cell_w;
            let y = area.y + row * cell_h;

            if x + cell_w > area.x + area.width || y + cell_h > area.y + area.height {
                continue;
            }

            let cell_area = Rect { x, y, width: cell_w, height: cell_h };
            let is_selected = slot == self.app.selected;

            render_grid_cell(cell_area, buf, entry.thumbnail.as_ref(), &entry.filename, is_selected);
        }
    }
}

fn render_grid_cell(
    area: Rect,
    buf: &mut Buffer,
    thumbnail: Option<&DynamicImage>,
    filename: &str,
    selected: bool,
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

    let thumb_height = inner.height.saturating_sub(1);
    let thumb_area = Rect { height: thumb_height, ..inner };
    let name_area = Rect {
        y: inner.y + thumb_height,
        height: 1,
        ..inner
    };

    if let Some(img) = thumbnail {
        render_thumbnail_to_buf(img, thumb_area, buf);
    }

    let max_chars = inner.width as usize;
    let truncated = truncate_str(filename, max_chars);
    let name_style = if selected {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };
    buf.set_string(name_area.x, name_area.y, &truncated, name_style);
}

/// Renders image as Unicode half-block chars (▀) into ratatui Buffer.
/// Each character represents 2 pixel rows (top = fg, bottom = bg).
pub fn render_thumbnail_to_buf(img: &DynamicImage, area: Rect, buf: &mut Buffer) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let pixel_w = area.width as u32;
    let pixel_h = (area.height as u32) * 2;
    let scaled = img.resize_exact(pixel_w, pixel_h, image::imageops::FilterType::Nearest);
    let rgba = scaled.to_rgba8();

    for cy in 0..area.height {
        for cx in 0..area.width {
            let px = cx as u32;
            let py_top = (cy as u32) * 2;
            let py_bot = py_top + 1;

            let top = pixel_color(&rgba, px, py_top);
            let bot = pixel_color(&rgba, px, py_bot);

            if let Some(cell) = buf.cell_mut((area.x + cx, area.y + cy)) {
                cell.set_char('▀').set_fg(top).set_bg(bot);
            }
        }
    }
}

fn pixel_color(img: &image::RgbaImage, x: u32, y: u32) -> Color {
    if x < img.width() && y < img.height() {
        let p = img.get_pixel(x, y);
        Color::Rgb(p[0], p[1], p[2])
    } else {
        Color::Black
    }
}

fn truncate_str(s: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = chars[..max_chars.saturating_sub(1)].iter().collect();
        format!("{}…", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_str_short_string_unchanged() {
        assert_eq!(truncate_str("img.png", 20), "img.png");
    }

    #[test]
    fn test_truncate_str_long_string_truncated() {
        let result = truncate_str("very_long_filename.png", 10);
        assert!(result.chars().count() <= 10);
        assert!(result.ends_with('…'));
    }

    #[test]
    fn test_truncate_str_zero_max() {
        assert_eq!(truncate_str("abc", 0), "");
    }

    #[test]
    fn test_render_thumbnail_to_buf_does_not_panic() {
        use ratatui::{backend::TestBackend, Terminal};
        let backend = TestBackend::new(20, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let img = DynamicImage::new_rgb8(4, 4);

        terminal
            .draw(|f| {
                let area = Rect { x: 0, y: 0, width: 10, height: 5 };
                render_thumbnail_to_buf(&img, area, f.buffer_mut());
            })
            .unwrap();
    }

    #[test]
    fn test_render_thumbnail_to_buf_zero_area_no_panic() {
        use ratatui::{backend::TestBackend, Terminal};
        let backend = TestBackend::new(20, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let img = DynamicImage::new_rgb8(4, 4);

        terminal
            .draw(|f| {
                let area = Rect { x: 0, y: 0, width: 0, height: 0 };
                render_thumbnail_to_buf(&img, area, f.buffer_mut());
            })
            .unwrap();
    }
}
