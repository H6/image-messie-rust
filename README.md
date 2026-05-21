# image-messie-rust

<p align="center">
  <img src="image-messie.png" alt="Image Messie mascot — a Messie character surrounded by chaotic piles of stuff" width="300"/>
</p>

A fast, colorful CLI tool that organizes messy image collections into a clean folder hierarchy based on EXIF metadata. Rust port of [image-messie](https://github.com/H6/image-messie).

_Messie [ˈmɛsiː]: german word for a compulsive hoarder._

## What it does

Scans a source directory recursively and copies images into:

```
destination/
└── YYYY/
    └── MM_MonthName/
        └── CAMERA_MODEL/
            └── photo.jpg
```

Files without complete EXIF data (missing date or camera model) go into `destination/UNKNOWN/`.  
Duplicate filenames get a `_YYYYMMDDHHMMSS` suffix automatically.

After processing, a ratatui stats panel shows totals — press `q` or `Enter` to exit.

## Supported formats

`jpg` · `jpeg` · `heic` · `heif` · `png` · `tiff` · `tif` · `bmp` · `gif` · `webp`

> HEIC/HEIF files are fully supported: EXIF metadata (date, camera model) is read directly without pixel decoding — no native libheif required.

## Requirements

- [Rust toolchain](https://rustup.rs/) (edition 2021, stable)

## Build

```powershell
cd image-messie-rust
cargo build --release
```

The binary is placed at `target/release/messie.exe` (Windows) or `target/release/messie` (Linux/macOS).

## Usage

```
messie --path <SOURCE> [--destination <DEST>] [--dry-run] [--verbose]
```

| Flag | Short | Default | Description |
|---|---|---|---|
| `--path` | `-p` | *(required)* | Source directory to scan recursively |
| `--destination` | `-d` | `.` (current dir) | Target directory for organized output |
| `--dry-run` | | `false` | Preview what would happen — no files are copied |
| `--verbose` | `-v` | `false` | Print extra details for each file processed |

### Examples

**Dry-run preview:**
```powershell
.\target\release\messie.exe --path "D:\Downloads\Camera Roll" --destination "D:\Photos" --dry-run
```

**Organize for real:**
```powershell
.\target\release\messie.exe --path "D:\Downloads\Camera Roll" --destination "D:\Photos"
```

**Verbose output:**
```powershell
.\target\release\messie.exe --path "D:\Downloads\Camera Roll" --destination "D:\Photos" --verbose
```

**Organize into the current directory:**
```powershell
.\target\release\messie.exe --path "C:\Unsorted"
```

## Output

During processing, each file prints a colored line to the terminal:

- **Green** — successfully copied (or would copy in dry-run)
- **Red** — error opening file, missing EXIF, or invalid date

After all files are processed, a ratatui panel displays:

```
┌─ Image Messie — Results ──────────────────────────────┐
│  Total Images  │  Total Non-Images  │  Total Size (MB) │
│  1 234         │  56                │  4 821.33        │
└───────────────────────────────────────────────────────┘
  Press q or Enter to exit
```

## Project structure

```
src/
├── main.rs       # CLI args (clap), orchestration, colored output
├── models.rs     # FileInfo, ExifData, Stats, IMAGE_EXTENSIONS
├── scanner.rs    # Recursive file discovery (walkdir + extension filter)
├── exif.rs       # EXIF extraction via kamadak-exif (date, camera model, GPS)
├── organizer.rs  # Target path calculation, directory creation, file copy, duplicate suffix
└── ui.rs         # Ratatui stats panel shown after processing
```

## Run tests

Run the full test suite:

```powershell
cargo test
```

Run only a specific test by name (substring match):

```powershell
cargo test dsc_jpeg
```

Show `println!` output from passing tests:

```powershell
cargo test -- --nocapture
```

The unit tests in `src/exif.rs` use the real sample images in `examples/` — in particular `examples/DSC_0354.JPG` (a Nikon D3300 JPEG with full EXIF). The tests verify:

- Correct date extraction (`year = 2023`, `month = 03`, `month_name = March`)
- Correct camera model normalization (`NIKON_D3300`)
- Scanner discovery of `.JPG` files
- Correct target directory routing (`2023/03_March/NIKON_D3300`)

## Dependencies

| Crate | Purpose |
|---|---|
| `clap 4` | CLI argument parsing |
| `walkdir 2` | Recursive directory traversal |
| `kamadak-exif 0.5` | EXIF metadata extraction (JPEG, HEIC, TIFF) |
| `chrono 0.4` | Date parsing and formatting |
| `owo-colors 4` | Colored terminal output |
| `ratatui 0.30` | Final stats panel |
| `crossterm 0.29` | Terminal backend for ratatui |
| `anyhow 1` | Ergonomic error handling |
