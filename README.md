# mkv_subscale

[![CI Status](https://github.com/vincentdesiree/mkv_subscale/actions/workflows/ci.yml/badge.svg)](https://github.com/vincentdesiree/mkv_subscale/actions/workflows/ci.yml)
[![GitHub Release](https://img.shields.io/github/v/release/vincentdesiree/mkv_subscale?include_prereleases&sort=semver&color=blue)](https://github.com/vincentdesiree/mkv_subscale/releases/latest)
[![License](https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-blue.svg)](https://github.com/vincentdesiree/mkv_subscale#license)
[![Rust Edition](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org/)
[![Downloads](https://img.shields.io/github/downloads/vincentdesiree/mkv_subscale/total?color=green)](https://github.com/vincentdesiree/mkv_subscale/releases)

`mkv_subscale` is a fast and reliable CLI utility written in Rust to inspect and automatically fix ASS/SSA subtitle scaling parameters (`ScaledBorderAndShadow: yes`) inside MKV containers.

By default, some media players render ASS/SSA subtitle borders and shadows out of proportion when the video resolution differs from the script's target resolution. This tool ensures the `ScaledBorderAndShadow` directive is properly set to `yes` in the `[Script Info]` section of all relevant subtitle tracks.

---

## 🚀 Features

- 🔍 **Automatic Inspection**: Identifies ASS (`S_TEXT/ASS`) and SSA (`S_TEXT/SSA`) subtitle tracks while leaving video, audio, and other subtitle formats (e.g., SRT) untouched.
- ⚙️ **Smart Patching**: Sets `ScaledBorderAndShadow: yes` only if missing or set to non-compliant values (`no`, `0`).
- ⚡ **Parallel Processing**: Leverages [Rayon](https://crates.io/crates/rayon) for concurrent multi-file processing.
- 💾 **I/O Storage Aware (Linux)**: Automatically detects mechanical HDDs vs. SSDs/NVMes to adjust concurrency and avoid head thrashing.
- 🛡️ **Atomic & Safe File Operations**: Uses temporary files located on the target file's filesystem for atomic replacements, preventing data corruption or cross-device mount errors (`EXDEV`).
- 🧪 **Dry-Run Mode (`--dry-run`)**: Preview tracks that require modification without altering any files on disk.

---

## 📋 Prerequisites

`mkv_subscale` relies on **MKVToolNix** utilities for extracting and remuxing MKV containers.

The following binaries must be installed and available in your `PATH`:
- `mkvmerge`
- `mkvextract`

### Installing MKVToolNix

#### Linux
```bash
# Ubuntu / Debian
sudo apt install mkvtoolnix

# Arch Linux
sudo pacman -S mkvtoolnix-cli

# Fedora
sudo dnf install mkvtoolnix
```

#### macOS
```bash
brew install mkvtoolnix
```

---

## 🛠️ Installation

### Pre-built Binaries (No Cargo required)

1. Download the latest release archive for your system from [GitHub Releases](https://github.com/vincentdesiree/mkv_subscale/releases/latest).
2. Extract the archive and make the binary executable:

```bash
# Example for Linux x86_64
tar -xzf mkv_subscale-x86_64-unknown-linux-gnu.tar.gz
chmod +x mkv_subscale
sudo mv mkv_subscale /usr/local/bin/
```

### Direct install script (Linux / macOS)

```bash
curl -sSL https://raw.githubusercontent.com/vincentdesiree/mkv_subscale/main/install.sh | bash
```

---

### From Source (Requires Cargo)

```bash
cargo install --git https://github.com/vincentdesiree/mkv_subscale.git
```

---

## 📖 Usage

### Basic Syntax

```bash
mkv_subscale [OPTIONS] <PATH>
```

### Examples

#### 1. Process a single MKV file
```bash
mkv_subscale /path/to/episode.mkv
```

#### 2. Process a directory recursively
```bash
mkv_subscale -r /path/to/tv_show/
```

#### 3. Run in Dry-Run mode
Simulate the run and show files that need modifications without modifying them:
```bash
mkv_subscale --dry-run /path/to/tv_show/
```

---

## 🏗️ Architecture & Data Safety

1. **Dependency Verification**: Verifies system availability of `mkvmerge` and `mkvextract` at startup.
2. **Storage Topology Detection**: Inspects `/proc/mounts` and `/sys/block/*/queue/rotational` (Linux) to tune threadpool size dynamically.
3. **Extraction & Patching**:
   - Extracts ASS tracks to isolated temporary files managed by `tempfile`.
   - Modifies `[Script Info]` content in memory, correctly handling UTF-8 BOM markers and key case-insensitivity.
4. **Atomic Remux**: Rebuilds the `.mkv` container adjacent to the original file on the same mount point and replaces it atomically via `persist()`.

---

## 🧪 Testing & Linting

Run unit tests:

```bash
cargo test
```

Check code compliance with pedantic Clippy lints:

```bash
cargo clippy -- -D warnings -W clippy::pedantic
```

---

## 📄 License

Dual-licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
