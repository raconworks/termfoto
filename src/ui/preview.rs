use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};
use ratatui_image::{protocol::StatefulProtocol, Resize, StatefulImage};
use crate::app::App;

pub struct PreviewView<'a> {
    pub app: &'a App,
    pub image_state: Option<&'a mut Box<dyn StatefulProtocol>>,
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

        if let Some(state) = self.image_state {
            let widget = StatefulImage::new(None).resize(Resize::Fit(None));
            StatefulWidget::render(widget, image_area, buf, state);
        } else {
            Block::default()
                .borders(Borders::NONE)
                .render(image_area, buf);
        }

        if let Some(entry) = self.app.images.get(self.app.selected) {
            let info = format!(
                " {} [{}/{}]  ← → 切换  Esc/q 返回",
                entry.filename,
                self.app.selected + 1,
                self.app.images.len()
            );
            let span = Span::styled(info, Style::default().fg(Color::White).bg(Color::DarkGray));
            Paragraph::new(span)
                .alignment(Alignment::Left)
                .render(status_area, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, AppState};
    use crate::scanner::ImageEntry;
    use std::path::PathBuf;
    use ratatui::{backend::TestBackend, Terminal};

    fn make_app() -> App {
        let images = vec![ImageEntry {
            path: PathBuf::from("test.png"),
            filename: "test.png".to_string(),
            thumbnail: None,
        }];
        let mut app = App::new(images, AppState::Preview);
        app.grid_cols = 1;
        app
    }

    #[test]
    fn test_preview_renders_without_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = make_app();

        terminal
            .draw(|f| {
                let widget = PreviewView { app: &app, image_state: None };
                f.render_widget(widget, f.area());
            })
            .unwrap();
    }

    #[test]
    fn test_preview_status_bar_shows_filename() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = make_app();

        terminal
            .draw(|f| {
                let widget = PreviewView { app: &app, image_state: None };
                f.render_widget(widget, f.area());
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        let last_row: String = (0..80u16)
            .map(|x| buf.cell((x, 23)).map(|c| c.symbol().chars().next().unwrap_or(' ')).unwrap_or(' '))
            .collect();
        assert!(last_row.contains("test.png"), "Status bar should contain filename, got: {last_row:?}");
    }
}
