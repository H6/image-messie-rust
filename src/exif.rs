use std::{fs::File, io::BufReader, path::Path};

use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use exif::{In, Reader, Tag, Value};

use crate::models::ExifData;

/// Extract EXIF metadata from the given image file.
///
/// Returns `Ok(ExifData)` in all cases where the file itself is readable:
/// - Full metadata found → all fields populated.
/// - No EXIF container (e.g. JFIF JPEG, PNG without EXIF) → all fields `None`.
/// - Invalid date field → date fields set to 2000-01-01 fallback, `date_warning` set.
///
/// Returns `Err` only for genuine IO errors (file not found, permission denied).
pub fn extract_exif(path: &Path) -> Result<ExifData> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut reader = BufReader::new(file);

    let exif = match Reader::new().read_from_container(&mut reader) {
        Ok(e) => e,
        Err(_) => {
            // File is readable but has no EXIF APP1 segment (e.g. JFIF, stripped JPEG).
            // Return empty ExifData — organizer routes it to UNKNOWN/.
            return Ok(ExifData::default());
        }
    };

    let mut data = ExifData::default();

    // ── DateTime ────────────────────────────────────────────────────────────
    if let Some(field) = exif.get_field(Tag::DateTime, In::PRIMARY) {
        let raw = field_to_string(&field.value);
        // EXIF DateTime format: "YYYY:MM:DD HH:MM:SS"
        match NaiveDateTime::parse_from_str(raw.trim(), "%Y:%m:%d %H:%M:%S") {
            Ok(dt) => {
                data.year = Some(dt.format("%Y").to_string());
                data.month = Some(dt.format("%m").to_string());
                data.day = Some(dt.format("%d").to_string());
                data.month_name = Some(dt.format("%B").to_string());
            }
            Err(_) => {
                // Mirrors Python: warn, default to 2000-01-01, continue processing.
                let fallback = chrono::NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
                data.year = Some("2000".into());
                data.month = Some("01".into());
                data.day = Some("01".into());
                data.month_name = Some(fallback.format("%B").to_string());
                data.date_warning = Some(raw.trim().to_string());
            }
        }
    }

    // ── Camera Model ─────────────────────────────────────────────────────────
    if let Some(field) = exif.get_field(Tag::Model, In::PRIMARY) {
        let raw = field_to_string(&field.value);
        data.camera_model = Some(raw.trim().to_string());
    }

    // ── GPS (parsed but not used for organization) ───────────────────────────
    if let Some(field) = exif.get_field(Tag::GPSLatitude, In::PRIMARY) {
        data.gps_lat = Some(field_to_string(&field.value));
    }
    if let Some(field) = exif.get_field(Tag::GPSLongitude, In::PRIMARY) {
        data.gps_lon = Some(field_to_string(&field.value));
    }

    Ok(data)
}

fn field_to_string(value: &Value) -> String {
    match value {
        Value::Ascii(vec) => vec
            .iter()
            .filter_map(|bytes| std::str::from_utf8(bytes).ok())
            .collect::<Vec<_>>()
            .join(""),
        other => format!("{other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ExifData;
    use crate::organizer::target_dir;
    use crate::scanner::scan_images;
    use std::path::Path;

    #[test]
    fn organizer_normalizes_camera_model_for_directory() {
        // Normalization (spaces → underscores, uppercase) now happens in organizer::target_dir.
        let exif = ExifData {
            year: Some("2024".into()),
            month: Some("01".into()),
            month_name: Some("January".into()),
            camera_model: Some("Canon EOS R5".into()),
            ..ExifData::default()
        };
        let (dir, is_unknown) = target_dir(Path::new("/dst"), &exif);
        assert!(!is_unknown);
        assert_eq!(dir, Path::new("/dst/2024/01_January/CANON_EOS_R5"));
    }

    /// A non-image file (e.g. Cargo.toml) has no EXIF container.
    /// `extract_exif` must return `Ok` with all-None fields rather than `Err`,
    /// so such files would be routed to UNKNOWN/ rather than treated as unreadable.
    #[test]
    fn file_without_exif_returns_empty_ok() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        assert!(path.exists(), "Cargo.toml not found at {}", path.display());

        let result = extract_exif(&path);

        assert!(
            result.is_ok(),
            "Expected Ok for a readable file with no EXIF, but got Err({:?})",
            result.unwrap_err()
        );

        let exif = result.unwrap();
        assert!(
            exif.year.is_none()
                && exif.month.is_none()
                && exif.camera_model.is_none()
                && exif.date_warning.is_none(),
            "Expected all-None ExifData for a file with no EXIF, got: {exif:?}"
        );
    }

    /// When EXIF extraction fails the organizer must route the file to UNKNOWN/.
    #[test]
    fn example_jpeg_without_exif_routes_to_unknown() {
        // Simulate what main.rs does when extract_exif returns Err:
        // use a default (all-None) ExifData.
        let (dir, is_unknown) = target_dir(Path::new("/dst"), &ExifData::default());

        assert!(is_unknown, "Missing EXIF should route to UNKNOWN/");
        assert_eq!(dir, Path::new("/dst/UNKNOWN"));
    }

    /// The scanner must discover all images in the examples/ directory.
    /// Only DSC_0354.JPG is present; verify the total count matches.
    #[test]
    fn examples_dir_contains_exactly_one_image() {
        let examples_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
        assert!(
            examples_dir.exists(),
            "examples/ directory not found at {}",
            examples_dir.display()
        );

        let found: Vec<_> = scan_images(&examples_dir).collect();

        assert_eq!(found.len(), 1, "Expected exactly one image in examples/");
        assert_eq!(found[0].filename, "DSC_0354.JPG");
    }

    fn dsc_image_path() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples")
            .join("DSC_0354.JPG")
    }

    /// DSC_0354.JPG is a real Nikon JPEG with full EXIF data.
    /// Verify that `extract_exif` returns the expected date and camera model.
    #[test]
    fn dsc_jpeg_exif_fields_are_correct() {
        let path = dsc_image_path();
        assert!(path.exists(), "example image not found at {}", path.display());

        let exif = extract_exif(&path)
            .expect("extract_exif should succeed for a valid JPEG with EXIF");

        assert_eq!(exif.year.as_deref(), Some("2023"), "year mismatch");
        assert_eq!(exif.month.as_deref(), Some("03"), "month mismatch");
        assert_eq!(exif.month_name.as_deref(), Some("March"), "month_name mismatch");
        assert_eq!(
            exif.camera_model.as_deref(),
            Some("NIKON D3300"),
            "camera_model mismatch"
        );
        assert!(
            exif.date_warning.is_none(),
            "no date_warning expected for a well-formed EXIF DateTime"
        );
    }

    /// DSC_0354.JPG must be picked up by the scanner (`.JPG` extension).
    #[test]
    fn dsc_jpeg_is_discovered_by_scanner() {
        let examples_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
        assert!(
            examples_dir.exists(),
            "examples/ directory not found at {}",
            examples_dir.display()
        );

        let found: Vec<_> = scan_images(&examples_dir)
            .filter(|f| f.filename == "DSC_0354.JPG")
            .collect();

        assert_eq!(found.len(), 1, "Scanner should find exactly one DSC_0354.JPG");

        let file = &found[0];
        assert_eq!(file.extension, "jpg");
        assert_eq!(file.stem, "dsc_0354");
        assert!(file.size_bytes > 0, "File size should be non-zero");
    }

    /// DSC_0354.JPG with full EXIF must be routed to 2023/03_March/NIKON_D3300/.
    #[test]
    fn dsc_jpeg_routes_to_correct_target_dir() {
        let path = dsc_image_path();
        assert!(path.exists(), "example image not found at {}", path.display());

        let exif = extract_exif(&path).expect("extract_exif should succeed");
        let (dir, is_unknown) = target_dir(Path::new("/dst"), &exif);

        assert!(!is_unknown, "DSC_0354.JPG should not route to UNKNOWN/");
        assert_eq!(
            dir,
            Path::new("/dst/2023/03_March/NIKON_D3300"),
            "unexpected target directory"
        );
    }
}
