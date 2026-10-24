//! macOS and Linux. The macOS port builds on this; today it covers paths,
//! protection, trash and a basic app inventory so the core and UI can be
//! developed and tested on any machine.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use trufi_core::model::{InstalledApp, StartupItem};
use trufi_core::platform::{LeftoverRoots, OsFamily, Platform};
use trufi_core::Result;

pub struct UnixPlatform {
    home: PathBuf,
}

impl UnixPlatform {
    pub fn new() -> Self {
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| "/".into());
        UnixPlatform { home }
    }

    fn trash_dir(&self) -> PathBuf {
        if cfg!(target_os = "macos") {
            self.home.join(".Trash")
        } else {
            self.home.join(".local/share/Trash")
        }
    }
}

fn dir_size(dir: &Path) -> (u64, u64) {
    let mut bytes = 0;
    let mut items = 0;
    for e in walk(dir) {
        if let Ok(m) = e.symlink_metadata() {
            if m.is_file() {
                bytes += m.len();
                items += 1;
            }
        }
    }
    (bytes, items)
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if e.file_type().is_ok_and(|t| t.is_dir()) {
                stack.push(p.clone());
            }
            out.push(p);
        }
    }
    out
}

impl Platform for UnixPlatform {
    fn os(&self) -> OsFamily {
        if cfg!(target_os = "macos") {
            OsFamily::Macos
        } else {
            OsFamily::Linux
        }
    }

    fn os_label(&self) -> String {
        sysinfo::System::long_os_version().unwrap_or_else(|| self.os().as_str().into())
    }

    fn path_vars(&self) -> HashMap<String, PathBuf> {
        HashMap::from([("HOME".into(), self.home.clone())])
    }

    fn path_lists(&self) -> HashMap<String, Vec<PathBuf>> {
        // Large-file search stays inside the home folder on Unix.
        HashMap::from([("DRIVES".into(), vec![self.home.clone()])])
    }

    fn large_file_skip(&self) -> Vec<PathBuf> {
        vec![self.home.join("Library"), self.home.join(".cache"), self.home.join(".local")]
    }

    fn protected_paths(&self) -> Vec<PathBuf> {
        let mut p: Vec<PathBuf> = ["/System", "/usr", "/bin", "/sbin", "/etc", "/boot", "/Applications"]
            .iter()
            .map(PathBuf::from)
            .collect();
        for d in ["Documents", "Desktop", "Pictures", "Movies", "Music", "Videos", ".ssh", ".gnupg"] {
            p.push(self.home.join(d));
        }
        p.push(self.home.join("Library/Keychains"));
        p.push(self.home.join("Library/Mobile Documents"));
        p
    }

    fn is_elevated(&self) -> bool {
        false
    }

    fn installed_apps(&self) -> Result<Vec<InstalledApp>> {
        let mut apps = Vec::new();
        for dir in [PathBuf::from("/Applications"), self.home.join("Applications")] {
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if let Some(app) = name.strip_suffix(".app") {
                    apps.push(InstalledApp {
                        id: e.path().display().to_string(),
                        name: app.to_string(),
                        install_location: Some(e.path()),
                        ..Default::default()
                    });
                }
            }
        }
        Ok(apps)
    }

    fn leftover_roots(&self) -> LeftoverRoots {
        if cfg!(target_os = "macos") {
            LeftoverRoots {
                data_roots: vec![self.home.join("Library/Application Support")],
                admin_roots: vec![],
                ignore_names: ["Apple", "com.apple", "CloudDocs", "CrashReporter", "AddressBook", "Knowledge"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            }
        } else {
            // No reliable app inventory on Linux: leftover detection is off.
            LeftoverRoots::default()
        }
    }

    fn startup_items(&self) -> Result<Vec<StartupItem>> {
        Ok(Vec::new())
    }

    fn set_startup_enabled(&self, _id: &str, _enabled: bool) -> Result<()> {
        Err(trufi_core::Error::Unsupported("startup items"))
    }

    fn trash_info(&self) -> Result<Option<(u64, u64)>> {
        let dir = self.trash_dir();
        if !dir.exists() {
            return Ok(None);
        }
        Ok(Some(dir_size(&dir)))
    }

    fn empty_trash(&self) -> Result<()> {
        let dir = self.trash_dir();
        let Ok(rd) = std::fs::read_dir(&dir) else { return Ok(()) };
        for e in rd.flatten() {
            let p = e.path();
            let is_dir = e.file_type().is_ok_and(|t| t.is_dir() && !t.is_symlink());
            let _ = if is_dir { std::fs::remove_dir_all(&p) } else { std::fs::remove_file(&p) };
        }
        Ok(())
    }
}
