use std::path::PathBuf;

use clap::{ArgAction, Parser};

macro_rules! define_thread_defaults {
    ($hdd:literal, $ssd:literal) => {
        /// Maximum recommended threads when working on a mechanical HDD.
        pub const MAX_HDD_THREADS: usize = $hdd;

        /// Maximum recommanded threads when working on an SSD/NVMe.
        pub const MAX_SSD_THREADS: usize = $ssd;

        /// Help string displaying default thread limits for `clap`.
        pub const JOBS_HELP: &str = concat!(
            "Number of threads to use [default: auto-detected (",
            $hdd,
            " for HDD, ",
            $ssd,
            " for SSD)]"
        );
    };
}

define_thread_defaults!(1, 4);

/// CLI tool to inspect and fix ASS/SSA subtitle scaling parameters inside MKV containers.
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Automatically adjusts ScaledBorderAndShadow in ASS/SSA subtitles inside MKV files.",
    long_about = None
)]
pub struct Cli {
    /// Target MKV file or directory to process
    #[arg(value_name = "PATH")]
    pub path: PathBuf,

    /// Recursively traverse subdirectories when PATH is a directory
    #[arg(short, long)]
    pub recursive: bool,

    /// Verbosity level (-v for info, -vv for debug)
    #[arg(short, long, action = ArgAction::Count)]
    pub verbose: u8,

    /// Perform a dry run without modifying any files on disk
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    #[arg(short, long, value_name = "NUM", help = JOBS_HELP)]
    pub jobs: Option<usize>,
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn verify_cli_command() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parse_minimal_args() {
        let args = vec!["mkv-subscale", "/tmp/video.mkv"];
        let cli = Cli::try_parse_from(args).expect("Failed to parse minimal CLI args");

        assert_eq!(cli.path, PathBuf::from("/tmp/video.mkv"));
        assert!(!cli.recursive);
        assert_eq!(cli.verbose, 0);
        assert!(!cli.dry_run);
        assert_eq!(cli.jobs, None);
    }

    #[test]
    fn parse_all_flags_short() {
        let args = vec!["mkv-subscale", "-r", "-n", "-vv", "-j", "8", "movie.mkv"];
        let cli = Cli::try_parse_from(args).expect("Failed to parse short flags");

        assert_eq!(cli.path, PathBuf::from("movie.mkv"));
        assert!(cli.recursive);
        assert!(cli.dry_run);
        assert_eq!(cli.verbose, 2);
        assert_eq!(cli.jobs, Some(8));
    }

    #[test]
    fn parse_all_flags_long() {
        let args = vec![
            "mkv-subscale",
            "--recursive",
            "--dry-run",
            "--verbose",
            "--jobs",
            "2",
            "/path/to/dir",
        ];
        let cli = Cli::try_parse_from(args).expect("Failed to parse long flags");

        assert_eq!(cli.path, PathBuf::from("/path/to/dir"));
        assert!(cli.recursive);
        assert!(cli.dry_run);
        assert_eq!(cli.verbose, 1);
        assert_eq!(cli.jobs, Some(2));
    }

    #[test]
    fn constants_match_macro_expansion() {
        assert_eq!(MAX_HDD_THREADS, 1);
        assert_eq!(MAX_SSD_THREADS, 4);
        assert!(JOBS_HELP.contains("1 for HDD"));
        assert!(JOBS_HELP.contains("4 for SSD"));
    }
}
