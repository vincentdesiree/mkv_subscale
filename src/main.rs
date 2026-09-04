use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::{Error, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use log::{LevelFilter, debug, info};
use rayon::prelude::*;

use mkv_subscale::{
    cli::{Cli, MAX_HDD_THREADS, MAX_SSD_THREADS},
    deps,
    processor::{FileOutcome, SkipReason, process_file},
    scanner::discover_mkv_files,
    storage::is_rotational_hdd,
};

fn main() -> Result<()> {
    deps::ensure_dependencies()?;

    let args = Cli::parse();

    set_env_logger(&args);

    let start_time = Instant::now();

    set_threads(&args)?;

    if args.dry_run {
        info!("Dry-run mode enabled (-n): no files will be modified on disk.");
    }

    debug!(
        "Scanning path: {} (recursive: {})",
        args.path.display(),
        args.recursive
    );

    let files = discover_mkv_files(&args.path, args.recursive)?;

    if files.is_empty() {
        println!("No MKV files found in target path: {}", args.path.display());
        return Ok(());
    }

    info!("Found {} MKV file(s) to process", files.len());

    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:30.cyan/blue}] {pos}/{len} ({percent}%) | ETA: {eta} | {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_bar())
            .progress_chars("=>-")
    );

    let reports: Vec<_> = files
        .par_iter()
        .map(|path| {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                pb.set_message(file_name.to_string());
            }
            let res = process_file(path, args.dry_run);
            pb.inc(1);
            res
        })
        .collect();

    pb.finish_and_clear();

    let mut modified_count: usize = 0;
    let mut skipped_no_ass: usize = 0;
    let mut skipped_compliant: usize = 0;
    let mut failures = Vec::new();

    for report in reports {
        match report.outcome {
            FileOutcome::Modified { tracks_fixed } => {
                modified_count += 1;
                info!(
                    "Modified: {} ({} track(s) fixed)",
                    report.path.display(),
                    tracks_fixed
                );
            }
            FileOutcome::Skipped(reason) => {
                debug!("Skipped {}: {}", report.path.display(), reason);
                match reason {
                    SkipReason::NoAssTracks => skipped_no_ass += 1,
                    SkipReason::AlreadyCompliant => skipped_compliant += 1,
                }
            }
            FileOutcome::Failed(err) => {
                failures.push((report.path, err));
            }
        }
    }

    if !failures.is_empty() {
        println!("\n❌ Errors encountered ({} file(s)):", failures.len());
        for (path, err) in &failures {
            println!("   - {}: {:#}", path.display(), err);
        }
    }

    let elapsed = start_time.elapsed();

    display_summary_report(
        files.len(),
        modified_count,
        skipped_compliant,
        skipped_no_ass,
        &failures,
        elapsed,
    );

    Ok(())
}

fn set_env_logger(args: &Cli) {
    let log_level = match args.verbose {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        _ => LevelFilter::Debug,
    };

    env_logger::Builder::new()
        .filter_level(log_level)
        .format_timestamp(None)
        .init();
}

fn set_threads(args: &Cli) -> Result<()> {
    let threads = args.jobs.map_or_else(
        || auto_detect_threads(&args.path),
        |n| {
            info!("Using manual thread override: {n}");
            n
        },
    );

    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build_global()?;

    Ok(())
}

fn auto_detect_threads(path: &Path) -> usize {
    if is_rotational_hdd(path) {
        info!(
            "Mechanical HDD detected at '{}'. Limiting to {MAX_HDD_THREADS} thread to prevent disk thrashing.",
            path.display()
        );
        MAX_HDD_THREADS
    } else {
        info!(
            "Solid-state storage (SSD/NVMe) detected at '{}'. Using {MAX_SSD_THREADS} threads.",
            path.display()
        );
        MAX_SSD_THREADS
    }
}

fn display_summary_report(
    total_scanned: usize,
    modified_count: usize,
    skipped_compliant: usize,
    skipped_no_ass: usize,
    failures: &[(PathBuf, Error)],
    elapsed: Duration,
) {
    println!("\n--- Execution Summary ---");
    println!("  Total scanned     : {total_scanned}");
    println!("  Modified          : {modified_count}");
    println!("  Already compliant : {skipped_compliant}");
    println!("  No ASS tracks     : {skipped_no_ass}");
    println!("  Failed            : {}", failures.len());
    println!("  Elapsed time      : {elapsed:.2?}");
}
