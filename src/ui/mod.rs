pub mod browser;
pub mod preview;
pub mod search;

use crate::app::{App, AppState, LOGO_HEIGHT};
use crate::ui::browser::BrowserView;
use crate::ui::preview::PreviewView;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Span,
    Frame,
};

const LOGO_LINES: [&str; LOGO_HEIGHT as usize] = [
    "▀█▀ █▀▀ █▀█ █▀▄▀█ █▀▀ █▀█ ▀█▀ █▀█",
    " █  █▀  █▀▄ █ ▀ █ █▀  █▄█  █  █▄█",
    " ▀  ▀▀▀ ▀ ▀ ▀   ▀ ▀   ▀▀▀  ▀  ▀▀▀",
];

const LOGO_COLORS: [Color; LOGO_HEIGHT as usize] = [
    Color::Rgb(255, 0, 0),
    Color::Rgb(0, 255, 0),
    Color::Rgb(127, 0, 255),
];

pub fn render_logo(area: Rect, buf: &mut Buffer) {
    let max_w = LOGO_LINES
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);
    let logo_w = max_w.min(area.width as usize);
    let offset_x = area.x + area.width.saturating_sub(logo_w as u16);

    for (i, line) in LOGO_LINES.iter().enumerate() {
        let trimmed: String = line.chars().take(logo_w).collect();
        let style = Style::default().fg(LOGO_COLORS[i]);
        buf.set_span(
            offset_x,
            area.y + i as u16,
            &Span::styled(trimmed, style),
            logo_w as u16,
        );
    }
}

pub fn draw(frame: &mut Frame, app: &mut App, cell_w: u16, cell_h: u16) {
    let area = frame.area();
    match app.state {
        AppState::Browser => {
            frame.render_widget(
                BrowserView {
                    app,
                    cell_w,
                    cell_h,
                },
                area,
            );
        }
        AppState::Fullscreen => {
            frame.render_widget(PreviewView { app }, area);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_logo_uses_three_rows() {
        assert_eq!(LOGO_HEIGHT, 3);
        assert_eq!(LOGO_LINES.len(), LOGO_HEIGHT as usize);
        assert_eq!(LOGO_COLORS.len(), LOGO_HEIGHT as usize);
    }
}
