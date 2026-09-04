use std::process::Command;

use anyhow::{Result, bail};

/// Verifies that all required external system tools are installed and available in PATH.
///
/// # Errors
///
/// Returns an error with actionable installation instructions if any required tool is missing.
pub fn ensure_dependencies() -> Result<()> {
    let required_tools = ["mkvmerge", "mkvextract"];
    let mut missing = Vec::new();

    for tool in required_tools {
        if !is_binary_available(tool) {
            missing.push(tool);
        }
    }

    if !missing.is_empty() {
        bail!(
            "Missing required system tool(s): {}\n\n\
            Please install MKVToolNix to use this CLI:\n  \
            - Ubuntu/Debian: sudo apt install mkvtoolnix\n  \
            - Arch Linux:    sudo pacman -S mkvtoolnix-cli\n  \
            - Fedora:        sudo dnf install mkvtoolnix\n  \
            - macOS:         brew install mkvtoolnix\n  \
            - Official site: https://mkvtoolnix.download/",
            missing.join(", ")
        );
    }

    Ok(())
}

/// Helper checking if a binary responds to `--version`.
fn is_binary_available(binary_name: &str) -> bool {
    Command::new(binary_name)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_binary_available_with_non_existent_binary() {
        assert!(!is_binary_available("non_existent_binary_987654321"));
    }

    #[test]
    fn test_is_binary_available_with_common_system_binary() {
        assert!(is_binary_available("cargo"));
    }
}
