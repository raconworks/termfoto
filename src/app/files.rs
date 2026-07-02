use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct RenameState {
    pub original_path: PathBuf,
    pub original_filename: String,
    pub original_stem: String,
    pub input: String,
    pub pending_overwrite: bool,
    pub origin: AppState,
    pub message: Option<String>,
}

impl RenameState {
    pub(super) fn new(original_path: PathBuf, original_filename: String, origin: AppState) -> Self {
        let original_stem = original_path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .filter(|stem| !stem.is_empty())
            .unwrap_or_else(|| original_filename.clone());
        Self {
            original_path,
            original_filename,
            original_stem: original_stem.clone(),
            input: original_stem,
            pending_overwrite: false,
            origin,
            message: None,
        }
    }

    pub(super) fn target_filename(&self) -> String {
        self.original_path
            .extension()
            .and_then(|ext| ext.to_str())
            .filter(|ext| !ext.is_empty())
            .map(|ext| format!("{}.{}", self.input, ext))
            .unwrap_or_else(|| self.input.clone())
    }

    pub(super) fn target_path(&self) -> PathBuf {
        let mut target = self.original_path.clone();
        target.set_file_name(self.target_filename());
        target
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeleteState {
    pub origin: AppState,
    pub path: PathBuf,
    pub filename: String,
    cache_key: ImageCacheKey,
}

impl DeleteState {
    pub(super) fn new(entry: &ImageEntry, origin: AppState) -> Self {
        Self {
            origin,
            path: entry.path.clone(),
            filename: entry.filename.clone(),
            cache_key: ImageCacheKey::from_entry(entry),
        }
    }
}

impl App {
    pub fn browser_status_message(&mut self) -> Option<String> {
        let (message, expires_at) = self.status_message.as_ref()?;
        if Instant::now() >= *expires_at {
            self.status_message = None;
            return None;
        }
        Some(message.clone())
    }

    pub fn rename_prompt_lines(&self) -> Option<Vec<String>> {
        let rename = self.rename.as_ref()?;
        if rename.pending_overwrite {
            Some(self.lang.rename_overwrite_prompt_lines(
                &rename.original_filename,
                &rename.target_filename(),
            ))
        } else {
            Some(self.lang.rename_prompt_lines(
                &rename.original_filename,
                &rename.input,
                rename.message.as_deref(),
            ))
        }
    }

    pub fn rename_prompt(&self) -> Option<crate::ui::RenamePrompt> {
        let rename = self.rename.as_ref()?;
        Some(crate::ui::RenamePrompt {
            lang: self.lang,
            original_name: rename.original_filename.clone(),
            input: rename.input.clone(),
            message: rename.message.clone(),
            pending_overwrite: rename.pending_overwrite,
            target_name: rename.pending_overwrite.then(|| rename.target_filename()),
        })
    }

    pub fn delete_prompt_lines(&self) -> Option<Vec<String>> {
        self.delete
            .as_ref()
            .map(|delete| self.lang.delete_prompt_lines(&delete.filename))
    }

    pub fn delete_prompt(&self) -> Option<crate::ui::DeletePrompt> {
        self.delete.as_ref().map(|delete| crate::ui::DeletePrompt {
            lang: self.lang,
            filename: delete.filename.clone(),
        })
    }

    pub(super) fn set_status_message(&mut self, message: String) {
        self.status_message = Some((message, Instant::now() + Duration::from_secs(2)));
    }

    pub(super) fn start_rename_current(&mut self) {
        let Some(entry) = self.images.get(self.selected) else {
            self.set_status_message(self.lang.rename_no_image().to_string());
            return;
        };
        self.rename = Some(RenameState::new(
            entry.path.clone(),
            entry.filename.clone(),
            self.state.clone(),
        ));
    }

    pub(super) fn start_delete_current(&mut self) {
        let Some(entry) = self.images.get(self.selected) else {
            self.set_status_message(self.lang.delete_no_image().to_string());
            return;
        };
        self.delete = Some(DeleteState::new(entry, self.state.clone()));
    }

    pub(super) fn handle_delete_key(&mut self, code: KeyCode) -> bool {
        let Some(delete) = self.delete.take() else {
            return false;
        };

        match code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                let favorite_path = FavoriteStore::normalize_path(&delete.path);
                match fs::remove_file(&delete.path) {
                    Ok(()) => self.finish_delete_success(delete, favorite_path),
                    Err(err) => {
                        self.set_status_message(self.lang.delete_failed(&err.to_string()));
                    }
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.set_status_message(self.lang.delete_cancelled().to_string());
            }
            _ => {
                self.delete = Some(delete);
            }
        }
        false
    }

    pub(super) fn handle_rename_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        let Some(mut rename) = self.rename.take() else {
            return false;
        };

        if rename.pending_overwrite {
            match code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    let target_path = rename.target_path();
                    match rename_over_existing(&rename.original_path, &target_path) {
                        Ok(()) => self.finish_rename_success(rename.original_path, target_path),
                        Err(err) => self.set_status_message(
                            self.lang.rename_failed(&err.to_string()).to_string(),
                        ),
                    }
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.set_status_message(self.lang.rename_cancelled().to_string());
                }
                _ => {
                    self.rename = Some(rename);
                }
            }
            return false;
        }

        match code {
            KeyCode::Esc => {
                self.set_status_message(self.lang.rename_cancelled().to_string());
            }
            KeyCode::Backspace => {
                rename.input.pop();
                rename.message = None;
                self.rename = Some(rename);
            }
            KeyCode::Enter => {
                if let Some(rename) = self.submit_rename(rename) {
                    self.rename = Some(rename);
                }
            }
            KeyCode::Char(c)
                if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                rename.input.push(c);
                rename.message = None;
                self.rename = Some(rename);
            }
            _ => {
                self.rename = Some(rename);
            }
        }
        false
    }

    pub(super) fn submit_rename(&mut self, mut rename: RenameState) -> Option<RenameState> {
        if rename.input.is_empty() {
            rename.message = Some(self.lang.rename_empty_name().to_string());
            return Some(rename);
        }
        if rename.input.contains('/') || rename.input.contains('\\') {
            rename.message = Some(self.lang.rename_invalid_name().to_string());
            return Some(rename);
        }
        if rename.input == rename.original_stem {
            self.set_status_message(self.lang.rename_unchanged().to_string());
            return None;
        }

        let target_path = rename.target_path();
        if target_path.exists() {
            rename.pending_overwrite = true;
            rename.message = None;
            return Some(rename);
        }

        match fs::rename(&rename.original_path, &target_path) {
            Ok(()) => {
                self.finish_rename_success(rename.original_path, target_path);
                None
            }
            Err(err) => {
                self.set_status_message(self.lang.rename_failed(&err.to_string()).to_string());
                None
            }
        }
    }

    pub(super) fn finish_delete_success(&mut self, delete: DeleteState, favorite_path: PathBuf) {
        let DeleteState {
            path: _,
            filename,
            cache_key,
            ..
        } = delete;
        let selected_path = self.selection_after_current_delete();
        let favorite_remove_error = if self.favorites.is_favorite(&favorite_path) {
            self.favorites
                .remove(&favorite_path)
                .err()
                .map(|err| err.to_string())
        } else {
            None
        };

        self.search = None;
        self.rename = None;
        self.delete = None;
        self.remove_deleted_image_cache(&cache_key);

        match scan_directory(&self.image_dir) {
            Ok(mut images) => {
                sort_image_entries(&mut images, self.sort_mode);
                self.directory_images = images;
            }
            Err(_) if self.gallery_mode == GalleryMode::Directory => {
                self.set_status_message(self.lang.directory_error().to_string());
                return;
            }
            Err(_) => {}
        }

        let skipped = self.rebuild_active_gallery(selected_path);
        self.invalidate_active_gallery();
        if let Some(error) = favorite_remove_error {
            self.set_status_message(self.lang.favorite_save_failed(&error));
        } else if skipped > 0 {
            self.set_status_message(self.lang.favorite_missing_skipped(skipped));
        } else if self.gallery_mode == GalleryMode::Favorites && self.images.is_empty() {
            self.set_status_message(self.lang.favorite_none().to_string());
        } else {
            self.set_status_message(self.lang.delete_success(&filename));
        }
    }

    pub(super) fn selection_after_current_delete(&self) -> Option<PathBuf> {
        self.images
            .get(self.selected + 1)
            .or_else(|| {
                self.selected
                    .checked_sub(1)
                    .and_then(|idx| self.images.get(idx))
            })
            .map(|entry| entry.path.clone())
    }

    pub(super) fn finish_rename_success(&mut self, original_path: PathBuf, target_path: PathBuf) {
        let target_name = target_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| target_path.to_string_lossy().into_owned());
        let favorite_update_error = if self.favorites.is_favorite(&original_path) {
            self.favorites
                .update_path(&original_path, &target_path)
                .err()
                .map(|err| err.to_string())
        } else {
            None
        };

        self.search = None;
        self.rename = None;

        match scan_directory(&self.image_dir) {
            Ok(mut images) => {
                sort_image_entries(&mut images, self.sort_mode);
                self.directory_images = images;
            }
            Err(_) if self.gallery_mode == GalleryMode::Directory => {
                self.set_status_message(self.lang.directory_error().to_string());
                return;
            }
            Err(_) => {}
        }

        let skipped = self.rebuild_active_gallery(Some(target_path.clone()));
        self.invalidate_active_gallery();
        if let Some(error) = favorite_update_error {
            self.set_status_message(self.lang.favorite_save_failed(&error));
        } else if skipped > 0 {
            self.set_status_message(self.lang.favorite_missing_skipped(skipped));
        } else {
            self.set_status_message(self.lang.rename_success(&target_name).to_string());
        }
    }
}

fn rename_over_existing(source: &Path, target: &Path) -> io::Result<()> {
    if !target.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "target is not a file",
        ));
    }
    let backup = unique_overwrite_backup_path(target)?;
    fs::rename(target, &backup)?;
    match fs::rename(source, target) {
        Ok(()) => {
            let _ = fs::remove_file(backup);
            Ok(())
        }
        Err(err) => {
            let _ = fs::rename(&backup, target);
            Err(err)
        }
    }
}

fn unique_overwrite_backup_path(target: &Path) -> io::Result<PathBuf> {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let filename = target
        .file_name()
        .map(|name| name.to_string_lossy())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing target filename"))?;
    let pid = std::process::id();

    for attempt in 0..1000 {
        let candidate = parent.join(format!(
            ".{}.termfoto-overwrite-backup-{}-{}",
            filename, pid, attempt
        ));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not create backup path",
    ))
}
