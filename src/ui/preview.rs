use crate::app::App;
use crate::ui::layout::three_panel_areas;
use crate::ui::{render_directory_context, render_info_panel, render_panel, render_prompt_lines};
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use ratatui_image::Image;

pub struct PreviewView<'a> {
    pub app: &'a mut App,
}

impl<'a> Widget for PreviewView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let areas = three_panel_areas(area);
        let context_inner = render_panel(areas.context, self.app.lang.title_context(), false, buf);
        let image_area = render_panel(areas.gallery, self.app.lang.title_gallery(), false, buf);
        let info_inner = render_panel(areas.info, self.app.lang.title_info(), false, buf);

        let context_entries = self.app.directory_context_for_browser();
        render_directory_context(
            context_inner,
            &context_entries,
            self.app.lang.empty_folder_context(),
            None,
            0,
            buf,
        );

        // Record viewport size for zoom calculations.
        self.app
            .set_fullscreen_viewport(image_area.width, image_area.height);

        // --- Image area ---
        if let Some(proto) = self.app.current_fullscreen_protocol() {
            let proto_size = proto.size();
            // Center protocol in image area
            let offset_x = (image_area.width.saturating_sub(proto_size.width) / 2) as i16;
            let offset_y = (image_area.height.saturating_sub(proto_size.height) / 2) as i16;
            let render_x = image_area.x.saturating_add_signed(offset_x);
            let render_y = image_area.y.saturating_add_signed(offset_y);
            let visible_w = proto_size.width.min(image_area.width);
            let visible_h = proto_size.height.min(image_area.height);

            let render_area = Rect {
                x: render_x,
                y: render_y,
                width: visible_w,
                height: visible_h,
            };
            // Protocol is already sized to viewport via regenerate_zoom_protocol(),
            // so no allow_clipping needed — just render centered.
            Image::new(proto).render(render_area, buf);
        }

        render_info_panel(
            info_inner,
            self.app.images.get(self.app.selected),
            self.app.fullscreen_dims,
            self.app,
            buf,
        );

        // --- Status bar ---
        if let Some(lines) = self.app.rename_prompt_lines() {
            render_prompt_lines(areas.prompt, &lines, buf);
        } else if let Some(entry) = self.app.images.get(self.app.selected) {
            let status = if self.app.fullscreen_pending {
                self.app.lang.loading_text().to_string()
            } else if (self.app.zoom - 1.0).abs() > f32::EPSILON {
                format!(" [{:.0}%]", self.app.zoom * 100.0)
            } else {
                String::new()
            };
            let lines = self.app.lang.fullscreen_prompt_lines(
                &entry.filename,
                self.app.selected + 1,
                self.app.images.len(),
                &status,
                self.app.is_favorites_view(),
            );
            render_prompt_lines(areas.prompt, &lines, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppStart, AppState, LoadRequest, LoadResult};
    use crate::lang::Lang;
    use crate::scanner::ImageEntry;
    use ratatui_image::picker::Picker;
    use std::fs;
    use tempfile::{tempdir, TempDir};

    fn render_test_app() -> (TempDir, App) {
        let dir = tempdir().unwrap();
        let photos = dir.path().join("photos");
        fs::create_dir(&photos).unwrap();
        let image_path = photos.join("sample.png");
        fs::write(&image_path, b"sample").unwrap();

        let images = vec![ImageEntry {
            path: image_path,
            filename: "sample.png".to_string(),
            file_size: 6,
            modified_at: None,
        }];
        let (tx, _rx) = std::sync::mpsc::channel::<LoadRequest>();
        let (_tx2, rx2) = std::sync::mpsc::channel::<LoadResult>();
        (
            dir,
            App::new(
                AppStart {
                    images,
                    image_dir: photos,
                    state: AppState::Fullscreen,
                    selected: 0,
                },
                tx,
                rx2,
                Lang::En,
                Picker::halfblocks(),
            ),
        )
    }

    fn buffer_text(buf: &Buffer) -> String {
        buf.content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    }

    #[test]
    fn preview_prompt_uses_bottom_three_rows() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 30,
        };

        let areas = three_panel_areas(area);

        assert_eq!(areas.gallery.height, 27);
        assert_eq!(areas.prompt.y, 27);
        assert_eq!(areas.prompt.height, 3);
    }

    #[test]
    fn preview_render_includes_three_panel_titles_and_browser_context() {
        let (_dir, mut app) = render_test_app();
        let area = Rect::new(0, 0, 100, 20);
        let mut buf = Buffer::empty(area);

        PreviewView { app: &mut app }.render(area, &mut buf);

        let text = buffer_text(&buf);
        assert!(text.contains("Context"));
        assert!(text.contains("Gallery"));
        assert!(text.contains("Info"));
        assert!(text.contains("> photos/"));
    }
}
