use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct ImageEntry {
    pub path: PathBuf,
    pub filename: String,
    pub file_size: u64,
    pub modified_at: Option<SystemTime>,
}

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "gif", "bmp", "tiff", "tif", "ico",
];

pub fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn image_entry_from_path(path: &Path) -> Option<ImageEntry> {
    if !path.is_file() || !is_supported_image(path) {
        return None;
    }

    let filename = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let metadata = std::fs::metadata(path).ok();
    let file_size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
    let modified_at = metadata.and_then(|m| m.modified().ok());
    Some(ImageEntry {
        path: path.to_path_buf(),
        filename,
        file_size,
        modified_at,
    })
}

pub fn scan_directory(dir: &Path) -> anyhow::Result<Vec<ImageEntry>> {
    let mut entries: Vec<ImageEntry> = std::fs::read_dir(dir)?
        .filter_map(|res| res.ok())
        .map(|e| e.path())
        .filter_map(|path| image_entry_from_path(&path))
        .collect();

    entries.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn create_fake_png(dir: &Path, name: &str) {
        let img = image::RgbImage::from_fn(1, 1, |_, _| image::Rgb([255, 0, 0]));
        img.save(dir.join(name)).unwrap();
    }

    #[test]
    fn test_is_supported_image_png() {
        assert!(is_supported_image(Path::new("photo.png")));
    }

    #[test]
    fn test_is_supported_image_jpg() {
        assert!(is_supported_image(Path::new("photo.jpg")));
        assert!(is_supported_image(Path::new("photo.jpeg")));
    }

    #[test]
    fn test_is_supported_image_webp() {
        assert!(is_supported_image(Path::new("photo.webp")));
    }

    #[test]
    fn test_is_supported_image_gif() {
        assert!(is_supported_image(Path::new("anim.gif")));
    }

    #[test]
    fn test_is_supported_image_bmp() {
        assert!(is_supported_image(Path::new("photo.bmp")));
    }

    #[test]
    fn test_is_supported_image_tiff() {
        assert!(is_supported_image(Path::new("photo.tiff")));
    }

    #[test]
    fn test_is_supported_image_ico() {
        assert!(is_supported_image(Path::new("icon.ico")));
    }

    #[test]
    fn test_is_supported_image_rejects_txt() {
        assert!(!is_supported_image(Path::new("readme.txt")));
    }

    #[test]
    fn test_is_supported_image_case_insensitive() {
        assert!(is_supported_image(Path::new("photo.PNG")));
        assert!(is_supported_image(Path::new("photo.JPG")));
    }

    #[test]
    fn test_scan_directory_returns_sorted_entries() {
        let dir = tempdir().unwrap();
        create_fake_png(dir.path(), "zebra.png");
        create_fake_png(dir.path(), "apple.png");
        create_fake_png(dir.path(), "mango.png");

        let entries = scan_directory(dir.path()).unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.filename.as_str()).collect();
        assert_eq!(names, vec!["apple.png", "mango.png", "zebra.png"]);
    }

    #[test]
    fn test_scan_directory_populates_metadata() {
        let dir = tempdir().unwrap();
        create_fake_png(dir.path(), "photo.png");

        let entries = scan_directory(dir.path()).unwrap();

        assert_eq!(entries.len(), 1);
        assert!(entries[0].file_size > 0);
        assert!(entries[0].modified_at.is_some());
    }

    #[test]
    fn test_scan_directory_filters_non_images() {
        let dir = tempdir().unwrap();
        create_fake_png(dir.path(), "photo.png");
        fs::write(dir.path().join("readme.txt"), b"hello").unwrap();
        fs::write(dir.path().join("script.sh"), b"#!/bin/sh").unwrap();

        let entries = scan_directory(dir.path()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].filename, "photo.png");
    }

    #[test]
    fn test_scan_directory_not_recursive() {
        let dir = tempdir().unwrap();
        create_fake_png(dir.path(), "top.png");
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        create_fake_png(&subdir, "nested.png");

        let entries = scan_directory(dir.path()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].filename, "top.png");
    }

    #[test]
    fn test_scan_empty_directory() {
        let dir = tempdir().unwrap();
        let entries = scan_directory(dir.path()).unwrap();
        assert!(entries.is_empty());
    }
}
