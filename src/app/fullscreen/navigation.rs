use super::*;

impl App {
    pub fn enter_fullscreen(&mut self) {
        if !self.images.is_empty() {
            self.state = AppState::Fullscreen;
            self.reset_fullscreen_content();
            self.prepare_fullscreen_selection();
        }
    }

    pub fn exit_fullscreen(&mut self) {
        self.state = AppState::Browser;
        self.reset_fullscreen_content();
        self.zoom = 1.0;
        self.pan_x = 0;
        self.pan_y = 0;
        self.fullscreen_image_w = 0;
        self.fullscreen_image_h = 0;
        self.fullscreen_pending = false;
        self.render_dirty_reason = None;
        self.render_settle_deadline = None;
        self.load_control
            .clear_original_interest(self.directory_generation);
        self.clear_pending_original_requests();
    }

    pub fn fullscreen_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.reset_fullscreen_content();
            self.zoom = 1.0;
            self.pan_x = 0;
            self.pan_y = 0;
            self.prepare_fullscreen_selection();
        }
    }

    pub fn fullscreen_next(&mut self) {
        if self.selected + 1 < self.images.len() {
            self.selected += 1;
            self.reset_fullscreen_content();
            self.zoom = 1.0;
            self.pan_x = 0;
            self.pan_y = 0;
            self.prepare_fullscreen_selection();
        }
    }

    pub(in crate::app) fn reset_fullscreen_content(&mut self) {
        self.fullscreen_content = None;
        self.fullscreen_content_key = None;
        self.fullscreen_frame_idx = 0;
        self.fullscreen_next_frame_at = None;
        self.fullscreen_dims = None;
        self.zoom_dirty = false;
        self.render_dirty_reason = None;
        self.render_settle_deadline = None;
        self.fullscreen_protocol_key = None;
        self.render_generation = self.render_generation.wrapping_add(1);
        self.clear_pending_original_requests();
    }

    pub(in crate::app) fn prepare_fullscreen_selection(&mut self) {
        self.update_fullscreen_original_interest();
        if self.show_cached_fullscreen_content(Instant::now()) {
            self.fullscreen_pending = self.current_fullscreen_protocol().is_none();
        } else {
            self.fullscreen_pending = true;
            if let Some(size) = self.current_original_load_size(OriginalLoadKind::Selected) {
                self.request_load(self.selected, size);
            }
        }
        self.prefetch_fullscreen_neighbors();
    }

    pub(in crate::app) fn update_fullscreen_original_interest(&self) {
        let Some((w, h)) = self.current_fullscreen_viewport() else {
            self.load_control
                .clear_original_interest(self.directory_generation);
            return;
        };
        self.load_control.update_original_interest(
            self.directory_generation,
            w,
            h,
            self.current_selected_cache_key(),
            self.fullscreen_prefetch_keys(),
        );
    }

    pub(in crate::app) fn fullscreen_prefetch_keys(&self) -> Vec<ImageCacheKey> {
        if self.images.is_empty() || self.selected >= self.images.len() {
            return Vec::new();
        }

        let mut keys = Vec::with_capacity(2);
        if self.selected > 0 {
            if let Some(key) = self.image_cache_key_for_slot(self.selected - 1) {
                keys.push(key);
            }
        }
        if self.selected + 1 < self.images.len() {
            if let Some(key) = self.image_cache_key_for_slot(self.selected + 1) {
                keys.push(key);
            }
        }
        keys
    }

    pub(in crate::app) fn show_cached_fullscreen_content(&mut self, now: Instant) -> bool {
        let Some(key) = self.current_selected_cache_key() else {
            return false;
        };
        if let Some(image) = self.cached_fullscreen_original(&key) {
            let dims = Some((image.width(), image.height()));
            self.set_fullscreen_content_for_key(
                FullscreenContent::Static(StaticContent {
                    protocol: None,
                    original: image,
                }),
                dims,
                now,
                Some(key),
            );
            return true;
        }

        let Some(cache_key) = self.current_animation_cache_key_for_image_key(key.clone()) else {
            return false;
        };
        let Some((content, dims)) = self.cached_animation(&cache_key) else {
            return false;
        };
        self.set_fullscreen_content_for_key(
            FullscreenContent::Animation(content),
            dims,
            now,
            Some(key),
        );
        self.fullscreen_pending = false;
        true
    }

    pub(in crate::app) fn prefetch_fullscreen_neighbors(&mut self) {
        if self.images.is_empty() {
            return;
        }
        let Some(size) = self.current_original_load_size(OriginalLoadKind::Prefetch) else {
            return;
        };
        if self.selected > 0 {
            self.request_load(self.selected - 1, size.clone());
        }
        if self.selected + 1 < self.images.len() {
            self.request_load(self.selected + 1, size);
        }
    }

    pub(in crate::app) fn current_fullscreen_viewport(&self) -> Option<(u16, u16)> {
        if self.fullscreen_image_w == 0 || self.fullscreen_image_h == 0 {
            None
        } else {
            Some((self.fullscreen_image_w, self.fullscreen_image_h))
        }
    }

    pub(in crate::app) fn current_original_load_size(
        &self,
        kind: OriginalLoadKind,
    ) -> Option<LoadSize> {
        let (w, h) = self.current_fullscreen_viewport()?;
        Some(LoadSize::Original { w, h, kind })
    }

    #[cfg(test)]
    pub fn set_fullscreen_content(
        &mut self,
        content: FullscreenContent,
        dims: Option<(u32, u32)>,
        now: Instant,
    ) {
        let key = self.current_selected_cache_key();
        self.set_fullscreen_content_for_key(content, dims, now, key);
    }

    pub(in crate::app) fn set_fullscreen_content_for_key(
        &mut self,
        content: FullscreenContent,
        dims: Option<(u32, u32)>,
        now: Instant,
        key: Option<ImageCacheKey>,
    ) {
        self.fullscreen_frame_idx = 0;
        self.zoom = 1.0;
        self.pan_x = 0;
        self.pan_y = 0;
        let is_static = matches!(&content, FullscreenContent::Static(_));
        self.fullscreen_next_frame_at = match &content {
            FullscreenContent::Animation(animation) if animation.frames.len() >= 2 => {
                animation.frames.first().map(|frame| now + frame.delay)
            }
            FullscreenContent::Animation(_) => None,
            FullscreenContent::Static(_) => None,
        };
        self.fullscreen_content = Some(content);
        self.fullscreen_content_key = key;
        self.fullscreen_dims = dims;
        self.fullscreen_protocol_key = None;
        if is_static {
            self.mark_render_dirty(RenderDirtyReason::ContentOrViewport);
        } else {
            self.zoom_dirty = false;
            self.render_dirty_reason = None;
            self.render_settle_deadline = None;
        }
    }

    pub fn set_fullscreen_viewport(&mut self, width: u16, height: u16) {
        let changed = self.fullscreen_image_w != width || self.fullscreen_image_h != height;
        self.fullscreen_image_w = width;
        self.fullscreen_image_h = height;
        if changed && self.state == AppState::Fullscreen {
            self.update_fullscreen_original_interest();
            match self.fullscreen_content {
                Some(FullscreenContent::Static(_)) => {
                    self.clamp_pan();
                    self.mark_render_dirty(RenderDirtyReason::ContentOrViewport);
                }
                Some(FullscreenContent::Animation(_)) => {
                    self.reset_fullscreen_content();
                    self.prepare_fullscreen_selection();
                }
                None => self.prepare_fullscreen_selection(),
            }
        }
    }

    pub fn current_fullscreen_protocol(&self) -> Option<&Protocol> {
        match self.fullscreen_content.as_ref()? {
            FullscreenContent::Static(sc) => sc.protocol.as_ref(),
            FullscreenContent::Animation(animation) => animation
                .frames
                .get(self.fullscreen_frame_idx)
                .or_else(|| animation.frames.first())
                .map(|frame| &frame.protocol),
        }
    }

    #[cfg(test)]
    pub fn fullscreen_frame_index(&self) -> usize {
        self.fullscreen_frame_idx
    }

    pub fn next_animation_deadline(&self) -> Option<Instant> {
        if self.state == AppState::Fullscreen {
            self.fullscreen_next_frame_at
        } else {
            None
        }
    }

    pub fn advance_animation(&mut self, now: Instant) -> bool {
        if self.state != AppState::Fullscreen {
            return false;
        }

        let Some(FullscreenContent::Animation(animation)) = self.fullscreen_content.as_ref() else {
            return false;
        };
        let frames = &animation.frames;
        if frames.len() < 2 {
            return false;
        }

        let Some(next_at) = self.fullscreen_next_frame_at else {
            return false;
        };
        if now < next_at {
            return false;
        }

        self.fullscreen_frame_idx = (self.fullscreen_frame_idx + 1) % frames.len();
        self.fullscreen_next_frame_at = Some(now + frames[self.fullscreen_frame_idx].delay);
        true
    }
}
