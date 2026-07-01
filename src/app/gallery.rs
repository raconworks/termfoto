use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectoryContextKind {
    Directory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryContextEntry {
    pub name: String,
    pub path: PathBuf,
    pub kind: DirectoryContextKind,
    pub is_current: bool,
    pub depth: usize,
}

impl App {
    pub fn sort_label(&self) -> &'static str {
        match (self.lang, self.sort_mode) {
            (Lang::Zh, ImageSortMode::Name) => "名称",
            (Lang::Zh, ImageSortMode::Modified) => "修改时间",
            (Lang::Zh, ImageSortMode::Size) => "大小",
            (Lang::En, ImageSortMode::Name) => "Name",
            (Lang::En, ImageSortMode::Modified) => "Modified",
            (Lang::En, ImageSortMode::Size) => "Size",
        }
    }

    pub fn set_grid_layout(&mut self, grid_cols: usize, visible_rows: usize) {
        let grid_cols = grid_cols.max(1);
        let previous_cols = self.grid_cols;
        self.grid_cols = grid_cols;
        self.visible_rows = visible_rows.max(1);
        if previous_cols != grid_cols && self.gallery_mode == GalleryMode::Directory {
            let selected_path = self.current_selected_path();
            let old_images = active_path_keys(&self.images);
            self.rebuild_directory_gallery(selected_path);
            if old_images != active_path_keys(&self.images) {
                self.invalidate_active_gallery();
            }
        }
    }

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

    pub(super) fn current_selected_path(&self) -> Option<PathBuf> {
        self.images
            .get(self.selected)
            .map(|entry| entry.path.clone())
    }

    pub(super) fn current_selected_cache_key(&self) -> Option<ImageCacheKey> {
        self.images
            .get(self.selected)
            .map(ImageCacheKey::from_entry)
    }

    pub(crate) fn image_cache_key_for_slot(&self, idx: usize) -> Option<ImageCacheKey> {
        self.images.get(idx).map(ImageCacheKey::from_entry)
    }

    pub(super) fn rebuild_active_gallery(&mut self, selected_path: Option<PathBuf>) -> usize {
        match self.gallery_mode {
            GalleryMode::Directory => {
                self.rebuild_directory_gallery(selected_path);
                0
            }
            GalleryMode::Favorites => self.rebuild_favorites_gallery(selected_path),
        }
    }

    pub(super) fn rebuild_directory_gallery(&mut self, selected_path: Option<PathBuf>) {
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

    pub(super) fn rebuild_favorites_gallery(&mut self, selected_path: Option<PathBuf>) -> usize {
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

    pub(super) fn select_path_or_clamp(&mut self, selected_path: Option<&Path>) {
        if self.images.is_empty() {
            self.selected = 0;
            self.scroll_row = 0;
            return;
        }

        if let Some(selected_path) = selected_path {
            let selected_key = FavoriteStore::normalize_path(selected_path);
            if let Some(idx) = self
                .images
                .iter()
                .position(|entry| FavoriteStore::normalize_path(&entry.path) == selected_key)
            {
                self.selected = idx;
                self.clamp_scroll(self.visible_rows.max(1));
                return;
            }
        }

        self.selected = self.selected.min(self.images.len().saturating_sub(1));
        self.clamp_scroll(self.visible_rows.max(1));
    }

    pub(super) fn invalidate_active_gallery(&mut self) {
        if self.state == AppState::Fullscreen {
            if self.images.is_empty() {
                self.exit_fullscreen();
            } else if self.fullscreen_content_key == self.current_selected_cache_key() {
                self.update_fullscreen_original_interest();
                self.prefetch_fullscreen_neighbors();
            } else {
                self.reset_fullscreen_content();
                self.zoom = 1.0;
                self.pan_x = 0;
                self.pan_y = 0;
                self.prepare_fullscreen_selection();
            }
        }
    }

    pub fn cycle_sort_mode(&mut self) {
        self.sort_mode = self.sort_mode.next();
        self.sort_images_preserving_selection();
    }

    pub(super) fn sort_images_preserving_selection(&mut self) {
        let selected_path = self.current_selected_path();
        sort_image_entries(&mut self.directory_images, self.sort_mode);
        if self.gallery_mode == GalleryMode::Directory {
            self.rebuild_directory_gallery(selected_path);
            self.invalidate_active_gallery();
        }
    }

    pub fn directory_context_for_browser(&self) -> Vec<DirectoryContextEntry> {
        browser_directory_context_entries(self.context_dir.as_path())
    }

    pub fn clamp_context_selection(&mut self, len: usize, visible_rows: usize) {
        self.context_visible_rows = visible_rows.max(1);
        if len == 0 {
            self.context_selected = 0;
            self.context_scroll = 0;
            return;
        }
        self.context_selected = self.context_selected.min(len - 1);
        self.clamp_context_scroll_bounds(len);
    }

    pub(super) fn clamp_context_scroll_to_selection(&mut self, len: usize) {
        if len == 0 {
            self.context_scroll = 0;
            return;
        }
        let visible_rows = self.context_visible_rows.max(1);
        if self.context_selected < self.context_scroll {
            self.context_scroll = self.context_selected;
        } else if self.context_selected >= self.context_scroll + visible_rows {
            self.context_scroll = self.context_selected + 1 - visible_rows;
        }
        self.clamp_context_scroll_bounds(len);
    }

    pub(super) fn clamp_context_scroll_bounds(&mut self, len: usize) {
        let max_scroll = len.saturating_sub(self.context_visible_rows.max(1));
        self.context_scroll = self.context_scroll.min(max_scroll);
    }

    pub(super) fn context_down(&mut self) {
        let len = self.directory_context_for_browser().len();
        if self.context_selected + 1 < len {
            self.context_selected += 1;
            self.clamp_context_scroll_to_selection(len);
        }
    }

    pub(super) fn context_up(&mut self) {
        self.context_selected = self.context_selected.saturating_sub(1);
        let len = self.directory_context_for_browser().len();
        self.clamp_context_scroll_to_selection(len);
    }

    pub(super) fn context_home(&mut self) {
        self.context_selected = 0;
        let len = self.directory_context_for_browser().len();
        self.clamp_context_scroll_to_selection(len);
    }

    pub(super) fn context_end(&mut self) {
        let len = self.directory_context_for_browser().len();
        self.context_selected = len.saturating_sub(1);
        self.clamp_context_scroll_to_selection(len);
    }

    pub(super) fn enter_selected_context_directory(&mut self) {
        let entries = self.directory_context_for_browser();
        let Some(entry) = entries.get(self.context_selected) else {
            return;
        };
        self.enter_directory_with_context(entry.path.clone(), entry.path.clone());
    }

    pub(super) fn enter_parent_directory(&mut self) {
        let Some(new_image_dir) = browser_context_parent(self.image_dir.as_path()) else {
            return;
        };
        self.enter_directory_with_context(new_image_dir.clone(), new_image_dir);
    }

    #[cfg(test)]
    pub(super) fn enter_directory(&mut self, dir: PathBuf) {
        self.enter_directory_with_context(dir.clone(), dir);
    }

    pub(super) fn enter_directory_with_context(&mut self, dir: PathBuf, context_dir: PathBuf) {
        let Ok(images) = scan_directory(&dir) else {
            self.status_message = Some((
                format!("{}: {}", self.lang.directory_error(), dir.display()),
                Instant::now() + Duration::from_secs(2),
            ));
            return;
        };

        self.image_dir = dir;
        self.context_dir = context_dir;
        let mut images = images;
        sort_image_entries(&mut images, self.sort_mode);
        self.directory_images = images;
        self.gallery_mode = GalleryMode::Directory;
        self.directory_selected_path = None;
        self.rebuild_directory_gallery(None);
        self.selected = 0;
        self.scroll_row = 0;
        self.reset_context_selection_to_current_folder();
        self.search = None;
        self.status_message = None;
    }

    pub(super) fn reset_context_selection_to_current_folder(&mut self) {
        let entries = self.directory_context_for_browser();
        self.context_selected = if entries.len() > 1 { 1 } else { 0 };
        self.context_scroll = 0;
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
        let Some((row, col)) = self.visual_position(self.selected) else {
            return;
        };
        if row == 0 {
            if !self.has_favorite_row() {
                self.selected = 0;
            }
            return;
        }
        if let Some(idx) = self.nearest_index_in_visual_row(row - 1, col) {
            self.selected = idx;
        }
    }

    pub fn navigate_down(&mut self) {
        let Some((row, col)) = self.visual_position(self.selected) else {
            return;
        };
        if let Some(idx) = self.index_at_visual_position(row + 1, col) {
            self.selected = idx;
        }
    }

    pub fn navigate_home(&mut self) {
        let favorite_row_len = self.favorite_row_len();
        self.selected = if favorite_row_len > 0 && favorite_row_len < self.images.len() {
            favorite_row_len
        } else {
            0
        };
        self.scroll_row = 0;
    }

    pub fn navigate_end(&mut self) {
        self.selected = self.images.len().saturating_sub(1);
    }

    pub fn navigate_page_down(&mut self, visible_rows: usize) {
        let Some((row, col)) = self.visual_position(self.selected) else {
            return;
        };
        let target_row = (row + visible_rows.max(1)).min(self.last_visual_row());
        self.selected = self
            .nearest_index_in_visual_row(target_row, col)
            .unwrap_or_else(|| self.images.len().saturating_sub(1));
    }

    pub fn navigate_page_up(&mut self, visible_rows: usize) {
        let Some((row, col)) = self.visual_position(self.selected) else {
            return;
        };
        let target_row = row.saturating_sub(visible_rows.max(1));
        if let Some(idx) = self.nearest_index_in_visual_row(target_row, col) {
            self.selected = idx;
        }
    }

    pub fn clamp_scroll(&mut self, visible_rows: usize) {
        let grid_cols = self.grid_cols.max(1);
        let favorite_row_len = self.favorite_row_len();
        if favorite_row_len > 0 {
            let normal_visible_rows = visible_rows.saturating_sub(1);
            if self.selected < favorite_row_len || normal_visible_rows == 0 {
                return;
            }
            let selected_row = (self.selected - favorite_row_len) / grid_cols;
            if selected_row < self.scroll_row {
                self.scroll_row = selected_row;
            } else if selected_row >= self.scroll_row + normal_visible_rows {
                self.scroll_row = selected_row + 1 - normal_visible_rows;
            }
            let normal_len = self.images.len().saturating_sub(favorite_row_len);
            let normal_rows = normal_len.div_ceil(grid_cols);
            self.scroll_row = self
                .scroll_row
                .min(normal_rows.saturating_sub(normal_visible_rows));
            return;
        }

        let selected_row = self.selected / grid_cols;
        if selected_row < self.scroll_row {
            self.scroll_row = selected_row;
        } else if selected_row >= self.scroll_row + visible_rows {
            self.scroll_row = selected_row + 1 - visible_rows;
        }
        let rows = self.images.len().div_ceil(grid_cols);
        self.scroll_row = self.scroll_row.min(rows.saturating_sub(visible_rows));
    }

    pub(super) fn visual_position(&self, idx: usize) -> Option<(usize, usize)> {
        if idx >= self.images.len() {
            return None;
        }
        let grid_cols = self.grid_cols.max(1);
        let favorite_row_len = self.favorite_row_len();
        if favorite_row_len > 0 {
            if idx < favorite_row_len {
                Some((0, idx))
            } else {
                let normal_idx = idx - favorite_row_len;
                Some((1 + normal_idx / grid_cols, normal_idx % grid_cols))
            }
        } else {
            Some((idx / grid_cols, idx % grid_cols))
        }
    }

    pub(super) fn index_at_visual_position(&self, row: usize, col: usize) -> Option<usize> {
        let grid_cols = self.grid_cols.max(1);
        if col >= grid_cols {
            return None;
        }
        let favorite_row_len = self.favorite_row_len();
        let idx = if favorite_row_len > 0 {
            if row == 0 {
                if col < favorite_row_len {
                    col
                } else {
                    return None;
                }
            } else {
                favorite_row_len + (row - 1) * grid_cols + col
            }
        } else {
            row * grid_cols + col
        };
        (idx < self.images.len()).then_some(idx)
    }

    pub(super) fn nearest_index_in_visual_row(
        &self,
        row: usize,
        preferred_col: usize,
    ) -> Option<usize> {
        if let Some(idx) = self.index_at_visual_position(row, preferred_col) {
            return Some(idx);
        }
        let grid_cols = self.grid_cols.max(1);
        (0..grid_cols)
            .rev()
            .filter(|col| *col <= preferred_col)
            .find_map(|col| self.index_at_visual_position(row, col))
            .or_else(|| {
                (preferred_col + 1..grid_cols)
                    .find_map(|col| self.index_at_visual_position(row, col))
            })
    }

    pub(super) fn last_visual_row(&self) -> usize {
        self.images
            .len()
            .checked_sub(1)
            .and_then(|idx| self.visual_position(idx))
            .map(|(row, _)| row)
            .unwrap_or(0)
    }

    pub(super) fn toggle_favorite_current(&mut self) {
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

    pub(super) fn toggle_favorites_view(&mut self) {
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

    pub(super) fn directory_contains_path(&self, path: &Path) -> bool {
        let key = FavoriteStore::normalize_path(path);
        self.directory_images
            .iter()
            .any(|entry| FavoriteStore::normalize_path(&entry.path) == key)
    }
}

#[cfg(not(test))]
pub(super) fn default_favorite_store() -> FavoriteStore {
    FavoriteStore::load_default()
}

#[cfg(test)]
pub(super) fn default_favorite_store() -> FavoriteStore {
    static NEXT_FAVORITE_STORE: AtomicUsize = AtomicUsize::new(0);
    FavoriteStore::empty_at(std::env::temp_dir().join(format!(
        "termfoto-test-favorites-{}-{}.tsv",
        std::process::id(),
        NEXT_FAVORITE_STORE.fetch_add(1, Ordering::Relaxed)
    )))
}

pub(super) fn browser_directory_context_entries(context_dir: &Path) -> Vec<DirectoryContextEntry> {
    let mut entries = vec![DirectoryContextEntry {
        name: directory_display_name(context_dir),
        path: context_dir.to_path_buf(),
        kind: DirectoryContextKind::Directory,
        is_current: true,
        depth: 0,
    }];

    let Ok(read_dir) = std::fs::read_dir(context_dir) else {
        return entries;
    };

    let mut children: Vec<_> = read_dir
        .filter_map(|res| res.ok())
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            if !file_type.is_dir() {
                return None;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                return None;
            }
            Some(DirectoryContextEntry {
                name,
                path: entry.path(),
                kind: DirectoryContextKind::Directory,
                is_current: false,
                depth: 1,
            })
        })
        .collect();

    children.sort_by(|a, b| a.name.cmp(&b.name));
    entries.extend(children);
    entries
}

pub(super) fn directory_display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

pub(super) fn browser_context_parent(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    if parent.as_os_str().is_empty() {
        Some(PathBuf::from("."))
    } else {
        Some(parent.to_path_buf())
    }
}

pub(super) fn sort_image_entries(images: &mut [ImageEntry], sort_mode: ImageSortMode) {
    match sort_mode {
        ImageSortMode::Name => {
            images.sort_by(|a, b| a.filename.cmp(&b.filename));
        }
        ImageSortMode::Modified => {
            images.sort_by(|a, b| {
                b.modified_at
                    .cmp(&a.modified_at)
                    .then_with(|| a.filename.cmp(&b.filename))
            });
        }
        ImageSortMode::Size => {
            images.sort_by(|a, b| {
                b.file_size
                    .cmp(&a.file_size)
                    .then_with(|| a.filename.cmp(&b.filename))
            });
        }
    }
}

pub(super) fn active_path_keys(images: &[ImageEntry]) -> Vec<PathBuf> {
    images
        .iter()
        .map(|entry| FavoriteStore::normalize_path(&entry.path))
        .collect()
}
