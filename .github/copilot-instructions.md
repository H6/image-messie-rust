<!--
  This file is synced from the repository's CI-verified instructions for Copilot.
  Repo: H6/image-messie-rust
  Generated: 2026-05-25
-->

# Image Messie — Copilot Instructions

A CLI tool that organizes messy image collections into `YYYY/MM_MonthName/CAMERA_MODEL/` by reading EXIF metadata.
Binary name: `messie`.

## Build, test, lint

```powershell
cargo build --release          # Build
cargo test                     # Full suite
cargo test dsc_jpeg            # Run a single test by substring match
cargo test -- --nocapture      # Show println! output from tests
```

No dedicated lint command (`cargo clippy` is not configured in CI). CI runs `cargo test --verbose` on every push.

## Architecture

Six flat modules in `src/`, each doing one thing:

| File | Role |
|---|---|
| `main.rs` | CLI args (clap derive), orchestration loop, colored terminal output (owo-colors) |
| `models.rs` | `FileInfo`, `ExifData`, `Stats`, `ProcessOutcome`, `IMAGE_EXTENSIONS` constant |
| `scanner.rs` | Recursive file discovery via `walkdir`, filters by `IMAGE_EXTENSIONS`, yields `FileInfo` |
| `exif.rs` | EXIF extraction via `kamadak-exif`. Reads raw file, returns `Ok(ExifData)` even when no EXIF container exists (so the file routes to UNKNOWN/ instead of erroring out) |
| `organizer.rs` | Target-path construction (`target_dir`), duplicate filename resolution (`resolve_destination_filename`), file copy (`copy_file`) |
| `ui.rs` | Ratatui stats panel shown after processing; blocks until `q`/`Enter`/`Esc` |

No separate test files — tests live inline in `#[cfg(test)] mod tests` blocks within each source file.

## Key conventions

### Error handling philosophy
- `extract_exif` **never** returns `Err` for missing/malformed EXIF. It returns `Ok(ExifData::default())` (all fields `None`) so the file routes to `UNKNOWN/`. Only actual I/O errors (permission denied, file not found) become `Err`.
- Invalid dates that can't be parsed → fallback to `2000-01-01` with `date_warning: Some(raw_string)` set.

### Camera model normalization
Happens in `organizer::target_dir`, not during EXIF extraction:
```rust
let cam_folder = cam.to_uppercase().replace(' ', "_");
```
So `"NIKON D3300"` becomes `NIKON_D3300` in the directory name.

### Duplicate filename handling
When a file with the same name already exists at the destination, `resolve_destination_filename` appends a `_YYYYMMDDHHMMSS` timestamp suffix.

### HEIC/HEIF support
EXIF metadata is read directly via `kamadak-exif` — no native `libheif` required, no pixel decoding.

### Git workflows
- **CI** (`ci.yml`): runs `cargo test --verbose` on all pushes/PRs, Ubuntu only, with Cargo caching.
- **Release** (`release.yml`): triggers on `v*` tags, cross-compiles for linux/macos-x64/macos-arm64/windows-x64, uploads artifacts to GitHub Release.

### Test data
The `examples/DSC_0354.JPG` is a real Nikon D3300 JPEG used by tests. Expected EXIF values: year=2023, month=03, month_name=March, camera_model=NIKON D3300.
