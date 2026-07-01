use super::*;

impl App {
    /// 放大，上限 ZOOM_MAX
    pub fn zoom_in(&mut self) {
        if self.state != AppState::Fullscreen {
            return;
        }
        self.set_zoom((self.zoom + ZOOM_STEP).min(ZOOM_MAX));
    }

    /// 缩小，下限 ZOOM_MIN
    pub fn zoom_out(&mut self) {
        if self.state != AppState::Fullscreen {
            return;
        }
        self.set_zoom((self.zoom - ZOOM_STEP).max(ZOOM_MIN));
    }

    /// 重置缩放与平移
    pub fn zoom_reset(&mut self) {
        if self.state != AppState::Fullscreen {
            return;
        }
        self.zoom = 1.0;
        self.pan_x = 0;
        self.pan_y = 0;
        self.mark_render_dirty(RenderDirtyReason::Interaction);
    }

    pub(in crate::app) fn set_zoom(&mut self, zoom: f32) {
        self.zoom = normalized_zoom(zoom);
        self.clamp_pan();
        self.mark_render_dirty(RenderDirtyReason::Interaction);
    }

    /// 平移后钳制到缩放后整图超出视口的范围内
    pub(in crate::app) fn clamp_pan(&mut self) {
        let Some(FullscreenContent::Static(sc)) = self.fullscreen_content.as_ref() else {
            return;
        };
        if self.fullscreen_image_w == 0 || self.fullscreen_image_h == 0 {
            self.pan_x = 0;
            self.pan_y = 0;
            return;
        }
        let fs = self.picker.font_size();
        let viewport_px_w = (self.fullscreen_image_w as u32).saturating_mul(fs.width as u32);
        let viewport_px_h = (self.fullscreen_image_h as u32).saturating_mul(fs.height as u32);
        let display = zoom_display_geometry(
            sc.original.width(),
            sc.original.height(),
            viewport_px_w,
            viewport_px_h,
            self.zoom,
        );
        let max_cell_x = max_pan_cells(display.display_px_w, viewport_px_w, fs.width);
        let max_cell_y = max_pan_cells(display.display_px_h, viewport_px_h, fs.height);
        self.pan_x = self.pan_x.clamp(-max_cell_x, max_cell_x);
        self.pan_y = self.pan_y.clamp(-max_cell_y, max_cell_y);
    }

    pub(in crate::app) fn pan_step_x(&self) -> i16 {
        let Some(FullscreenContent::Static(sc)) = self.fullscreen_content.as_ref() else {
            return 1;
        };
        let fs = self.picker.font_size();
        let nat_w = sc.original.width().div_ceil(fs.width as u32) as f32;
        ((nat_w / self.zoom) * 0.1).max(1.0) as i16
    }

    pub(in crate::app) fn pan_step_y(&self) -> i16 {
        let Some(FullscreenContent::Static(sc)) = self.fullscreen_content.as_ref() else {
            return 1;
        };
        let fs = self.picker.font_size();
        let nat_h = sc.original.height().div_ceil(fs.height as u32) as f32;
        ((nat_h / self.zoom) * 0.1).max(1.0) as i16
    }

    pub fn pan_left(&mut self) {
        self.pan_x -= self.pan_step_x();
        self.clamp_pan();
        self.mark_render_dirty(RenderDirtyReason::Interaction);
    }
    pub fn pan_right(&mut self) {
        self.pan_x += self.pan_step_x();
        self.clamp_pan();
        self.mark_render_dirty(RenderDirtyReason::Interaction);
    }
    pub fn pan_up(&mut self) {
        self.pan_y -= self.pan_step_y();
        self.clamp_pan();
        self.mark_render_dirty(RenderDirtyReason::Interaction);
    }
    pub fn pan_down(&mut self) {
        self.pan_y += self.pan_step_y();
        self.clamp_pan();
        self.mark_render_dirty(RenderDirtyReason::Interaction);
    }
}
