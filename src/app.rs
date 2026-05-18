use crate::scanner::ImageEntry;
use image;

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Grid,
    Preview,
}

pub struct App {
    pub state: AppState,
    pub images: Vec<ImageEntry>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub grid_cols: usize,
    pub zoom_factor: f32,
}

pub const CELL_WIDTH: usize = 22;
pub const CELL_HEIGHT: usize = 14;

impl App {
    pub fn new(images: Vec<ImageEntry>, state: AppState) -> Self {
        Self {
            state,
            images,
            selected: 0,
            scroll_offset: 0,
            grid_cols: 1,
            zoom_factor: 1.0,
        }
    }

    /// 更新 grid_cols 和 scroll_offset。visible_rows 由调用方传入。
    pub fn update_layout(&mut self, terminal_width: u16, visible_rows: usize) {
        self.grid_cols = ((terminal_width as usize) / CELL_WIDTH).max(1);
        self.clamp_scroll(visible_rows.max(1));
    }

    pub fn navigate_left(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn navigate_right(&mut self) {
        if self.selected + 1 < self.images.len() {
            self.selected += 1;
        }
    }

    pub fn navigate_up(&mut self) {
        self.selected = self.selected.saturating_sub(self.grid_cols);
    }

    pub fn navigate_down(&mut self) {
        let next = self.selected + self.grid_cols;
        if next < self.images.len() {
            self.selected = next;
        }
    }

    pub fn navigate_home(&mut self) {
        self.selected = 0;
    }

    pub fn navigate_end(&mut self) {
        self.selected = self.images.len().saturating_sub(1);
    }

    pub fn navigate_page_down(&mut self, visible_rows: usize) {
        let step = visible_rows * self.grid_cols;
        let next = (self.selected + step).min(self.images.len().saturating_sub(1));
        self.selected = next;
    }

    pub fn navigate_page_up(&mut self, visible_rows: usize) {
        let step = visible_rows * self.grid_cols;
        self.selected = self.selected.saturating_sub(step);
    }

    pub fn clamp_scroll(&mut self, visible_rows: usize) {
        let selected_row = self.selected / self.grid_cols.max(1);
        if selected_row < self.scroll_offset {
            self.scroll_offset = selected_row;
        } else if selected_row >= self.scroll_offset + visible_rows {
            self.scroll_offset = selected_row + 1 - visible_rows;
        }
    }

    pub fn enter_preview(&mut self) {
        if !self.images.is_empty() {
            self.state = AppState::Preview;
        }
    }

    pub fn exit_preview(&mut self) {
        self.state = AppState::Grid;
    }

    pub fn preview_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn preview_next(&mut self) {
        if self.selected + 1 < self.images.len() {
            self.selected += 1;
        }
    }

    pub fn zoom_in(&mut self) {
        self.zoom_factor = ((self.zoom_factor + 0.1) * 10.0).round() / 10.0;
        self.zoom_factor = self.zoom_factor.min(5.0);
    }

    pub fn zoom_out(&mut self) {
        self.zoom_factor = ((self.zoom_factor - 0.1) * 10.0).round() / 10.0;
        self.zoom_factor = self.zoom_factor.max(0.1);
    }

    pub fn zoom_reset(&mut self) {
        self.zoom_factor = 1.0;
    }

    /// 生成当前视口范围内尚未加载的缩略图。
    /// thumb_w / thumb_h: 缩略图像素目标尺寸（由 UI 传入）。
    pub fn load_visible_thumbnails(&mut self, visible_rows: usize, thumb_w: u32, thumb_h: u32) {
        let start = self.scroll_offset * self.grid_cols;
        let end = (start + visible_rows * self.grid_cols).min(self.images.len());

        for entry in &mut self.images[start..end] {
            if entry.thumbnail.is_none() {
                if let Ok(img) = image::open(&entry.path) {
                    let thumb = img.resize(thumb_w, thumb_h, image::imageops::FilterType::Lanczos3);
                    entry.thumbnail = Some(thumb);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_app(count: usize, cols: usize) -> App {
        let images = (0..count)
            .map(|i| ImageEntry {
                path: PathBuf::from(format!("img{:03}.png", i)),
                filename: format!("img{:03}.png", i),
                thumbnail: None,
            })
            .collect();
        let mut app = App::new(images, AppState::Grid);
        app.grid_cols = cols;
        app
    }

    #[test]
    fn test_navigate_right_increments_selected() {
        let mut app = make_app(5, 3);
        app.navigate_right();
        assert_eq!(app.selected, 1);
    }

    #[test]
    fn test_navigate_right_clamps_at_last() {
        let mut app = make_app(3, 3);
        app.selected = 2;
        app.navigate_right();
        assert_eq!(app.selected, 2);
    }

    #[test]
    fn test_navigate_left_decrements_selected() {
        let mut app = make_app(5, 3);
        app.selected = 2;
        app.navigate_left();
        assert_eq!(app.selected, 1);
    }

    #[test]
    fn test_navigate_left_clamps_at_zero() {
        let mut app = make_app(5, 3);
        app.navigate_left();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_navigate_down_skips_one_row() {
        let mut app = make_app(9, 3);
        app.selected = 1;
        app.navigate_down();
        assert_eq!(app.selected, 4);
    }

    #[test]
    fn test_navigate_down_clamps_at_last_row() {
        let mut app = make_app(7, 3);
        app.selected = 4;
        app.navigate_down();
        assert_eq!(app.selected, 4);
    }

    #[test]
    fn test_navigate_up_skips_one_row() {
        let mut app = make_app(9, 3);
        app.selected = 4;
        app.navigate_up();
        assert_eq!(app.selected, 1);
    }

    #[test]
    fn test_navigate_up_clamps_at_zero() {
        let mut app = make_app(9, 3);
        app.selected = 2;
        app.navigate_up();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_navigate_home() {
        let mut app = make_app(5, 3);
        app.selected = 4;
        app.navigate_home();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_navigate_end() {
        let mut app = make_app(5, 3);
        app.navigate_end();
        assert_eq!(app.selected, 4);
    }

    #[test]
    fn test_navigate_page_down() {
        let mut app = make_app(20, 4);
        app.selected = 0;
        app.navigate_page_down(3);
        assert_eq!(app.selected, 12);
    }

    #[test]
    fn test_navigate_page_down_clamps_at_last() {
        let mut app = make_app(5, 3);
        app.selected = 0;
        app.navigate_page_down(10);
        assert_eq!(app.selected, 4);
    }

    #[test]
    fn test_navigate_page_up() {
        let mut app = make_app(20, 4);
        app.selected = 12;
        app.navigate_page_up(3);
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_clamp_scroll_scrolls_down_when_selected_below_viewport() {
        let mut app = make_app(30, 3);
        app.scroll_offset = 0;
        app.selected = 9;
        app.clamp_scroll(3);
        assert_eq!(app.scroll_offset, 1);
    }

    #[test]
    fn test_clamp_scroll_scrolls_up_when_selected_above_viewport() {
        let mut app = make_app(30, 3);
        app.scroll_offset = 5;
        app.selected = 3;
        app.clamp_scroll(3);
        assert_eq!(app.scroll_offset, 1);
    }

    #[test]
    fn test_enter_preview_changes_state() {
        let mut app = make_app(3, 3);
        app.enter_preview();
        assert_eq!(app.state, AppState::Preview);
    }

    #[test]
    fn test_enter_preview_noop_when_empty() {
        let mut app = make_app(0, 3);
        app.enter_preview();
        assert_eq!(app.state, AppState::Grid);
    }

    #[test]
    fn test_exit_preview_returns_to_grid() {
        let mut app = make_app(3, 3);
        app.state = AppState::Preview;
        app.exit_preview();
        assert_eq!(app.state, AppState::Grid);
    }

    #[test]
    fn test_preview_prev_and_next() {
        let mut app = make_app(3, 3);
        app.state = AppState::Preview;
        app.selected = 1;
        app.preview_prev();
        assert_eq!(app.selected, 0);
        app.preview_next();
        assert_eq!(app.selected, 1);
    }

    #[test]
    fn zoom_in_increments_by_0_1() {
        let mut app = make_app(1, 1);
        app.zoom_in();
        assert_eq!(app.zoom_factor, 1.1);
    }

    #[test]
    fn zoom_in_capped_at_5() {
        let mut app = make_app(1, 1);
        app.zoom_factor = 5.0;
        app.zoom_in();
        assert_eq!(app.zoom_factor, 5.0);
    }

    #[test]
    fn zoom_out_decrements_by_0_1() {
        let mut app = make_app(1, 1);
        app.zoom_out();
        assert_eq!(app.zoom_factor, 0.9);
    }

    #[test]
    fn zoom_out_floored_at_0_1() {
        let mut app = make_app(1, 1);
        app.zoom_factor = 0.1;
        app.zoom_out();
        assert_eq!(app.zoom_factor, 0.1);
    }

    #[test]
    fn zoom_reset_returns_to_1() {
        let mut app = make_app(1, 1);
        app.zoom_factor = 3.5;
        app.zoom_reset();
        assert_eq!(app.zoom_factor, 1.0);
    }

    #[test]
    fn zoom_no_float_drift() {
        let mut app = make_app(1, 1);
        app.zoom_factor = 0.1;
        app.zoom_in(); // 0.2
        app.zoom_in(); // 0.3
        app.zoom_in(); // 0.4
        assert_eq!(app.zoom_factor, 0.4);
    }

    #[test]
    fn load_visible_thumbnails_stores_at_expected_size() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create a real 200×150 PNG in a temp file
        let mut tmp = NamedTempFile::with_suffix(".png").unwrap();
        let img = image::DynamicImage::new_rgb8(200, 150);
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        tmp.write_all(buf.get_ref()).unwrap();
        tmp.flush().unwrap();

        let mut app = App::new(
            vec![crate::scanner::ImageEntry {
                path: tmp.path().to_path_buf(),
                filename: "test.png".to_string(),
                thumbnail: None,
            }],
            AppState::Grid,
        );
        app.grid_cols = 1;

        // thumb_w and thumb_h as computed in ui/mod.rs after the 4× increase:
        // thumb_w = (CELL_WIDTH - 2) * 4 = 20 * 4 = 80
        // thumb_h = (CELL_HEIGHT - 3) * 2 * 4 = 11 * 2 * 4 = 88
        let thumb_w = (CELL_WIDTH as u32).saturating_sub(2) * 4;
        let thumb_h = (CELL_HEIGHT as u32).saturating_sub(3) * 2 * 4;

        app.load_visible_thumbnails(1, thumb_w, thumb_h);

        let thumb = app.images[0].thumbnail.as_ref().expect("thumbnail should be loaded");
        assert!(thumb.width() <= thumb_w, "width should fit within thumb_w");
        assert!(thumb.height() <= thumb_h, "height should fit within thumb_h");
        assert!(thumb.width() > 18, "width should be larger than old 18px size");
        assert!(thumb.height() > 20, "height should be larger than old 20px size");
    }
}
