use std::path::PathBuf;

/// Extensions considered image files (lowercase).
pub const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "heic", "heif", "png", "tiff", "tif", "bmp", "gif", "webp",
];

/// Information about a discovered file on disk.
#[derive(Debug)]
pub struct FileInfo {
    pub path: PathBuf,
    pub size_bytes: u64,
    /// Lower-cased stem (filename without extension).
    pub stem: String,
    /// Lower-cased extension without leading dot.
    pub extension: String,
    /// Original filename (stem + extension with dot).
    pub filename: String,
}

/// EXIF metadata extracted from an image file.
#[derive(Debug, Default)]
pub struct ExifData {
    pub year: Option<String>,
    /// Zero-padded two-digit month, e.g. "03".
    pub month: Option<String>,
    pub day: Option<String>,
    /// English month name, e.g. "March".
    pub month_name: Option<String>,
    /// Normalized camera model: UPPERCASE_WITH_UNDERSCORES.
    pub camera_model: Option<String>,
    pub gps_lat: Option<String>,
    pub gps_lon: Option<String>,
    /// Set when the DateTime field was present but could not be parsed.
    /// Contains the raw invalid date string. The date fields are filled
    /// with the 2000-01-01 fallback when this is Some.
    pub date_warning: Option<String>,
}

/// Outcome of processing one file — kept for future use.
#[allow(dead_code)]
#[derive(Debug)]
pub enum ProcessOutcome {
    /// File was copied (or would be, in dry-run) to the given destination path.
    Copied {
        destination: PathBuf,
    },
    /// File was skipped — not an image or EXIF could not be read.
    Skipped {
        reason: String,
    },
    /// File was placed in UNKNOWN/ due to missing metadata.
    Unknown {
        destination: PathBuf,
        reason: String,
    },
}

/// Accumulated statistics over the entire run.
#[derive(Debug, Default)]
pub struct Stats {
    pub total_images: u64,
    pub total_non_images: u64,
    pub total_size_bytes: u64,
}

impl Stats {
    pub fn total_size_mb(&self) -> f64 {
        (self.total_size_bytes as f64) / (1024.0 * 1024.0)
    }
}
