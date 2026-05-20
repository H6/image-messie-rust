use std::path::Path;

use walkdir::WalkDir;

use crate::models::{FileInfo, IMAGE_EXTENSIONS};

/// Recursively walk `root` and yield one [`FileInfo`] per image file found.
/// Files whose extension is not in [`IMAGE_EXTENSIONS`] are silently skipped.
pub fn scan_images(root: &Path) -> impl Iterator<Item = FileInfo> + '_ {
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let path = entry.into_path();

            let extension = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .unwrap_or_default();

            if !IMAGE_EXTENSIONS.contains(&extension.as_str()) {
                return None;
            }

            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();

            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let size_bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

            Some(FileInfo {
                path,
                size_bytes,
                stem,
                extension,
                filename,
            })
        })
}
