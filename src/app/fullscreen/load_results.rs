use super::*;

impl App {
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

    pub(in crate::app) fn current_selected_stream_matches(
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

    pub(in crate::app) fn push_fullscreen_animation_frame(
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

    pub(in crate::app) fn finish_fullscreen_animation(&mut self, complete: bool) {
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
}
