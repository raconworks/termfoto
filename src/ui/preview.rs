use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, Widget},
};
use ratatui_image::{protocol::Protocol, Image};
use crate::app::App;

pub struct PreviewView<'a> {
    pub app: &'a App,
    pub protocol: Option<&'a Protocol>,
}

impl<'a> Widget for PreviewView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let status_height = 1u16;
        let image_area = Rect {
            height: area.height.saturating_sub(status_height),
            ..area
        };
        let status_area = Rect {
            y: area.y + image_area.height,
            height: status_height,
            ..area
        };

        if let Some(proto) = self.protocol {
            let proto_size = proto.size();
            // Center the image in the available area
            let offset_x = image_area
                .width
                .saturating_sub(proto_size.width)
                / 2;
            let offset_y = image_area
                .height
                .saturating_sub(proto_size.height)
                / 2;
            let centered = Rect {
                x: image_area.x + offset_x,
                y: image_area.y + offset_y,
                width: proto_size.width.min(image_area.width),
                height: proto_size.height.min(image_area.height),
            };
            Image::new(proto).allow_clipping(true).render(centered, buf);
        } else {
            Block::default()
                .borders(Borders::NONE)
                .render(image_area, buf);
        }

        if let Some(entry) = self.app.images.get(self.app.selected) {
            let status = if self.app.fullscreen_pending {
                " ⏳ 加载中..."
            } else {
                ""
            };
            let info = format!(
                " {} [{}/{}]  原图尺寸  ← → 切换  Enter/Esc/q 返回{}",
                entry.filename,
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
