use super::*;

impl App {
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

    pub(in crate::app) fn current_selected_path(&self) -> Option<PathBuf> {
        self.images
            .get(self.selected)
            .map(|entry| entry.path.clone())
    }

    pub(in crate::app) fn current_selected_cache_key(&self) -> Option<ImageCacheKey> {
        self.images
            .get(self.selected)
            .map(ImageCacheKey::from_entry)
    }

    pub(crate) fn image_cache_key_for_slot(&self, idx: usize) -> Option<ImageCacheKey> {
        self.images.get(idx).map(ImageCacheKey::from_entry)
    }

    pub(in crate::app) fn select_path_or_clamp(&mut self, selected_path: Option<&Path>) {
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

    pub(in crate::app) fn invalidate_active_gallery(&mut self) {
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

    pub(in crate::app) fn visual_position(&self, idx: usize) -> Option<(usize, usize)> {
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

    pub(in crate::app) fn index_at_visual_position(&self, row: usize, col: usize) -> Option<usize> {
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

    pub(in crate::app) fn nearest_index_in_visual_row(
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

    pub(in crate::app) fn last_visual_row(&self) -> usize {
        self.images
            .len()
            .checked_sub(1)
            .and_then(|idx| self.visual_position(idx))
            .map(|(row, _)| row)
            .unwrap_or(0)
    }
}
