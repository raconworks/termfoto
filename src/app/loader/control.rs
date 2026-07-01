use super::*;

#[derive(Clone)]
pub struct LoadControl {
    inner: Arc<Mutex<LoadControlState>>,
}

#[derive(Default)]
struct LoadControlState {
    generation: u64,
    thumbnail_interest: Option<ThumbnailInterest>,
    original_interest: Option<OriginalInterest>,
}

struct ThumbnailInterest {
    w: u16,
    h: u16,
    keys: HashSet<ImageCacheKey>,
}

struct OriginalInterest {
    w: u16,
    h: u16,
    selected: Option<ImageCacheKey>,
    prefetch: HashSet<ImageCacheKey>,
}

impl LoadControl {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(LoadControlState::default())),
        }
    }

    pub(in crate::app) fn set_generation(&self, generation: u64) {
        let mut state = self.inner.lock().unwrap();
        state.generation = generation;
        state.thumbnail_interest = None;
        state.original_interest = None;
    }

    pub(in crate::app) fn update_thumbnail_interest<I>(
        &self,
        generation: u64,
        w: u16,
        h: u16,
        keys: I,
    ) where
        I: IntoIterator<Item = ImageCacheKey>,
    {
        let mut state = self.inner.lock().unwrap();
        ensure_load_generation(&mut state, generation);
        state.thumbnail_interest = Some(ThumbnailInterest {
            w,
            h,
            keys: keys.into_iter().collect(),
        });
    }

    pub(in crate::app) fn clear_thumbnail_interest(&self, generation: u64) {
        let mut state = self.inner.lock().unwrap();
        ensure_load_generation(&mut state, generation);
        state.thumbnail_interest = None;
    }

    pub(in crate::app) fn update_original_interest<I>(
        &self,
        generation: u64,
        w: u16,
        h: u16,
        selected: Option<ImageCacheKey>,
        prefetch: I,
    ) where
        I: IntoIterator<Item = ImageCacheKey>,
    {
        let mut state = self.inner.lock().unwrap();
        ensure_load_generation(&mut state, generation);
        state.original_interest = Some(OriginalInterest {
            w,
            h,
            selected,
            prefetch: prefetch.into_iter().collect(),
        });
    }

    pub(in crate::app) fn clear_original_interest(&self, generation: u64) {
        let mut state = self.inner.lock().unwrap();
        ensure_load_generation(&mut state, generation);
        state.original_interest = None;
    }

    pub(in crate::app) fn remove_interest_key(&self, generation: u64, key: &ImageCacheKey) {
        let mut state = self.inner.lock().unwrap();
        ensure_load_generation(&mut state, generation);
        if let Some(interest) = state.thumbnail_interest.as_mut() {
            interest.keys.remove(key);
        }
        if let Some(interest) = state.original_interest.as_mut() {
            if interest.selected.as_ref() == Some(key) {
                interest.selected = None;
            }
            interest.prefetch.remove(key);
        }
    }

    pub(in crate::app) fn allows(&self, req: &LoadRequest) -> bool {
        let state = self.inner.lock().unwrap();
        if req.generation != state.generation {
            return false;
        }

        match &req.size {
            LoadSize::Thumbnail { w, h } => {
                state.thumbnail_interest.as_ref().is_some_and(|interest| {
                    interest.w == *w && interest.h == *h && interest.keys.contains(&req.key)
                })
            }
            LoadSize::Original { w, h, kind } => {
                state.original_interest.as_ref().is_some_and(|interest| {
                    interest.w == *w
                        && interest.h == *h
                        && match kind {
                            OriginalLoadKind::Selected => {
                                interest.selected.as_ref() == Some(&req.key)
                            }
                            OriginalLoadKind::Prefetch => interest.prefetch.contains(&req.key),
                        }
                })
            }
        }
    }
}

impl Default for LoadControl {
    fn default() -> Self {
        Self::new()
    }
}

fn ensure_load_generation(state: &mut LoadControlState, generation: u64) {
    if state.generation != generation {
        state.generation = generation;
        state.thumbnail_interest = None;
        state.original_interest = None;
    }
}
