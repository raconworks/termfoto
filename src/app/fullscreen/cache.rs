use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(in crate::app) struct AnimationCacheKey {
    image_key: ImageCacheKey,
    viewport_w: u16,
    viewport_h: u16,
    font_w: u16,
    font_h: u16,
}

#[derive(Clone)]
pub(in crate::app) struct CachedOriginal {
    image: Arc<image::RgbaImage>,
    bytes: usize,
}

#[derive(Clone)]
pub(in crate::app) struct CachedAnimation {
    content: AnimationContent,
    bytes: usize,
    dims: Option<(u32, u32)>,
}

fn rgba_bytes(image: &image::RgbaImage) -> usize {
    image.len()
}

pub(in crate::app) fn animation_cache_key(
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
    pub(in crate::app) fn cached_fullscreen_original(
        &mut self,
        key: &ImageCacheKey,
    ) -> Option<Arc<image::RgbaImage>> {
        self.fullscreen_original_cache
            .get(key)
            .map(|entry| Arc::clone(&entry.image))
    }

    pub(in crate::app) fn insert_fullscreen_original(
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

    pub(in crate::app) fn evict_fullscreen_originals(
        &mut self,
        protect_key: Option<ImageCacheKey>,
    ) {
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

    pub(in crate::app) fn current_animation_cache_key(&self) -> Option<AnimationCacheKey> {
        self.current_animation_cache_key_for_image_key(self.current_selected_cache_key()?)
    }

    pub(in crate::app) fn current_animation_cache_key_for_image_key(
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

    pub(in crate::app) fn animation_cache_key_for_size(
        &self,
        image_key: ImageCacheKey,
        w: u16,
        h: u16,
    ) -> AnimationCacheKey {
        animation_cache_key(image_key, w, h, self.picker.font_size())
    }

    pub(in crate::app) fn cached_animation(
        &mut self,
        key: &AnimationCacheKey,
    ) -> Option<(AnimationContent, Option<(u32, u32)>)> {
        self.animation_cache
            .get(key)
            .map(|entry| (entry.content.clone(), entry.dims))
    }

    pub(in crate::app) fn insert_animation_cache(
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

    pub(in crate::app) fn evict_animation_cache(&mut self, protect_key: Option<AnimationCacheKey>) {
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

    pub(in crate::app) fn insert_cache(&mut self, key: ImageCacheKey, proto: Protocol) {
        self.protocol_cache.put(key, proto);
    }

    pub(in crate::app) fn remove_deleted_image_cache(&mut self, key: &ImageCacheKey) {
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

    pub(in crate::app) fn clear_pending_original_requests(&mut self) {
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
}
