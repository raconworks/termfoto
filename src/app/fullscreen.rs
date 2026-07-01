use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct AnimationCacheKey {
    image_key: ImageCacheKey,
    viewport_w: u16,
    viewport_h: u16,
    font_w: u16,
    font_h: u16,
}

#[derive(Clone)]
pub(super) struct CachedOriginal {
    image: Arc<image::RgbaImage>,
    bytes: usize,
}

#[derive(Clone)]
pub(super) struct CachedAnimation {
    content: AnimationContent,
    bytes: usize,
    dims: Option<(u32, u32)>,
}

fn rgba_bytes(image: &image::RgbaImage) -> usize {
    image.len()
}

pub(super) fn animation_cache_key(
    image_key: ImageCacheKey,
    viewport_w: u16,
    viewport_h: u16,
    font_size: FontSize,
) -> AnimationCacheKey {
    AnimationCacheKey {
        image_key,
        viewport_w: viewport_w.max(1),
        viewport_h: viewport_h.max(1),
        font_w: font_size.width.max(1),
        font_h: font_size.height.max(1),
    }
}

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

    pub(super) fn reset_fullscreen_content(&mut self) {
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

    pub(super) fn prepare_fullscreen_selection(&mut self) {
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

    pub(super) fn update_fullscreen_original_interest(&self) {
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

    pub(super) fn fullscreen_prefetch_keys(&self) -> Vec<ImageCacheKey> {
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

    pub(super) fn show_cached_fullscreen_content(&mut self, now: Instant) -> bool {
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

    pub(super) fn prefetch_fullscreen_neighbors(&mut self) {
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

    pub(super) fn current_fullscreen_viewport(&self) -> Option<(u16, u16)> {
        if self.fullscreen_image_w == 0 || self.fullscreen_image_h == 0 {
            None
        } else {
            Some((self.fullscreen_image_w, self.fullscreen_image_h))
        }
    }

    pub(super) fn current_original_load_size(&self, kind: OriginalLoadKind) -> Option<LoadSize> {
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

    pub(super) fn set_fullscreen_content_for_key(
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

    pub fn next_render_deadline(&self) -> Option<Instant> {
        self.render_settle_deadline
    }

    pub(super) fn mark_render_dirty(&mut self, reason: RenderDirtyReason) {
        self.zoom_dirty = true;
        self.render_dirty_reason = Some(reason);
        self.render_settle_deadline = None;
        self.render_generation = self.render_generation.wrapping_add(1);
    }

    /// Check for completed background image loads.
    /// In Browser mode, results go into protocol_cache.
    /// In Fullscreen mode, original results populate the decoded-original cache.
    pub fn collect_loads(&mut self) {
        let now = Instant::now();
        while let Ok(result) = self.load_rx.try_recv() {
            let LoadResult {
                key,
                size,
                generation,
                content,
                dims,
                ..
            } = result;
            if load_content_is_terminal(&content) {
                self.requested.remove(&(key.clone(), size.clone()));
            }
            if generation != self.directory_generation {
                continue;
            }
            match content {
                LoadContent::Skipped => continue,
                LoadContent::AnimationStarted { dims } => {
                    if !self.current_selected_stream_matches(&key, &size) {
                        continue;
                    }
                    self.set_fullscreen_content_for_key(
                        FullscreenContent::Animation(AnimationContent::empty()),
                        Some(dims),
                        now,
                        Some(key),
                    );
                    self.fullscreen_pending = true;
                }
                LoadContent::AnimationFrame { index, frame } => {
                    if !self.current_selected_stream_matches(&key, &size) {
                        continue;
                    }
                    self.push_fullscreen_animation_frame(index, frame, now);
                }
                LoadContent::AnimationFinished { complete } => {
                    if !self.current_selected_stream_matches(&key, &size) {
                        continue;
                    }
                    self.finish_fullscreen_animation(complete);
                }
                LoadContent::Original(content) => match content {
                    FullscreenContent::Static(sc) => {
                        self.insert_fullscreen_original(key.clone(), Arc::clone(&sc.original));
                        if self.state == AppState::Fullscreen
                            && self.current_selected_cache_key().as_ref() == Some(&key)
                        {
                            self.set_fullscreen_content_for_key(
                                FullscreenContent::Static(StaticContent {
                                    protocol: None,
                                    original: sc.original,
                                }),
                                dims,
                                now,
                                Some(key),
                            );
                            self.fullscreen_pending = true;
                        }
                    }
                    FullscreenContent::Animation(animation) => {
                        if self.state == AppState::Fullscreen
                            && self.current_selected_cache_key().as_ref() == Some(&key)
                        {
                            self.set_fullscreen_content_for_key(
                                FullscreenContent::Animation(animation),
                                dims,
                                now,
                                Some(key),
                            );
                            self.fullscreen_pending = false;
                        }
                    }
                },
                LoadContent::Thumbnail(proto) => {
                    if !matches!(size, LoadSize::Thumbnail { w, h } if w == self.thumb_w && h == self.thumb_h)
                    {
                        continue;
                    }
                    self.insert_cache(key, proto);
                }
            }
        }
    }

    pub(super) fn current_selected_stream_matches(
        &self,
        key: &ImageCacheKey,
        size: &LoadSize,
    ) -> bool {
        self.state == AppState::Fullscreen
            && self.current_selected_cache_key().as_ref() == Some(key)
            && matches!(
                size,
                LoadSize::Original {
                    w,
                    h,
                    kind: OriginalLoadKind::Selected,
                } if *w == self.fullscreen_image_w && *h == self.fullscreen_image_h
            )
    }

    pub(super) fn push_fullscreen_animation_frame(
        &mut self,
        index: usize,
        frame: AnimationFrame,
        now: Instant,
    ) {
        let font_size = self.picker.font_size();
        let Some(FullscreenContent::Animation(animation)) = self.fullscreen_content.as_mut() else {
            return;
        };

        if index == animation.frames.len() {
            animation.estimated_bytes = animation
                .estimated_bytes
                .saturating_add(animation_frame_estimated_bytes(&frame, font_size));
            animation.frames.push(frame);
        } else if let Some(existing) = animation.frames.get_mut(index) {
            *existing = frame;
        } else {
            return;
        }

        self.fullscreen_pending = false;
        if animation.frames.len() >= 2 && self.fullscreen_next_frame_at.is_none() {
            self.fullscreen_next_frame_at = animation.frames.first().map(|frame| now + frame.delay);
        }
    }

    pub(super) fn finish_fullscreen_animation(&mut self, complete: bool) {
        let Some(FullscreenContent::Animation(animation)) = self.fullscreen_content.as_mut() else {
            return;
        };
        animation.complete = complete;
        if !complete || animation.frames.len() < 2 {
            return;
        }
        let content = animation.clone();
        let dims = self.fullscreen_dims;
        if let Some(cache_key) = self.current_animation_cache_key() {
            self.insert_animation_cache(cache_key, content, dims);
        }
    }

    pub fn collect_render_results(&mut self) {
        while let Ok(result) = self.render_rx.try_recv() {
            self.apply_render_result(result);
        }
    }

    pub(super) fn apply_render_result(&mut self, result: RenderResult) {
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

    pub(super) fn apply_static_protocol(&mut self, key: RenderKey, protocol: Protocol) {
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

    pub(super) fn apply_cached_render(&mut self, quality: RenderQuality) -> bool {
        let Some(key) = self.current_render_key(quality) else {
            return false;
        };
        let Some(protocol) = self.fullscreen_render_cache.get(&key).cloned() else {
            return false;
        };
        self.apply_static_protocol(key, protocol);
        true
    }

    pub(super) fn submit_render_request(&mut self, quality: RenderQuality) -> bool {
        let Some(request) = self.current_render_request(quality) else {
            return false;
        };
        self.render_tx.send(request).is_ok()
    }

    pub(super) fn current_render_request(&self, quality: RenderQuality) -> Option<RenderRequest> {
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

    pub(super) fn current_render_key(&self, quality: RenderQuality) -> Option<RenderKey> {
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

    pub(super) fn render_key(
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

    pub(super) fn current_render_geometry(&self) -> Option<ZoomRenderGeometry> {
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

    pub(super) fn cached_fullscreen_original(
        &mut self,
        key: &ImageCacheKey,
    ) -> Option<Arc<image::RgbaImage>> {
        self.fullscreen_original_cache
            .get(key)
            .map(|entry| Arc::clone(&entry.image))
    }

    pub(super) fn insert_fullscreen_original(
        &mut self,
        key: ImageCacheKey,
        image: Arc<image::RgbaImage>,
    ) {
        let bytes = rgba_bytes(&image);
        if let Some(old) = self
            .fullscreen_original_cache
            .put(key, CachedOriginal { image, bytes })
        {
            self.fullscreen_original_cache_bytes = self
                .fullscreen_original_cache_bytes
                .saturating_sub(old.bytes);
        }
        self.fullscreen_original_cache_bytes =
            self.fullscreen_original_cache_bytes.saturating_add(bytes);
        self.evict_fullscreen_originals(self.current_selected_cache_key());
    }

    pub(super) fn evict_fullscreen_originals(&mut self, protect_key: Option<ImageCacheKey>) {
        let mut protected = Vec::new();
        while self.fullscreen_original_cache_bytes > FULLSCREEN_ORIGINAL_CACHE_BYTES
            && self.fullscreen_original_cache.len() + protected.len() > 1
        {
            let Some((key, entry)) = self.fullscreen_original_cache.pop_lru() else {
                break;
            };
            if Some(&key) == protect_key.as_ref() {
                protected.push((key, entry));
                continue;
            }
            self.fullscreen_original_cache_bytes = self
                .fullscreen_original_cache_bytes
                .saturating_sub(entry.bytes);
        }
        for (key, entry) in protected {
            self.fullscreen_original_cache.put(key, entry);
        }
    }

    pub(super) fn current_animation_cache_key(&self) -> Option<AnimationCacheKey> {
        self.current_animation_cache_key_for_image_key(self.current_selected_cache_key()?)
    }

    pub(super) fn current_animation_cache_key_for_image_key(
        &self,
        image_key: ImageCacheKey,
    ) -> Option<AnimationCacheKey> {
        let (viewport_w, viewport_h) = self.current_fullscreen_viewport()?;
        Some(animation_cache_key(
            image_key,
            viewport_w,
            viewport_h,
            self.picker.font_size(),
        ))
    }

    pub(super) fn animation_cache_key_for_size(
        &self,
        image_key: ImageCacheKey,
        w: u16,
        h: u16,
    ) -> AnimationCacheKey {
        animation_cache_key(image_key, w, h, self.picker.font_size())
    }

    pub(super) fn cached_animation(
        &mut self,
        key: &AnimationCacheKey,
    ) -> Option<(AnimationContent, Option<(u32, u32)>)> {
        self.animation_cache
            .get(key)
            .map(|entry| (entry.content.clone(), entry.dims))
    }

    pub(super) fn insert_animation_cache(
        &mut self,
        key: AnimationCacheKey,
        content: AnimationContent,
        dims: Option<(u32, u32)>,
    ) {
        if !content.complete || content.frames.len() < 2 {
            return;
        }
        let bytes = content.estimated_bytes;
        if bytes > ANIMATION_CACHE_BYTES {
            return;
        }
        if let Some(old) = self.animation_cache.put(
            key,
            CachedAnimation {
                content,
                bytes,
                dims,
            },
        ) {
            self.animation_cache_bytes = self.animation_cache_bytes.saturating_sub(old.bytes);
        }
        self.animation_cache_bytes = self.animation_cache_bytes.saturating_add(bytes);
        self.evict_animation_cache(self.current_animation_cache_key());
    }

    pub(super) fn evict_animation_cache(&mut self, protect_key: Option<AnimationCacheKey>) {
        let mut protected = Vec::new();
        while self.animation_cache_bytes > ANIMATION_CACHE_BYTES
            && self.animation_cache.len() + protected.len() > 1
        {
            let Some((key, entry)) = self.animation_cache.pop_lru() else {
                break;
            };
            if Some(&key) == protect_key.as_ref() {
                protected.push((key, entry));
                continue;
            }
            self.animation_cache_bytes = self.animation_cache_bytes.saturating_sub(entry.bytes);
        }
        for (key, entry) in protected {
            self.animation_cache.put(key, entry);
        }
    }

    pub(super) fn insert_cache(&mut self, key: ImageCacheKey, proto: Protocol) {
        self.protocol_cache.put(key, proto);
    }

    pub(super) fn remove_deleted_image_cache(&mut self, key: &ImageCacheKey) {
        self.protocol_cache.pop(key);
        self.requested
            .retain(|(requested_key, _)| requested_key != key);
        self.load_control
            .remove_interest_key(self.directory_generation, key);

        if let Some(old) = self.fullscreen_original_cache.pop(key) {
            self.fullscreen_original_cache_bytes = self
                .fullscreen_original_cache_bytes
                .saturating_sub(old.bytes);
        }

        let animation_keys: Vec<AnimationCacheKey> = self
            .animation_cache
            .iter()
            .filter(|(animation_key, _)| animation_key.image_key == *key)
            .map(|(animation_key, _)| animation_key.clone())
            .collect();
        for animation_key in animation_keys {
            if let Some(old) = self.animation_cache.pop(&animation_key) {
                self.animation_cache_bytes = self.animation_cache_bytes.saturating_sub(old.bytes);
            }
        }

        let render_keys: Vec<RenderKey> = self
            .fullscreen_render_cache
            .iter()
            .filter(|(render_key, _)| render_key.image_key == *key)
            .map(|(render_key, _)| render_key.clone())
            .collect();
        for render_key in render_keys {
            self.fullscreen_render_cache.pop(&render_key);
        }

        if self
            .fullscreen_protocol_key
            .as_ref()
            .is_some_and(|render_key| render_key.image_key == *key)
        {
            self.fullscreen_protocol_key = None;
        }
        if self.fullscreen_content_key.as_ref() == Some(key) {
            self.reset_fullscreen_content();
        }
    }

    pub fn request_load(&mut self, idx: usize, size: LoadSize) {
        let Some(entry) = self.images.get(idx) else {
            return;
        };
        let key = ImageCacheKey::from_entry(entry);
        if let LoadSize::Original { w, h, .. } = &size {
            if self.fullscreen_original_cache.contains(&key) {
                return;
            }
            let animation_key = self.animation_cache_key_for_size(key.clone(), *w, *h);
            if self.animation_cache.contains(&animation_key) {
                return;
            }
        }
        let requested_key = (key.clone(), size.clone());
        if self.requested.contains(&requested_key) {
            return;
        }
        self.requested.insert(requested_key);
        let _ = self.load_tx.send(LoadRequest {
            key,
            path: entry.path.clone(),
            size,
            generation: self.directory_generation,
        });
    }

    pub fn clear_protocol_cache(&mut self) {
        self.protocol_cache.clear();
        self.requested
            .retain(|(_, size)| !matches!(size, LoadSize::Thumbnail { .. }));
        self.cache_width = 0;
        self.load_control
            .clear_thumbnail_interest(self.directory_generation);
    }

    pub(super) fn clear_pending_original_requests(&mut self) {
        self.requested
            .retain(|(_, size)| !matches!(size, LoadSize::Original { .. }));
    }

    pub(crate) fn update_thumbnail_interest<I>(&self, w: u16, h: u16, slots: I)
    where
        I: IntoIterator<Item = usize>,
    {
        let keys = slots
            .into_iter()
            .filter_map(|slot| self.image_cache_key_for_slot(slot));
        self.load_control
            .update_thumbnail_interest(self.directory_generation, w, h, keys);
    }

    pub(crate) fn clear_thumbnail_interest(&self) {
        self.load_control
            .clear_thumbnail_interest(self.directory_generation);
    }

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

    pub(super) fn set_zoom(&mut self, zoom: f32) {
        self.zoom = normalized_zoom(zoom);
        self.clamp_pan();
        self.mark_render_dirty(RenderDirtyReason::Interaction);
    }

    /// 平移后钳制到缩放后整图超出视口的范围内
    pub(super) fn clamp_pan(&mut self) {
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

    pub(super) fn pan_step_x(&self) -> i16 {
        let Some(FullscreenContent::Static(sc)) = self.fullscreen_content.as_ref() else {
            return 1;
        };
        let fs = self.picker.font_size();
        let nat_w = sc.original.width().div_ceil(fs.width as u32) as f32;
        ((nat_w / self.zoom) * 0.1).max(1.0) as i16
    }

    pub(super) fn pan_step_y(&self) -> i16 {
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
