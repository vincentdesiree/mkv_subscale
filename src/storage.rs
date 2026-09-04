use std::fs;
use std::path::{Path, PathBuf};

/// Determines whether the specified filesystem path resides on a rotational storage device (HDD).
///
/// On Linux, this function resolves the backing block device for `target_path` via `/proc/mounts`,
/// then reads `/sys/block/<device>/queue/rotational`.
///
/// # Returns
/// - `true` if the underlying device is a mechanical HDD (`rotational == 1`).
/// - `false` if the device is non-rotational (SSD, `NVMe`, RAM/virtual FS, non-Linux OS), or if detection fails.
#[must_use]
pub fn is_rotational_hdd(target_path: &Path) -> bool {
    let canonical = fs::canonicalize(target_path).unwrap_or_else(|_| target_path.to_path_buf());

    let Ok(mounts) = fs::read_to_string("/proc/mounts") else {
        return false;
    };

    let Some(dev_path) = parse_best_mount(&mounts, &canonical) else {
        return false;
    };

    let dev_name = dev_path.trim_start_matches("/dev/");
    let base_dev = extract_base_block_device(dev_name);

    let sys_path = PathBuf::from(format!("/sys/block/{base_dev}/queue/rotational"));
    if let Ok(content) = fs::read_to_string(sys_path) {
        return content.trim() == "1";
    }

    false
}

/// Finds the most specific mount point in `/proc/mounts` matching `target_path`.
fn parse_best_mount<'a>(mounts_content: &'a str, target_path: &Path) -> Option<&'a str> {
    let mut best_match: Option<(&str, &str)> = None;

    for line in mounts_content.lines() {
        let mut parts = line.split_whitespace();
        if let (Some(dev), Some(mount_point)) = (parts.next(), parts.next()) {
            let mount_path = Path::new(mount_point);
            if target_path.starts_with(mount_path) {
                match best_match {
                    Some((_, best_mount)) if mount_point.len() > best_mount.len() => {
                        best_match = Some((dev, mount_point));
                    }
                    None => {
                        best_match = Some((dev, mount_point));
                    }
                    _ => {}
                }
            }
        }
    }

    best_match.map(|(dev, _)| dev)
}

/// Strips partition identifiers to isolate the primary block device name in `/sys/block/`.
///
/// # Examples
/// - `"sda1"` -> `"sda"`
/// - `"vda2"` -> `"vda"`
/// - `"nvme0n1p1"` -> `"nvme0n1"`
/// - `"mmcblk0p1"` -> `"mmcblk0"`
/// - `"dm-0"` -> `"dm-0"` (LVM / LUKS keeps full name)
/// - `"md0"` -> `"md0"` (Software RAID keeps full name)
fn extract_base_block_device(dev_name: &str) -> &str {
    if dev_name.starts_with("nvme") || dev_name.starts_with("mmcblk") {
        if let Some(idx) = dev_name.find('p') {
            return &dev_name[..idx];
        }
    } else if dev_name.starts_with("sd") || dev_name.starts_with("hd") || dev_name.starts_with("vd")
    {
        return dev_name.trim_end_matches(|c: char| c.is_ascii_digit());
    }
    dev_name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_base_block_device() {
        // Standard SATA / IDE / VirtIO drives
        assert_eq!(extract_base_block_device("sda1"), "sda");
        assert_eq!(extract_base_block_device("sdb12"), "sdb");
        assert_eq!(extract_base_block_device("hda"), "hda");
        assert_eq!(extract_base_block_device("vda2"), "vda");

        // NVMe & SD cards
        assert_eq!(extract_base_block_device("nvme0n1p1"), "nvme0n1");
        assert_eq!(extract_base_block_device("nvme1n1p2"), "nvme1n1");
        assert_eq!(extract_base_block_device("mmcblk0p1"), "mmcblk0");

        // Device Mapper (LVM/LUKS), Software RAID, Loop devices
        assert_eq!(extract_base_block_device("dm-0"), "dm-0");
        assert_eq!(extract_base_block_device("md0"), "md0");
        assert_eq!(extract_base_block_device("loop0"), "loop0");
    }

    #[test]
    fn test_parse_best_mount() {
        let mock_mounts = "\
sysfs /sys sysfs rw,nosuid,nodev,noexec,relatime 0 0
proc /proc proc rw,nosuid,nodev,noexec,relatime 0 0
/dev/sda2 / ext4 rw,relatime 0 0
/dev/nvme0n1p1 /boot/efi vfat rw,relatime 0 0
/dev/sdb1 /media/storage ext4 rw,relatime 0 0
/dev/mapper/vg-home /home ext4 rw,relatime 0 0
";

        assert_eq!(
            parse_best_mount(mock_mounts, Path::new("/usr/bin/ls")),
            Some("/dev/sda2")
        );

        assert_eq!(
            parse_best_mount(mock_mounts, Path::new("/media/storage/Movies/video.mkv")),
            Some("/dev/sdb1")
        );

        assert_eq!(
            parse_best_mount(mock_mounts, Path::new("/home/user/file.txt")),
            Some("/dev/mapper/vg-home")
        );
        // test
        assert_eq!(parse_best_mount("", Path::new("/path")), None);
    }
}
