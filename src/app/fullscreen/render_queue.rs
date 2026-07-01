use super::*;

impl App {
    pub fn next_render_deadline(&self) -> Option<Instant> {
        self.render_settle_deadline
    }

    pub(in crate::app) fn mark_render_dirty(&mut self, reason: RenderDirtyReason) {
        self.zoom_dirty = true;
        self.render_dirty_reason = Some(reason);
        self.render_settle_deadline = None;
        self.render_generation = self.render_generation.wrapping_add(1);
    }

    pub fn collect_render_results(&mut self) {
        while let Ok(result) = self.render_rx.try_recv() {
            self.apply_render_result(result);
        }
    }

    pub(in crate::app) fn apply_render_result(&mut self, result: RenderResult) {
        if self.state != AppState::Fullscreen
            || self.current_selected_cache_key().as_ref() != Some(&result.image_key)
            || self.render_generation != result.generation
        {
            return;
        }

        let Some(current_key) = self.current_render_key(result.key.quality) else {
            return;
        };
        if current_key != result.key {
            return;
        }
        if result.key.quality == RenderQuality::Interactive
            && self.fullscreen_protocol_key.as_ref().is_some_and(|key| {
                key.quality == RenderQuality::Final && key.same_view(&result.key)
            })
        {
            return;
        }

        self.fullscreen_render_cache
            .put(result.key.clone(), result.protocol.clone());
        self.apply_static_protocol(result.key, result.protocol);
    }

    pub(in crate::app) fn apply_static_protocol(&mut self, key: RenderKey, protocol: Protocol) {
        let Some(FullscreenContent::Static(sc)) = self.fullscreen_content.as_mut() else {
            return;
        };
        sc.protocol = Some(protocol);
        self.fullscreen_protocol_key = Some(key);
        self.fullscreen_pending = false;
    }

    pub fn drive_render_queue(&mut self, now: Instant) {
        if self.state != AppState::Fullscreen {
            return;
        }

        let Some(geometry) = self.current_render_geometry() else {
            return;
        };

        if self.zoom_dirty {
            let reason = self
                .render_dirty_reason
                .unwrap_or(RenderDirtyReason::ContentOrViewport);
            let quality = match reason {
                RenderDirtyReason::Interaction => RenderQuality::Interactive,
                RenderDirtyReason::ContentOrViewport => {
                    if u64::from(geometry.target_px_w) * u64::from(geometry.target_px_h)
                        <= DIRECT_FINAL_RENDER_PIXELS
                    {
                        RenderQuality::Final
                    } else {
                        RenderQuality::Interactive
                    }
                }
            };
            if !self.apply_cached_render(quality) {
                self.submit_render_request(quality);
            }
            self.zoom_dirty = false;
            self.render_dirty_reason = None;
            self.render_settle_deadline =
                (quality == RenderQuality::Interactive).then_some(now + INTERACTIVE_SETTLE_DELAY);
            return;
        }

        if self
            .render_settle_deadline
            .is_some_and(|deadline| now >= deadline)
        {
            self.render_settle_deadline = None;
            if !self.apply_cached_render(RenderQuality::Final) {
                self.submit_render_request(RenderQuality::Final);
            }
        }
    }

    /// Compatibility wrapper for tests that still exercise the dirty flag semantics.
    #[cfg(test)]
    pub fn regenerate_if_dirty(&mut self) {
        self.drive_render_queue(Instant::now());
    }

    pub(in crate::app) fn apply_cached_render(&mut self, quality: RenderQuality) -> bool {
        let Some(key) = self.current_render_key(quality) else {
            return false;
        };
        let Some(protocol) = self.fullscreen_render_cache.get(&key).cloned() else {
            return false;
        };
        self.apply_static_protocol(key, protocol);
        true
    }

    pub(in crate::app) fn submit_render_request(&mut self, quality: RenderQuality) -> bool {
        let Some(request) = self.current_render_request(quality) else {
            return false;
        };
        self.render_tx.send(request).is_ok()
    }

    pub(in crate::app) fn current_render_request(
        &self,
        quality: RenderQuality,
    ) -> Option<RenderRequest> {
        let FullscreenContent::Static(sc) = self.fullscreen_content.as_ref()? else {
            return None;
        };
        if self.fullscreen_image_w == 0 || self.fullscreen_image_h == 0 {
            return None;
        }
        let font_size = self.picker.font_size();
        let key = self.current_render_key(quality)?;
        Some(RenderRequest {
            image_key: key.image_key.clone(),
            image: Arc::clone(&sc.original),
            viewport: Size::new(
                self.fullscreen_image_w.max(1),
                self.fullscreen_image_h.max(1),
            ),
            font_size,
            zoom: self.zoom,
            pan_x: self.pan_x,
            pan_y: self.pan_y,
            key,
            generation: self.render_generation,
        })
    }

    pub(in crate::app) fn current_render_key(&self, quality: RenderQuality) -> Option<RenderKey> {
        if !matches!(
            self.fullscreen_content.as_ref()?,
            FullscreenContent::Static(_)
        ) || self.fullscreen_image_w == 0
            || self.fullscreen_image_h == 0
        {
            return None;
        }
        Some(self.render_key(
            quality,
            self.picker.font_size(),
            self.current_selected_cache_key()?,
        ))
    }

    pub(in crate::app) fn render_key(
        &self,
        quality: RenderQuality,
        font_size: FontSize,
        image_key: ImageCacheKey,
    ) -> RenderKey {
        RenderKey {
            image_key,
            viewport_w: self.fullscreen_image_w.max(1),
            viewport_h: self.fullscreen_image_h.max(1),
            font_w: font_size.width.max(1),
            font_h: font_size.height.max(1),
            zoom_percent: zoom_percent(self.zoom),
            pan_x: self.pan_x,
            pan_y: self.pan_y,
            quality,
        }
    }

    pub(in crate::app) fn current_render_geometry(&self) -> Option<ZoomRenderGeometry> {
        let FullscreenContent::Static(sc) = self.fullscreen_content.as_ref()? else {
            return None;
        };
        if self.fullscreen_image_w == 0 || self.fullscreen_image_h == 0 {
            return None;
        }
        let fs = self.picker.font_size();
        let vp_px_w = (self.fullscreen_image_w as u32).saturating_mul(fs.width as u32);
        let vp_px_h = (self.fullscreen_image_h as u32).saturating_mul(fs.height as u32);
        let pan_px_x = (self.pan_x as f32 * fs.width as f32) as i32;
        let pan_px_y = (self.pan_y as f32 * fs.height as f32) as i32;
        Some(zoom_render_geometry(
            sc.original.width(),
            sc.original.height(),
            vp_px_w,
            vp_px_h,
            self.zoom,
            pan_px_x,
            pan_px_y,
        ))
    }
}
