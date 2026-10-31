//! Linux (and other Unix systems). Not a release target: enough for the
//! core and the UI to be developed and tested on any machine. Paths,
//! protection and the freedesktop trash are real; startup items and the app
//! inventory are not implemented.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use trufi_core::model::{InstalledApp, StartupItem};
use trufi_core::platform::{LeftoverRoots, OsFamily, Platform};
use trufi_core::Result;

pub struct LinuxPlatform {
    home: PathBuf,
}

impl LinuxPlatform {
    pub fn new() -> Self {
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| "/".into());
        LinuxPlatform { home }
    }

    fn trash_dir(&self) -> PathBuf {
        self.home.join(".local/share/Trash")
    }
}

fn dir_size(dir: &Path) -> (u64, u64) {
    let mut bytes = 0;
    let mut items = 0;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_dir() {
                stack.push(e.path());
            } else if ft.is_file() {
                bytes += e.metadata().map(|m| m.len()).unwrap_or(0);
                items += 1;
            }
        }
    }
    (bytes, items)
}

impl Platform for LinuxPlatform {
    fn os(&self) -> OsFamily {
        OsFamily::Linux
    }

    fn os_label(&self) -> String {
        sysinfo::System::long_os_version().unwrap_or_else(|| "Linux".into())
    }

    fn path_vars(&self) -> HashMap<String, PathBuf> {
        HashMap::from([("HOME".into(), self.home.clone())])
    }

    fn path_lists(&self) -> HashMap<String, Vec<PathBuf>> {
        // Large-file search stays inside the home folder.
        HashMap::from([("DRIVES".into(), vec![self.home.clone()])])
    }

    fn large_file_skip(&self) -> Vec<PathBuf> {
        vec![self.home.join(".cache"), self.home.join(".local")]
    }

    fn protected_paths(&self) -> Vec<PathBuf> {
        let mut p: Vec<PathBuf> = ["/usr", "/bin", "/sbin", "/etc", "/boot", "/lib", "/var/lib"]
            .iter()
            .map(PathBuf::from)
            .collect();
        for d in ["Documents", "Desktop", "Pictures", "Videos", "Music", ".ssh", ".gnupg"] {
            p.push(self.home.join(d));
        }
        p
    }

    fn is_elevated(&self) -> bool {
        false
    }

    fn installed_apps(&self) -> Result<Vec<InstalledApp>> {
        // No reliable app inventory: leftover detection stays off.
        Ok(Vec::new())
    }

    fn leftover_roots(&self) -> LeftoverRoots {
        LeftoverRoots::default()
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

    fn move_to_trash(&self, paths: &[PathBuf]) -> Result<()> {
        crate::move_to_trash(paths)
    }

    fn empty_trash(&self) -> Result<()> {
        for sub in ["files", "info"] {
            let Ok(rd) = std::fs::read_dir(self.trash_dir().join(sub)) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                let is_dir = e.file_type().is_ok_and(|t| t.is_dir());
                let _ = if is_dir { std::fs::remove_dir_all(&p) } else { std::fs::remove_file(&p) };
            }
        }
        Ok(())
    }
}
