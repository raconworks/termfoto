use super::*;

#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

impl App {
    pub fn is_favorites_view(&self) -> bool {
        self.gallery_mode == GalleryMode::Favorites
    }

    pub fn favorite_row_len(&self) -> usize {
        if self.gallery_mode == GalleryMode::Directory {
            self.favorite_row_len
        } else {
            0
        }
    }

    pub fn has_favorite_row(&self) -> bool {
        self.favorite_row_len() > 0
    }

    pub fn normal_visible_rows(&self, visible_rows: usize) -> usize {
        if self.has_favorite_row() {
            visible_rows.saturating_sub(1)
        } else {
            visible_rows
        }
    }

    pub fn is_favorite_index(&self, idx: usize) -> bool {
        self.images
            .get(idx)
            .is_some_and(|entry| self.favorites.is_favorite(&entry.path))
    }

    pub(in crate::app) fn rebuild_active_gallery(
        &mut self,
        selected_path: Option<PathBuf>,
    ) -> usize {
        match self.gallery_mode {
            GalleryMode::Directory => {
                self.rebuild_directory_gallery(selected_path);
                0
            }
            GalleryMode::Favorites => self.rebuild_favorites_gallery(selected_path),
        }
    }

    pub(in crate::app) fn rebuild_directory_gallery(&mut self, selected_path: Option<PathBuf>) {
        let max_pinned = self.grid_cols.max(1);
        let mut pinned = Vec::new();
        let mut pinned_keys = HashSet::new();

        for favorite in self.favorites.entries_newest_first() {
            if pinned.len() >= max_pinned {
                break;
            }
            let favorite_key = FavoriteStore::normalize_path(&favorite.path);
            if pinned_keys.contains(&favorite_key) {
                continue;
            }
            if let Some(entry) = image_entry_from_path(&favorite.path) {
                pinned.push(entry);
                pinned_keys.insert(favorite_key);
            }
        }

        let mut images = pinned;
        images.extend(
            self.directory_images
                .iter()
                .filter(|entry| !pinned_keys.contains(&FavoriteStore::normalize_path(&entry.path)))
                .cloned(),
        );

        self.favorite_row_len = images.len().min(pinned_keys.len());
        self.images = images;
        self.select_path_or_clamp(selected_path.as_deref());
    }

    pub(in crate::app) fn rebuild_favorites_gallery(
        &mut self,
        selected_path: Option<PathBuf>,
    ) -> usize {
        let mut images = Vec::new();
        let mut skipped = 0;
        let mut seen = HashSet::new();

        for favorite in self.favorites.entries_newest_first() {
            let key = FavoriteStore::normalize_path(&favorite.path);
            if !seen.insert(key) {
                continue;
            }
            if let Some(entry) = image_entry_from_path(&favorite.path) {
                images.push(entry);
            } else {
                skipped += 1;
            }
        }

        self.favorite_row_len = 0;
        self.images = images;
        self.select_path_or_clamp(selected_path.as_deref());
        skipped
    }

    pub(in crate::app) fn toggle_favorite_current(&mut self) {
        let Some(path) = self.current_selected_path() else {
            self.set_status_message(self.lang.favorite_no_image().to_string());
            return;
        };

        let was_favorite = self.favorites.is_favorite(&path);
        let old_selected = self.selected;
        let fallback_after_remove = if was_favorite && self.gallery_mode == GalleryMode::Favorites {
            self.images
                .get(old_selected + 1)
                .or_else(|| {
                    old_selected
                        .checked_sub(1)
                        .and_then(|idx| self.images.get(idx))
                })
                .map(|entry| entry.path.clone())
        } else {
            Some(path.clone())
        };

        let save_error = if was_favorite {
            self.favorites
                .remove(&path)
                .err()
                .map(|err| err.to_string())
        } else {
            self.favorites
                .add_now(&path)
                .err()
                .map(|err| err.to_string())
        };

        let selected_path = if was_favorite {
            fallback_after_remove
        } else {
            Some(path)
        };
        let skipped = self.rebuild_active_gallery(selected_path);
        self.invalidate_active_gallery();

        if let Some(error) = save_error {
            self.set_status_message(self.lang.favorite_save_failed(&error));
        } else if skipped > 0 {
            self.set_status_message(self.lang.favorite_missing_skipped(skipped));
        } else if was_favorite {
            self.set_status_message(self.lang.favorite_removed().to_string());
        } else {
            self.set_status_message(self.lang.favorite_added().to_string());
        }
    }

    pub(in crate::app) fn toggle_favorites_view(&mut self) {
        match self.gallery_mode {
            GalleryMode::Directory => {
                let current_path = self.current_selected_path();
                self.directory_selected_path = current_path.clone();
                self.gallery_mode = GalleryMode::Favorites;
                self.selected = 0;
                let skipped = self.rebuild_favorites_gallery(current_path);
                self.invalidate_active_gallery();
                if skipped > 0 {
                    self.set_status_message(self.lang.favorite_missing_skipped(skipped));
                } else if self.images.is_empty() {
                    self.set_status_message(self.lang.favorite_none().to_string());
                }
            }
            GalleryMode::Favorites => {
                let current_path = self.current_selected_path();
                let selected_path = current_path
                    .filter(|path| self.directory_contains_path(path))
                    .or_else(|| self.directory_selected_path.clone());
                self.gallery_mode = GalleryMode::Directory;
                self.directory_selected_path = None;
                self.selected = 0;
                self.rebuild_directory_gallery(selected_path);
                self.invalidate_active_gallery();
            }
        }
    }

    pub(in crate::app) fn directory_contains_path(&self, path: &Path) -> bool {
        let key = FavoriteStore::normalize_path(path);
        self.directory_images
            .iter()
            .any(|entry| FavoriteStore::normalize_path(&entry.path) == key)
    }
}

#[cfg(not(test))]
pub(in crate::app) fn default_favorite_store() -> FavoriteStore {
    FavoriteStore::load_default()
}

#[cfg(test)]
pub(in crate::app) fn default_favorite_store() -> FavoriteStore {
    static NEXT_FAVORITE_STORE: AtomicUsize = AtomicUsize::new(0);
    FavoriteStore::empty_at(std::env::temp_dir().join(format!(
        "termfoto-test-favorites-{}-{}.tsv",
        std::process::id(),
        NEXT_FAVORITE_STORE.fetch_add(1, Ordering::Relaxed)
    )))
}
