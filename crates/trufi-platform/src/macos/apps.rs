//! Applications: `.app` bundles in `/Applications` and `~/Applications`.
//! Removing one moves it to the Trash through Finder, the way a user would,
//! so it can be put back; when the vendor ships its own uninstaller next to
//! the app, that runs instead.

use super::cmd;
use plist::Value;
use rayon::prelude::*;
use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use trufi_core::model::{InstalledApp, UninstallOutcome};
use trufi_core::{Error, Result};

/// Apps macOS itself protects (System Integrity Protection).
const SIP_PROTECTED: &[&str] = &["com.apple.Safari"];
const OWN_BUNDLE_ID: &str = "app.trufi.cleaner";

/// Folders with applications. Bundles inside a vendor folder one level down
/// (`/Applications/Utilities`, `/Applications/Adobe Photoshop 2025`) count too.
pub fn app_dirs(home: &Path) -> Vec<PathBuf> {
    vec![PathBuf::from("/Applications"), home.join("Applications")]
}

fn is_app(path: &Path) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("app"))
}

fn find_bundles(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let Ok(ft) = e.file_type() else { continue };
        if !ft.is_dir() {
            continue; // symlinks and aliases point at apps listed elsewhere
        }
        let p = e.path();
        if is_app(&p) {
            out.push(p);
        } else if depth > 0 {
            find_bundles(&p, depth - 1, out);
        }
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct BundleInfo {
    pub id: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
}

pub fn parse_info(value: &Value) -> BundleInfo {
    let Some(d) = value.as_dictionary() else { return BundleInfo::default() };
    let s = |k: &str| {
        d.get(k)
            .and_then(Value::as_string)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
    };
    BundleInfo {
        id: s("CFBundleIdentifier"),
        name: s("CFBundleDisplayName").or_else(|| s("CFBundleName")),
        version: s("CFBundleShortVersionString").or_else(|| s("CFBundleVersion")),
    }
}

pub fn read_info(app: &Path) -> BundleInfo {
    Value::from_file(app.join("Contents/Info.plist"))
        .map(|v| parse_info(&v))
        .unwrap_or_default()
}

/// Space a folder takes on disk (allocated blocks: APFS compression and
/// sparse files count as what they really occupy). Links are not followed.
pub fn tree_size(dir: &Path) -> u64 {
    let mut bytes = 0;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_dir() {
                stack.push(e.path());
            } else if let Ok(m) = e.metadata() {
                bytes += m.blocks() * 512;
            }
        }
    }
    bytes
}

/// Bundle sizes by path, valid while the bundle's modification time is
/// unchanged (an update replaces the bundle). Walking Xcode alone takes
/// seconds, and every scan for leftovers lists the apps again.
pub type SizeCache = Mutex<HashMap<PathBuf, (SystemTime, u64)>>;

fn cached_size(path: &Path, cache: &SizeCache) -> u64 {
    let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    if let Some(m) = modified {
        if let Some(&(t, size)) = cache.lock().unwrap().get(path) {
            if t == m {
                return size;
            }
        }
    }
    let size = tree_size(path);
    if let Some(m) = modified {
        cache.lock().unwrap().insert(path.to_path_buf(), (m, size));
    }
    size
}

pub fn installed_apps(home: &Path, cache: &SizeCache) -> Vec<InstalledApp> {
    let mut bundles = Vec::new();
    for dir in app_dirs(home) {
        find_bundles(&dir, 1, &mut bundles);
    }
    let mut apps: Vec<InstalledApp> = bundles
        .par_iter()
        .map(|path| {
            let info = read_info(path);
            // Finder shows the bundle's file name; Info.plist names are
            // sometimes internal ("Electron").
            let name = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .or(info.name)
                .unwrap_or_default();
            let protected = info.id.as_deref().is_some_and(|id| SIP_PROTECTED.contains(&id) || id == OWN_BUNDLE_ID);
            InstalledApp {
                id: path.to_string_lossy().into_owned(),
                name,
                publisher: None,
                version: info.version,
                install_location: Some(path.clone()),
                size_bytes: Some(cached_size(path, cache)),
                broken: false,
                can_uninstall: !protected,
                bundle_id: info.id,
            }
        })
        .collect();
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

/// The vendor's own uninstaller shipped next to the app, if any
/// (`/Applications/Foo/Uninstall Foo.app`). Only inside a vendor folder:
/// in `/Applications` itself an "Uninstall X" belongs to another app.
pub fn sibling_uninstaller(app: &Path, home: &Path) -> Option<PathBuf> {
    let parent = app.parent()?;
    if app_dirs(home).iter().any(|d| d == parent) {
        return None;
    }
    let rd = std::fs::read_dir(parent).ok()?;
    rd.flatten().map(|e| e.path()).find(|p| {
        let n = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
        p != app && is_app(p) && (n.starts_with("uninstall") || n.contains("uninstaller"))
    })
}

pub fn uninstall(id: &str, home: &Path) -> Result<UninstallOutcome> {
    let path = PathBuf::from(id);
    let inside_app_dir = app_dirs(home).iter().any(|d| path.starts_with(d));
    if !path.is_absolute() || !is_app(&path) || !inside_app_dir || !path.is_dir() {
        return Err(Error::Platform(format!("Это не установленная программа: {id}")));
    }
    if path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(Error::Platform(format!("Это не установленная программа: {id}")));
    }
    if let Some(uninstaller) = sibling_uninstaller(&path, home) {
        cmd::open(&uninstaller.to_string_lossy())?;
        return Ok(UninstallOutcome::UninstallerStarted);
    }
    // Finder asks for the administrator password when the app belongs to
    // root (installed from a .pkg or the App Store).
    cmd::osascript(
        &[r#"tell application "Finder" to delete (POSIX file (item 1 of argv) as alias)"#],
        &[id],
        Duration::from_secs(300),
    )?;
    Ok(UninstallOutcome::MovedToTrash)
}

#[cfg(test)]
mod tests {
    use super::*;

    const INFO: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleIdentifier</key><string>com.tinyspeck.slackmacgap</string>
  <key>CFBundleName</key><string>Slack</string>
  <key>CFBundleShortVersionString</key><string>4.41.105</string>
  <key>CFBundleVersion</key><string>441105</string>
</dict></plist>"#;

    #[test]
    fn reads_bundle_info() {
        let v = Value::from_reader_xml(INFO.as_bytes()).unwrap();
        assert_eq!(
            parse_info(&v),
            BundleInfo {
                id: Some("com.tinyspeck.slackmacgap".into()),
                name: Some("Slack".into()),
                version: Some("4.41.105".into()),
            }
        );
    }

    #[test]
    fn finds_bundles_and_vendor_uninstallers() {
        let home = tempfile::tempdir().unwrap();
        let apps = home.path().join("Applications");
        let vendor = apps.join("Vendor Suite");
        for p in [apps.join("Slack.app/Contents"), vendor.join("Suite.app/Contents"), vendor.join("Uninstall Suite.app")] {
            std::fs::create_dir_all(p).unwrap();
        }
        std::fs::write(apps.join("Slack.app/Contents/Info.plist"), INFO).unwrap();
        std::fs::write(apps.join("Slack.app/Contents/blob"), vec![0u8; 10_000]).unwrap();

        let cache = SizeCache::default();
        let list = installed_apps(home.path(), &cache);
        let slack = list.iter().find(|a| a.name == "Slack").unwrap();
        assert_eq!(slack.bundle_id.as_deref(), Some("com.tinyspeck.slackmacgap"));
        assert!(slack.size_bytes.unwrap() >= 10_000);
        assert!(list.iter().any(|a| a.name == "Suite"));
        // Second listing comes from the cache.
        assert_eq!(cache.lock().unwrap().len(), list.len());
        let again = installed_apps(home.path(), &cache);
        assert_eq!(again.iter().find(|a| a.name == "Slack").unwrap().size_bytes, slack.size_bytes);

        assert_eq!(
            sibling_uninstaller(&vendor.join("Suite.app"), home.path()),
            Some(vendor.join("Uninstall Suite.app"))
        );
        // Directly in Applications: never guess an uninstaller.
        assert_eq!(sibling_uninstaller(&apps.join("Slack.app"), home.path()), None);
    }

    #[test]
    fn refuses_paths_outside_applications() {
        let home = tempfile::tempdir().unwrap();
        assert!(uninstall("/System/Applications/Mail.app", home.path()).is_err());
        assert!(uninstall("relative/Thing.app", home.path()).is_err());
        assert!(uninstall("/Applications/../System/Applications/Mail.app", home.path()).is_err());
    }
}
