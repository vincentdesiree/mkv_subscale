use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use crate::{
    mkv::{ModifiedTrack, extract_subtitle_track, inspect_mkv_subtitles, repack_mkv},
    subtitles::{SubtitleOutcome, process_ass_content},
};

/// Fixed set of reasons why an MKV file processing was skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    NoAssTracks,
    AlreadyCompliant,
}

impl fmt::Display for SkipReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAssTracks => write!(f, "No ASS/SSA tracks found"),
            Self::AlreadyCompliant => write!(f, "Already compliant"),
        }
    }
}

/// Outcome of processing an individual MKV file.
#[derive(Debug)]
pub enum FileOutcome {
    Skipped(SkipReason),
    Modified { tracks_fixed: usize },
    Failed(anyhow::Error),
}

/// Summary report for a single processed file.
pub struct FileReport {
    pub path: PathBuf,
    pub outcome: FileOutcome,
}

/// Entry point executed for each individual file (e.g. inside a Rayon thread).
#[must_use]
pub fn process_file(path: &Path, dry_run: bool) -> FileReport {
    let outcome = run_pipeline(path, dry_run).unwrap_or_else(FileOutcome::Failed);

    FileReport {
        path: path.to_path_buf(),
        outcome,
    }
}

/// Internal execution pipeline returning `Result` for error propagation with `?`.
fn run_pipeline(path: &Path, dry_run: bool) -> Result<FileOutcome> {
    let tracks = inspect_mkv_subtitles(path)
        .with_context(|| format!("Failed to inspect subtitles for {}", path.display()))?;

    if tracks.is_empty() {
        return Ok(FileOutcome::Skipped(SkipReason::NoAssTracks));
    }

    let mut modified_handles = Vec::new();

    for track in tracks {
        let temp_file = extract_subtitle_track(path, track.id).with_context(|| {
            format!(
                "Failed to extract track {} from {}",
                track.id,
                path.display()
            )
        })?;

        let content = fs::read_to_string(temp_file.path()).with_context(|| {
            format!(
                "Failed to read extracted ASS content for track {}",
                track.id
            )
        })?;

        if let SubtitleOutcome::Modified { new_content } = process_ass_content(&content) {
            fs::write(temp_file.path(), new_content).with_context(|| {
                format!("Failed to write updated ASS content for track {}", track.id)
            })?;

            modified_handles.push((temp_file, track.id, track.language, track.name));
        }
    }

    if modified_handles.is_empty() {
        return Ok(FileOutcome::Skipped(SkipReason::AlreadyCompliant));
    }

    let tracks_fixed = modified_handles.len();

    if !dry_run {
        let modified_tracks: Vec<ModifiedTrack> = modified_handles
            .iter()
            .map(|(temp_file, id, language, name)| ModifiedTrack {
                id: *id,
                ass_path: temp_file.path(),
                language,
                name: name.as_deref(),
            })
            .collect();

        repack_mkv(path, &modified_tracks)
            .with_context(|| format!("Failed to repack MKV file {}", path.display()))?;
    }

    Ok(FileOutcome::Modified { tracks_fixed })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_skip_reason_display() {
        assert_eq!(
            SkipReason::NoAssTracks.to_string(),
            "No ASS/SSA tracks found"
        );
        assert_eq!(
            SkipReason::AlreadyCompliant.to_string(),
            "Already compliant"
        );
    }

    #[test]
    fn test_process_non_existent_file_returns_failed_outcome() {
        let dummy_path = Path::new("non_existent_file_12345.mkv");
        let report = process_file(dummy_path, true);

        assert_eq!(report.path, dummy_path);
        match report.outcome {
            FileOutcome::Failed(err) => {
                let err_msg = err.to_string();
                assert!(
                    err_msg.contains("Failed to inspect subtitles"),
                    "Unexpected error message: {err_msg}"
                );
            }
            _ => panic!("Expected FileOutcome::Failed for non-existent file"),
        }
    }
}
