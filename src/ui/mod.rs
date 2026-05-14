pub mod grid;
pub mod preview;

use ratatui::Frame;
use ratatui_image::protocol::StatefulProtocol;
use crate::app::{App, AppState};
use crate::ui::grid::GridView;
use crate::ui::preview::PreviewView;

pub fn draw(
    frame: &mut Frame,
    app: &mut App,
    image_state: Option<&mut Box<dyn StatefulProtocol>>,
) {
    match app.state {
        AppState::Grid => {
            frame.render_widget(GridView { app }, frame.area());
        }
        AppState::Preview => {
            let widget = PreviewView { app, image_state };
            frame.render_widget(widget, frame.area());
        }
    }
}
