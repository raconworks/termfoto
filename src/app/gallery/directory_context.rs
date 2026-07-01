use super::*;

impl App {
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

    pub(in crate::app) fn clamp_context_scroll_to_selection(&mut self, len: usize) {
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

    pub(in crate::app) fn clamp_context_scroll_bounds(&mut self, len: usize) {
        let max_scroll = len.saturating_sub(self.context_visible_rows.max(1));
        self.context_scroll = self.context_scroll.min(max_scroll);
    }

    pub(in crate::app) fn context_down(&mut self) {
        let len = self.directory_context_for_browser().len();
        if self.context_selected + 1 < len {
            self.context_selected += 1;
            self.clamp_context_scroll_to_selection(len);
        }
    }

    pub(in crate::app) fn context_up(&mut self) {
        self.context_selected = self.context_selected.saturating_sub(1);
        let len = self.directory_context_for_browser().len();
        self.clamp_context_scroll_to_selection(len);
    }

    pub(in crate::app) fn context_home(&mut self) {
        self.context_selected = 0;
        let len = self.directory_context_for_browser().len();
        self.clamp_context_scroll_to_selection(len);
    }

    pub(in crate::app) fn context_end(&mut self) {
        let len = self.directory_context_for_browser().len();
        self.context_selected = len.saturating_sub(1);
        self.clamp_context_scroll_to_selection(len);
    }

    pub(in crate::app) fn enter_selected_context_directory(&mut self) {
        let entries = self.directory_context_for_browser();
        let Some(entry) = entries.get(self.context_selected) else {
            return;
        };
        self.enter_directory_with_context(entry.path.clone(), entry.path.clone());
    }

    pub(in crate::app) fn enter_parent_directory(&mut self) {
        let Some(new_image_dir) = browser_context_parent(self.image_dir.as_path()) else {
            return;
        };
        self.enter_directory_with_context(new_image_dir.clone(), new_image_dir);
    }

    #[cfg(test)]
    pub(in crate::app) fn enter_directory(&mut self, dir: PathBuf) {
        self.enter_directory_with_context(dir.clone(), dir);
    }

    pub(in crate::app) fn enter_directory_with_context(
        &mut self,
        dir: PathBuf,
        context_dir: PathBuf,
    ) {
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

    pub(in crate::app) fn reset_context_selection_to_current_folder(&mut self) {
        let entries = self.directory_context_for_browser();
        self.context_selected = if entries.len() > 1 { 1 } else { 0 };
        self.context_scroll = 0;
    }
}

pub(in crate::app) fn browser_directory_context_entries(
    context_dir: &Path,
) -> Vec<DirectoryContextEntry> {
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

pub(in crate::app) fn directory_display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

pub(in crate::app) fn browser_context_parent(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    if parent.as_os_str().is_empty() {
        Some(PathBuf::from("."))
    } else {
        Some(parent.to_path_buf())
    }
}
