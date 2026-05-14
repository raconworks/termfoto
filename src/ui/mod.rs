pub mod grid;
pub mod preview;

use ratatui::Frame;
use crate::app::{App, AppState};

pub fn draw(frame: &mut Frame, app: &mut App) {
    match app.state {
        AppState::Grid => {
            let widget = grid::GridView { app };
            frame.render_widget(widget, frame.area());
        }
        AppState::Preview => {
            // preview widget implemented in Task 5
        }
    }
}
