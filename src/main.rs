use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use owo_colors::OwoColorize;

mod exif;
mod models;
mod organizer;
mod scanner;
mod ui;

use models::Stats;

/// Image Messie — organizes image files into YYYY/MM_MonthName/CAMERA_MODEL/ directories
/// by reading EXIF metadata.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Source directory to scan recursively for images.
    #[arg(short, long)]
    path: PathBuf,

    /// Destination directory where organized images are placed. Defaults to current directory.
    #[arg(short, long, default_value = ".")]
    destination: PathBuf,

    /// Preview mode: show what would happen without copying any files.
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// Print extra details for each file.
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("Reading files from {}", args.path.display().bold());
    if args.dry_run {
        println!("{}", "┌─────────────────────────────────────┐".yellow());
        println!("{}", "│   DRY RUN — no files will be copied │".yellow().bold());
        println!("{}", "└─────────────────────────────────────┘".yellow());
    }

    let mut stats = Stats::default();

    for file in scanner::scan_images(&args.path) {
        stats.total_size_bytes += file.size_bytes;

        if args.verbose {
            println!(
                "{} {} (size: {} bytes)",
                "Processing".dimmed(),
                file.filename.dimmed(),
                file.size_bytes.dimmed()
            );
        }

        // ── Extract EXIF ─────────────────────────────────────────────────────
        let exif_result = exif::extract_exif(&file.path);

        let exif_data = match exif_result {
            Ok(data) => data,
            Err(e) => {
                let msg = e.to_string();
                // Detect our sentinel for invalid-but-parseable date.
                if msg.starts_with("InvalidDate:") {
                    let parts: Vec<&str> = msg.splitn(3, ':').collect();
                    let raw_date = parts.get(2).copied().unwrap_or("?");
                    eprintln!(
                        "{}",
                        format!(
                            "Invalid date '{}' in {}. Defaulting to 2000-01-01.",
                            raw_date, file.filename
                        )
                        .red()
                        .bold()
                    );
                    // exif::extract_exif returns Ok(data) inside the Err — unreachable here.
                    // We hit this branch only when the error truly propagates.
                    // Fall through to UNKNOWN.
                    stats.total_non_images += 1;
                    process_unknown(&file, &args, &mut stats);
                    continue;
                }
                eprintln!(
                    "{}",
                    format!(
                        "Error opening {}. Seems to be no image file.",
                        file.filename
                    )
                    .red()
                    .bold()
                );
                stats.total_non_images += 1;
                process_unknown(&file, &args, &mut stats);
                continue;
            }
        };

        // ── Determine target path ─────────────────────────────────────────────
        let (target_dir, is_unknown) = organizer::target_dir(&args.destination, &exif_data);

        if is_unknown {
            eprintln!(
                "{}",
                format!("No meta data extracted from {}.", file.filename)
                    .red()
                    .bold()
            );
            stats.total_non_images += 1;
        } else {
            stats.total_images += 1;
        }

        // ── Copy or dry-run log ───────────────────────────────────────────────
        let dest_filename = organizer::resolve_destination_filename(&target_dir, &file);

        if args.dry_run {
            println!(
                "Would copy {} → {}/{}",
                file.filename.cyan(),
                target_dir.display().cyan(),
                dest_filename.cyan()
            );
        } else {
            if args.verbose {
                println!(
                    "{}",
                    format!("Creating folder {} if it does not exist.", target_dir.display())
                        .dimmed()
                );
            }
            match organizer::copy_file(&file.path, &target_dir, &dest_filename) {
                Ok(dest_path) => {
                    println!(
                        "{}",
                        format!(
                            "📁 Copying {} to \"{}/{}\"",
                            file.filename,
                            target_dir.display(),
                            dest_filename
                        )
                        .green()
                        .bold()
                    );
                    let _ = dest_path;
                }
                Err(e) => {
                    eprintln!(
                        "{}",
                        format!("Failed to copy {}: {}", file.filename, e).red().bold()
                    );
                }
            }
        }
    }

    // ── Show ratatui stats panel ──────────────────────────────────────────────
    ui::show_stats(&stats, args.dry_run)?;

    Ok(())
}

/// Handle a file that could not be processed — copy to UNKNOWN/ (or log in dry-run).
fn process_unknown(
    file: &models::FileInfo,
    args: &Args,
    _stats: &mut Stats,
) {
    let target_dir = args.destination.join("UNKNOWN");
    let dest_filename = organizer::resolve_destination_filename(&target_dir, file);

    if args.dry_run {
        println!(
            "Would copy {} → {}/{}",
            file.filename.cyan(),
            target_dir.display().cyan(),
            dest_filename.cyan()
        );
    } else {
        match organizer::copy_file(&file.path, &target_dir, &dest_filename) {
            Ok(_) => println!(
                "{}",
                format!(
                    "📁 Copying {} to \"{}/{}\"",
                    file.filename,
                    target_dir.display(),
                    dest_filename
                )
                .green()
                .bold()
            ),
            Err(e) => eprintln!(
                "{}",
                format!("Failed to copy {}: {}", file.filename, e).red().bold()
            ),
        }
    }
}
