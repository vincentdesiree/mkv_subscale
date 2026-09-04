use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use walkdir::WalkDir;

/// Discovers all candidate MKV files based on an input path and recursion flag.
///
/// # Errors
///
/// Returns an error if:
/// - The target `path` does not exist on the filesystem.
/// - The target `path` is a single file, but does not have an MKV extension.
/// - The target `path` exists but is neither a regular file nor a directory.
pub fn discover_mkv_files(path: &Path, recursive: bool) -> Result<Vec<PathBuf>> {
    if !path.exists() {
        bail!("Target path does not exist: {}", path.display());
    }

    let mut mkv_files = Vec::new();

    if path.is_file() {
        if is_mkv_file(path) {
            mkv_files.push(path.to_path_buf());
        } else {
            bail!(
                "The specified file is not an MKV container: {}",
                path.display()
            );
        }
    } else if path.is_dir() {
        let mut walker = WalkDir::new(path);
        if !recursive {
            walker = walker.max_depth(1);
        }

        for entry in walker.into_iter().filter_map(Result::ok) {
            let entry_path = entry.path();
            if entry_path.is_file() && is_mkv_file(entry_path) {
                mkv_files.push(entry_path.to_path_buf());
            }
        }
        mkv_files.sort();
    } else {
        bail!(
            "Path is neither a regular file nor a directory: {}",
            path.display()
        );
    }

    Ok(mkv_files)
}

/// Helper function to perform a case-insensitive check on the `.mkv` extension.
fn is_mkv_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("mkv"))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::fs::{self, File};

    #[test]
    fn test_is_mkv_file() {
        assert!(is_mkv_file(Path::new("video.mkv")));
        assert!(is_mkv_file(Path::new("video.MKV")));
        assert!(is_mkv_file(Path::new("/path/to/movie.Mkv")));
        assert!(!is_mkv_file(Path::new("video.mp4")));
        assert!(!is_mkv_file(Path::new("video_mkv")));
        assert!(!is_mkv_file(Path::new(".mkv")));
    }

    #[test]
    fn test_non_existent_path_returns_error() {
        let path = Path::new("does_not_exist_987654.mkv");
        let result = discover_mkv_files(path, false);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Target path does not exist")
        );
    }

    #[test]
    fn test_single_non_mkv_file_returns_error() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let txt_file = temp.path().join("document.txt");
        File::create(&txt_file)?;

        let result = discover_mkv_files(&txt_file, false);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not an MKV container")
        );
        Ok(())
    }

    #[test]
    fn test_single_mkv_file() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let mkv_file = temp.path().join("episode.mkv");
        File::create(&mkv_file)?;

        let files = discover_mkv_files(&mkv_file, false)?;
        assert_eq!(files, vec![mkv_file]);
        Ok(())
    }

    #[test]
    fn test_directory_non_recursive() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();

        let ep1 = root.join("ep1.mkv");
        let ep2 = root.join("ep2.MKV");
        let txt = root.join("notes.txt");
        let subfolder = root.join("season2");
        let ep_sub = subfolder.join("ep3.mkv");

        File::create(&ep1)?;
        File::create(&ep2)?;
        File::create(&txt)?;
        fs::create_dir(&subfolder)?;
        File::create(&ep_sub)?;

        let files = discover_mkv_files(root, false)?;

        assert_eq!(files, vec![ep1, ep2]);
        Ok(())
    }

    #[test]
    fn test_directory_recursive() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();

        let ep1 = root.join("b_ep1.mkv");
        let subfolder = root.join("season2");
        let ep_sub = subfolder.join("a_ep3.mkv");

        fs::create_dir(&subfolder)?;
        File::create(&ep1)?;
        File::create(&ep_sub)?;

        let files = discover_mkv_files(root, true)?;

        assert_eq!(files.len(), 2);
        assert_eq!(files[0], ep1);
        assert_eq!(files[1], ep_sub);
        Ok(())
    }
}
