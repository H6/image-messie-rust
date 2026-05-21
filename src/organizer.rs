use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::Local;

use crate::models::{ExifData, FileInfo};

/// Calculate the target directory for a file given its EXIF data.
///
/// Returns `(target_dir, is_unknown)`:
/// - `is_unknown = false` → `destination/YYYY/MM_MonthName/CAMERA_MODEL/`
/// - `is_unknown = true`  → `destination/UNKNOWN/`
pub fn target_dir(destination: &Path, exif: &ExifData) -> (PathBuf, bool) {
    match (
        &exif.year,
        &exif.month,
        &exif.month_name,
        &exif.camera_model,
    ) {
        (Some(y), Some(m), Some(mn), Some(cam)) => {
            let month_folder = format!("{}_{}", m, mn);
            let cam_folder = cam.to_uppercase().replace(' ', "_");
            (destination.join(y).join(month_folder).join(cam_folder), false)
        }
        _ => (destination.join("UNKNOWN"), true),
    }
}

/// Resolve the final destination filename, appending a timestamp suffix when a
/// file with the same name already exists at `dir`.
pub fn resolve_destination_filename(dir: &Path, file: &FileInfo) -> String {
    let candidate = format!("{}.{}", file.stem, file.extension);
    let candidate_path = dir.join(&candidate);

    if candidate_path.exists() {
        let ts = Local::now().format("%Y%m%d%H%M%S");
        format!("{}_{}_{}.{}", file.stem, ts, file.stem, file.extension)
            // Keep it simple: stem_TIMESTAMP.ext
            .trim_start()
            .to_string();
        // Mirror Python exactly: f'{file_stem}_{datetime.datetime.now().strftime("%Y%m%d%H%M%S")}{file_suffix}'
        format!("{}_{}.{}", file.stem, ts, file.extension)
    } else {
        candidate
    }
}

/// Copy `src` to `dest_dir/dest_filename`, creating parent directories as needed.
/// Strips null bytes from the path (mirrors the Python behaviour).
pub fn copy_file(src: &Path, dest_dir: &Path, dest_filename: &str) -> Result<PathBuf> {
    // Strip null bytes from directory path string.
    let dir_str = dest_dir
        .to_string_lossy()
        .replace('\x00', "");
    let clean_dir = PathBuf::from(dir_str);

    std::fs::create_dir_all(&clean_dir)?;

    let dest_path = clean_dir.join(dest_filename);
    std::fs::copy(src, &dest_path)?;
    Ok(dest_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ExifData;

    fn make_exif(year: &str, month: &str, month_name: &str, model: &str) -> ExifData {
        ExifData {
            year: Some(year.into()),
            month: Some(month.into()),
            month_name: Some(month_name.into()),
            camera_model: Some(model.into()),
            ..Default::default()
        }
    }

    #[test]
    fn target_dir_with_full_metadata() {
        let exif = make_exif("2023", "03", "March", "CANON_EOS_R5");
        let (dir, unknown) = target_dir(Path::new("/dst"), &exif);
        assert!(!unknown);
        assert_eq!(dir, PathBuf::from("/dst/2023/03_March/CANON_EOS_R5"));
    }

    #[test]
    fn target_dir_missing_model_goes_to_unknown() {
        let exif = ExifData {
            year: Some("2023".into()),
            month: Some("03".into()),
            month_name: Some("March".into()),
            camera_model: None,
            ..Default::default()
        };
        let (dir, unknown) = target_dir(Path::new("/dst"), &exif);
        assert!(unknown);
        assert_eq!(dir, PathBuf::from("/dst/UNKNOWN"));
    }

    #[test]
    fn target_dir_all_missing_goes_to_unknown() {
        let (dir, unknown) = target_dir(Path::new("/dst"), &ExifData::default());
        assert!(unknown);
        assert_eq!(dir, PathBuf::from("/dst/UNKNOWN"));
    }

    #[test]
    fn resolve_filename_no_conflict() {
        use std::path::PathBuf;
        // Use a non-existent directory — no conflict possible.
        let dir = PathBuf::from("/nonexistent_dir_abc123");
        let file = FileInfo {
            path: PathBuf::from("/src/photo.jpg"),
            size_bytes: 0,
            stem: "photo".into(),
            extension: "jpg".into(),
            filename: "photo.jpg".into(),
        };
        let name = resolve_destination_filename(&dir, &file);
        assert_eq!(name, "photo.jpg");
    }
}
