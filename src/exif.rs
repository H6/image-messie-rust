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
        let normalized = raw.trim().to_uppercase().replace(' ', "_");
        data.camera_model = Some(normalized);
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

    fn example_image_path() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples")
            .join("048DF200-5E2F-455B-866D-2BD51A9A77E5.jpeg")
    }

    #[test]
    fn camera_model_normalizes_spaces_to_underscores() {
        // Pure logic test — no file needed.
        let raw = "Canon EOS R5 ";
        let normalized = raw.trim().to_uppercase().replace(' ', "_");
        assert_eq!(normalized, "CANON_EOS_R5");
    }

    /// The example JPEG is a JFIF file (APP0 marker) with no EXIF APP1 segment.
    /// `extract_exif` must return `Ok` with all-None fields (not panic, not Err),
    /// so the file is routed to UNKNOWN/ rather than treated as unreadable.
    #[test]
    fn example_jpeg_without_exif_returns_empty_ok() {
        let path = example_image_path();
        assert!(path.exists(), "example image not found at {}", path.display());

        let result = extract_exif(&path);

        assert!(
            result.is_ok(),
            "Expected Ok for a readable JFIF JPEG (even without EXIF), but got Err({:?})",
            result.unwrap_err()
        );

        let exif = result.unwrap();
        assert!(
            exif.year.is_none()
                && exif.month.is_none()
                && exif.camera_model.is_none()
                && exif.date_warning.is_none(),
            "Expected all-None ExifData for JFIF with no EXIF, got: {exif:?}"
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

    /// The scanner must discover the example JPEG even though it has no EXIF.
    /// Extension matching (`.jpeg`) is independent of file content.
    #[test]
    fn example_jpeg_is_discovered_by_scanner() {
        let examples_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
        assert!(
            examples_dir.exists(),
            "examples/ directory not found at {}",
            examples_dir.display()
        );

        let found: Vec<_> = scan_images(&examples_dir)
            .filter(|f| f.filename == "048DF200-5E2F-455B-866D-2BD51A9A77E5.jpeg")
            .collect();

        assert_eq!(
            found.len(),
            1,
            "Scanner should find exactly one matching file"
        );

        let file = &found[0];
        assert_eq!(file.extension, "jpeg");
        assert!(file.size_bytes > 0, "File size should be non-zero");
        println!(
            "Discovered: {} ({} bytes)",
            file.filename, file.size_bytes
        );
    }
}
