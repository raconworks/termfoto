use crate::app::{App, LOGO_HEIGHT, MIN_LOGO_WIDTH};
use crate::ui::render_logo;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};
use ratatui_image::Image;

pub struct PreviewView<'a> {
    pub app: &'a App,
}

struct PreviewAreas {
    main: Rect,
    logo: Rect,
    status: Rect,
}

fn preview_areas(area: Rect, show_logo: bool) -> PreviewAreas {
    let bottom_h = if show_logo { LOGO_HEIGHT } else { 1 };
    let main_h = area.height.saturating_sub(bottom_h);
    let main = Rect {
        height: main_h,
        ..area
    };
    let logo = Rect {
        y: area.y + area.height.saturating_sub(LOGO_HEIGHT),
        height: if show_logo { LOGO_HEIGHT } else { 0 },
        ..area
    };
    let status = Rect {
        y: area.y + area.height.saturating_sub(1),
        height: 1,
        ..area
    };

    PreviewAreas { main, logo, status }
}

/// Format file size for display.
fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

impl<'a> Widget for PreviewView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let show_logo = area.width >= MIN_LOGO_WIDTH;
        let areas = preview_areas(area, show_logo);
        let main_area = areas.main;

        // 3:1 split: image (left 75%) + info panel (right 25%)
        let info_w = (main_area.width / 4).max(20);
        let image_w = main_area.width.saturating_sub(info_w);
        let image_area = Rect {
            width: image_w,
            ..main_area
        };
        let info_area = Rect {
            x: main_area.x + image_w,
            width: info_w,
            ..main_area
        };

        // --- Image area ---
        if let Some(ref proto) = self.app.fullscreen_protocol {
            let proto_size = proto.size();
            let offset_x = image_area.width.saturating_sub(proto_size.width) / 2;
            let offset_y = image_area.height.saturating_sub(proto_size.height) / 2;
            let centered = Rect {
                x: image_area.x + offset_x,
                y: image_area.y + offset_y,
                width: proto_size.width.min(image_area.width),
                height: proto_size.height.min(image_area.height),
            };
            Image::new(proto).allow_clipping(true).render(centered, buf);
        }

        // --- Info panel ---
        if let Some(entry) = self.app.images.get(self.app.selected) {
            let lang = &self.app.lang;
            let ext = entry
                .path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("?")
                .to_uppercase();

            let mut lines: Vec<String> = Vec::new();

            // File name
            lines.push(format!("{}: {}", lang.label_file(), entry.filename));
            // Dimensions
            if let Some((w, h)) = self.app.fullscreen_dims {
                lines.push(format!("{}: {}×{}", lang.label_dims(), w, h));
            }
            // File size
            lines.push(format!(
                "{}: {}",
                lang.label_size(),
                format_size(entry.file_size)
            ));
            // Type
            lines.push(format!("{}: {}", lang.label_type(), ext));
            // Path
            let path_str = entry.path.to_string_lossy();
            let display_path = if path_str.len() > info_w as usize - lang.label_path().len() - 2 {
                format!(
                    "...{}",
                    &path_str[path_str.len().saturating_sub(info_w as usize - 6)..]
                )
            } else {
                path_str.into_owned()
            };
            lines.push(format!("{}: {}", lang.label_path(), display_path));

            let text_lines: Vec<Line> = lines
                .iter()
                .map(|l| Line::from(Span::styled(l.clone(), Style::default().fg(Color::White))))
                .collect();
            Paragraph::new(text_lines)
                .alignment(Alignment::Left)
                .render(info_area, buf);
        }

        // --- Logo ---
        if show_logo {
            render_logo(areas.logo, buf);
        }

        // --- Status bar ---
        if let Some(entry) = self.app.images.get(self.app.selected) {
            let status = if self.app.fullscreen_pending {
                self.app.lang.loading_text()
            } else {
                ""
            };
            let info = self.app.lang.preview_status(
                &entry.filename,
                self.app.selected + 1,
                self.app.images.len(),
                status,
            );
            let span = Span::styled(info, Style::default().fg(Color::White).bg(Color::DarkGray));
            Paragraph::new(span)
                .alignment(Alignment::Left)
                .render(areas.status, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_status_bar_shares_last_logo_row() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 30,
        };

        let areas = preview_areas(area, true);

        assert_eq!(areas.main.height, 30 - LOGO_HEIGHT);
        assert_eq!(areas.logo.y, 30 - LOGO_HEIGHT);
        assert_eq!(areas.logo.height, LOGO_HEIGHT);
        assert_eq!(areas.status.y, 29);
        assert_eq!(areas.status.height, 1);
    }
}
