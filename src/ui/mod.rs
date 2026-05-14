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
    let area = frame.area();
    match app.state {
        AppState::Grid => {
            let visible_rows = (area.height / crate::app::CELL_HEIGHT as u16) as usize;
            let thumb_w = (crate::app::CELL_WIDTH as u32).saturating_sub(4);
            let thumb_h = (crate::app::CELL_HEIGHT as u32).saturating_sub(4) * 2;
            app.load_visible_thumbnails(visible_rows, thumb_w, thumb_h);
            frame.render_widget(GridView { app }, area);
        }
        AppState::Preview => {
            let widget = PreviewView { app, image_state };
            frame.render_widget(widget, area);
        }
    }
}
