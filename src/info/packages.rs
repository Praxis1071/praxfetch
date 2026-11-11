//! Native package database detection and package-count parsing.

use std::{fs, path::Path};

use super::PackageInfo;

pub(super) fn read_packages() -> PackageInfo {
    let pacman_db = Path::new("/var/lib/pacman/local");
    match fs::read_dir(pacman_db) {
        Ok(entries) => {
            return PackageInfo {
                manager: "pacman".into(),
                count: count_package_entries(entries),
            };
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        // If the Pacman database exists but cannot be scanned, report its
        // count as unavailable instead of silently switching managers.
        Err(_) => {
            return PackageInfo {
                manager: "pacman".into(),
                count: None,
            };
        }
    }

    let dpkg_status = Path::new("/var/lib/dpkg/status");
    if dpkg_status.exists() {
        return PackageInfo {
            manager: "dpkg".into(),
            count: fs::read_to_string(dpkg_status)
                .ok()
                .map(|text| count_dpkg_installed(&text)),
        };
    }

    let apk_db = Path::new("/lib/apk/db/installed");
    if apk_db.exists() {
        return PackageInfo {
            manager: "apk".into(),
            count: fs::read_to_string(apk_db)
                .ok()
                .map(|text| count_apk_packages(&text)),
        };
    }

    if [
        "/var/lib/rpm/Packages",
        "/var/lib/rpm/rpmdb.sqlite",
        "/var/lib/rpm/Packages.db",
        "/usr/lib/sysimage/rpm/Packages",
        "/usr/lib/sysimage/rpm/rpmdb.sqlite",
        "/usr/lib/sysimage/rpm/Packages.db",
    ]
    .iter()
    .any(|path| Path::new(path).exists())
    {
        return PackageInfo {
            manager: "rpm".into(),
            count: None,
        };
    }

    PackageInfo {
        manager: "unknown".into(),
        count: None,
    }
}

fn count_package_directories(path: &Path) -> Option<usize> {
    count_package_entries(fs::read_dir(path).ok()?)
}

fn count_package_entries(mut entries: fs::ReadDir) -> Option<usize> {
    entries.try_fold(0usize, |count, entry| {
        let entry = entry.ok()?;
        let is_directory = entry.file_type().ok()?.is_dir();
        if is_directory {
            count.checked_add(1)
        } else {
            Some(count)
        }
    })
}

fn count_dpkg_installed(text: &str) -> usize {
    let mut count = 0usize;
    let mut installed = false;

    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            if installed {
                count += 1;
                installed = false;
            }
        } else if line == "Status: install ok installed" {
            installed = true;
        }
    }

    if installed {
        count += 1;
    }
    count
}

fn count_apk_packages(text: &str) -> usize {
    text.lines()
        .filter_map(|line| line.strip_prefix("P:"))
        .filter(|name| !name.trim().is_empty())
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pacman_package_counter_counts_only_complete_directory_entries() {
        let root =
            std::env::temp_dir().join(format!("praxfetch-pacman-db-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("alpha-1.0-1")).unwrap();
        fs::create_dir_all(root.join("beta-2.0-1")).unwrap();
        fs::write(root.join("db.lck"), "").unwrap();

        assert_eq!(count_package_directories(&root), Some(2));
        assert_eq!(
            count_package_directories(&root.join("does-not-exist")),
            None
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dpkg_package_counter_handles_lf_crlf_and_incomplete_stanzas() {
        let fixture = "Package: alpha\nStatus: install ok installed\n\nPackage: beta\nStatus: deinstall ok config-files\n\nPackage: gamma\nStatus: install ok installed";
        assert_eq!(count_dpkg_installed(fixture), 2);

        let crlf = "Package: alpha\r\nStatus: install ok installed\r\n\r\nPackage: beta\r\nStatus: install ok installed\r\n";
        assert_eq!(count_dpkg_installed(crlf), 2);
        assert_eq!(count_dpkg_installed("malformed stanza without status"), 0);
    }

    #[test]
    fn apk_package_counter_requires_a_nonempty_package_name() {
        let fixture = "C:Q1\nP:busybox\nV:1.0\n\nP:musl\nV:1.2\nP:\n";
        assert_eq!(count_apk_packages(fixture), 2);
        assert_eq!(count_apk_packages("not an installed package"), 0);
    }
}
