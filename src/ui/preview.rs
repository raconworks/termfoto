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
        let logo_h = if show_logo { LOGO_HEIGHT } else { 0 };
        let status_h = 1u16;

        // Reserve bottom area for logo + status bar
        let main_h = area.height.saturating_sub(logo_h + status_h);
        let main_area = Rect {
            height: main_h,
            ..area
        };

        let logo_area = Rect {
            y: area.y + main_h,
            height: logo_h,
            ..area
        };
        let status_area = Rect {
            y: area.y + main_h + logo_h,
            height: status_h,
            ..area
        };

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
            render_logo(logo_area, buf);
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
                .render(status_area, buf);
        }
    }
}
