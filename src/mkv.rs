use std::path::Path;
use std::process::Command;

use serde::Deserialize;
use tempfile::NamedTempFile;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MkvError {
    #[error("Failed to execute `{command}`: {source}")]
    Execution {
        command: &'static str,
        source: std::io::Error,
    },

    #[error("`{command}` process returned an error status: {stderr}")]
    CommandFailed {
        command: &'static str,
        stderr: String,
    },

    #[error("Failed to parse JSON output from `mkvmerge`: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Failed to create temporary file: {0}")]
    TempFile(#[source] std::io::Error),
}

#[derive(Debug, Deserialize)]
struct MkvMergeOutput {
    #[serde(default)]
    tracks: Vec<Track>,
}

#[derive(Debug, Deserialize)]
struct Track {
    id: u64,
    r#type: String,
    properties: TrackProperties,
}

#[derive(Debug, Deserialize)]
struct TrackProperties {
    codec_id: Option<String>,
    track_name: Option<String>,
    language: Option<String>,
    language_ietf: Option<String>,
}

/// Represents an ASS/SSA subtitle track inside an MKV container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssTrack {
    pub id: u64,
    pub codec_id: String,
    pub name: Option<String>,
    pub language: String,
}

/// Structure specifying a modified track replacement.
pub struct ModifiedTrack<'a> {
    pub id: u64,
    pub ass_path: &'a Path,
    pub language: &'a str,
    pub name: Option<&'a str>,
}

/// Inspects an MKV file using `mkvmerge -J` and extracts relevant ASS/SSA subtitle tracks.
///
/// # Errors
///
/// Returns [`MkvError::Execution`] if `mkvmerge` cannot be spawned (e.g., binary not found).
/// Returns [`MkvError::CommandFailed`] if `mkvmerge` exits with a non-zero status code.
/// Returns [`MkvError::JsonParse`](serde_json::Error) if parsing stdout fails.
pub fn inspect_mkv_subtitles(mkv_path: &Path) -> Result<Vec<AssTrack>, MkvError> {
    let output = Command::new("mkvmerge")
        .arg("-J")
        .arg(mkv_path)
        .output()
        .map_err(|source| MkvError::Execution {
            command: "mkvmerge",
            source,
        })?;

    if !output.status.success() {
        return Err(MkvError::CommandFailed {
            command: "mkvmerge",
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    parse_mkvmerge_output(&output.stdout)
}

/// Parses raw JSON output from `mkvmerge -J` to extract ASS/SSA subtitle tracks.
///
/// # Errors
///
/// Returns [`MkvError::JsonParse`](serde_json::Error) if parsing stdout fails.
fn parse_mkvmerge_output(json_bytes: &[u8]) -> Result<Vec<AssTrack>, MkvError> {
    let parsed: MkvMergeOutput = serde_json::from_slice(json_bytes)?;

    let ass_tracks = parsed
        .tracks
        .into_iter()
        .filter(|track| track.r#type == "subtitles")
        .filter_map(|track| {
            let codec = track.properties.codec_id.as_deref()?;

            if codec == "S_TEXT/ASS" || codec == "S_TEXT/SSA" {
                let lang = track
                    .properties
                    .language_ietf
                    .or(track.properties.language)
                    .unwrap_or_else(|| "und".to_string());

                Some(AssTrack {
                    id: track.id,
                    codec_id: codec.to_string(),
                    name: track.properties.track_name,
                    language: lang,
                })
            } else {
                None
            }
        })
        .collect();

    Ok(ass_tracks)
}

/// Extracts a specific track ID to a temporary `.ass` file.
///
/// # Errors
///
/// Returns [`MkvError::TempFile`] if the temporary file cannot be created.
/// Returns [`MkvError::Execution`] if `mkvextract` fails to launch.
/// Returns [`MkvError::CommandFailed`] if `mkvextract` exits with an error status.
pub fn extract_subtitle_track(mkv_path: &Path, track_id: u64) -> Result<NamedTempFile, MkvError> {
    let temp_file = NamedTempFile::new().map_err(MkvError::TempFile)?;

    let output = Command::new("mkvextract")
        .arg(mkv_path)
        .arg("tracks")
        .arg(format!("{}:{}", track_id, temp_file.path().display()))
        .output()
        .map_err(|source| MkvError::Execution {
            command: "mkvextract",
            source,
        })?;

    if !output.status.success() {
        return Err(MkvError::CommandFailed {
            command: "mkvextract",
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    Ok(temp_file)
}

/// Repacks an MKV file by replacing specific subtitle tracks with modified ASS files.
///
/// # Errors
///
/// Returns [`MkvError::TempFile`] if creation of the temporary output file fails.
/// Returns [`MkvError::Execution`] if `mkvmerge` cannot be spawned.
/// Returns [`MkvError::CommandFailed`] if `mkvmerge` exits with an unhandled exit code.
/// Returns [`MkvError::Execution`] if atomic replacement (`persist`) of the destination file fails.
pub fn repack_mkv(mkv_path: &Path, modified_tracks: &[ModifiedTrack]) -> Result<(), MkvError> {
    if modified_tracks.is_empty() {
        return Ok(());
    }

    let parent_dir = mkv_path.parent().unwrap_or_else(|| Path::new("."));
    let temp_mkv = tempfile::Builder::new()
        .prefix(".mkv_subscale_")
        .suffix(".mkv")
        .tempfile_in(parent_dir)
        .map_err(MkvError::TempFile)?;

    let temp_mkv_path = temp_mkv.path();

    let excluded_ids: Vec<String> = modified_tracks.iter().map(|t| t.id.to_string()).collect();
    let exclude_arg = format!("!{}", excluded_ids.join(","));

    let mut cmd = Command::new("mkvmerge");
    cmd.arg("-o")
        .arg(temp_mkv_path)
        .arg("--subtitle-tracks")
        .arg(&exclude_arg)
        .arg(mkv_path);

    for track in modified_tracks {
        cmd.arg("--language").arg(format!("0:{}", track.language));
        if let Some(name) = track.name {
            cmd.arg("--track-name").arg(format!("0:{name}"));
        }
        cmd.arg(track.ass_path);
    }

    let output = cmd.output().map_err(|source| MkvError::Execution {
        command: "mkvmerge",
        source,
    })?;

    if !output.status.success() && output.status.code() != Some(1) {
        return Err(MkvError::CommandFailed {
            command: "mkvmerge",
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    temp_mkv
        .persist(mkv_path)
        .map_err(|err| MkvError::Execution {
            command: "persist_mkv",
            source: err.error,
        })?;

    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mkvmerge_json_valid_ass_and_ssa() {
        let json = serde_json::json!({
            "tracks": [
                {
                    "id": 0,
                    "type": "video",
                    "properties": { "codec_id": "V_MPEG4/ISO/AVC" }
                },
                {
                    "id": 1,
                    "type": "subtitles",
                    "properties": {
                        "codec_id": "S_TEXT/ASS",
                        "track_name": "Full Subs",
                        "language_ietf": "fr-FR",
                        "language": "fre"
                    }
                },
                {
                    "id": 2,
                    "type": "subtitles",
                    "properties": {
                        "codec_id": "S_TEXT/SSA",
                        "language": "eng"
                    }
                },
                {
                    "id": 3,
                    "type": "subtitles",
                    "properties": {
                        "codec_id": "S_TEXT/UTF8",
                        "language": "fre"
                    }
                }
            ]
        });

        let json_bytes = serde_json::to_vec(&json).expect("Serialization failed");
        let tracks = parse_mkvmerge_output(&json_bytes).expect("Parsing failed");

        assert_eq!(tracks.len(), 2);

        assert_eq!(
            tracks[0],
            AssTrack {
                id: 1,
                codec_id: "S_TEXT/ASS".to_string(),
                name: Some("Full Subs".to_string()),
                language: "fr-FR".to_string(),
            }
        );

        assert_eq!(
            tracks[1],
            AssTrack {
                id: 2,
                codec_id: "S_TEXT/SSA".to_string(),
                name: None,
                language: "eng".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_mkvmerge_json_missing_language_defaults_to_und() {
        let json = serde_json::json!({
            "tracks": [
                {
                    "id": 0,
                    "type": "subtitles",
                    "properties": {
                        "codec_id": "S_TEXT/ASS"
                    }
                }
            ]
        });

        let json_bytes = serde_json::to_vec(&json).expect("Serialization failed");
        let tracks = parse_mkvmerge_output(&json_bytes).expect("Parsing failed");

        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].language, "und");
    }

    #[test]
    fn test_parse_mkvmerge_json_invalid_structure() {
        let invalid_json = b"{ \"invalid\": true }";
        let tracks = parse_mkvmerge_output(invalid_json).expect("Empty tracks fallback");
        assert!(tracks.is_empty());
    }
}
