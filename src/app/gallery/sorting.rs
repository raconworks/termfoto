use super::*;

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

    pub fn cycle_sort_mode(&mut self) {
        self.sort_mode = self.sort_mode.next();
        self.sort_images_preserving_selection();
    }

    pub(in crate::app) fn sort_images_preserving_selection(&mut self) {
        let selected_path = self.current_selected_path();
        sort_image_entries(&mut self.directory_images, self.sort_mode);
        if self.gallery_mode == GalleryMode::Directory {
            self.rebuild_directory_gallery(selected_path);
            self.invalidate_active_gallery();
        }
    }
}

pub(in crate::app) fn sort_image_entries(images: &mut [ImageEntry], sort_mode: ImageSortMode) {
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

pub(in crate::app) fn active_path_keys(images: &[ImageEntry]) -> Vec<PathBuf> {
    images
        .iter()
        .map(|entry| FavoriteStore::normalize_path(&entry.path))
        .collect()
}
