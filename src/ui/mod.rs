pub mod browser;
pub mod preview;

use ratatui::Frame;
use ratatui_image::protocol::Protocol;
use crate::app::{App, AppState};
use crate::ui::browser::BrowserView;
use crate::ui::preview::PreviewView;

pub fn draw(
    frame: &mut Frame,
    app: &mut App,
    cell_w: u16,
    cell_h: u16,
    protocol: Option<&Protocol>,
) {
    let area = frame.area();
    match app.state {
        AppState::Browser => {
            frame.render_widget(BrowserView { app, cell_w, cell_h }, area);
        }
        AppState::Fullscreen => {
            let widget = PreviewView { app, protocol };
            frame.render_widget(widget, area);
        }
    }
}
