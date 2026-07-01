use super::*;

mod directory_context;
mod favorites;
mod navigation;
mod sorting;

#[cfg(test)]
pub(super) use directory_context::{browser_context_parent, browser_directory_context_entries};
pub(super) use favorites::default_favorite_store;
pub(super) use sorting::{active_path_keys, sort_image_entries};

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
