//! The seam between the OS-agnostic core and a concrete operating system.
//!
//! Adding macOS (or Linux) support means implementing this trait and
//! shipping a rule pack for it; nothing else in the core changes.

use crate::drivers::DriverInfo;
use crate::model::{InstalledApp, StartupItem, UninstallOutcome};
use crate::monitor::MemoryPressure;
use crate::rules::Special;
use crate::updates::UpdateStatus;
use crate::Result;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OsFamily {
    Windows,
    Macos,
    Linux,
}

impl OsFamily {
    /// Name used in rule packs (`os = "windows"`).
    pub fn as_str(self) -> &'static str {
        match self {
            OsFamily::Windows => "windows",
            OsFamily::Macos => "macos",
            OsFamily::Linux => "linux",
        }
    }
}

/// System settings pages the UI can send the user to. Not every OS has
/// every page: the platform answers `Unsupported` for the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemPage {
    /// Windows Update / macOS Software Update.
    #[serde(alias = "windows_update")]
    SystemUpdate,
    /// Where Windows Update keeps optional driver updates.
    OptionalUpdates,
    DeviceManager,
    /// Apps that start at login (Windows: Settings > Startup apps; macOS:
    /// General > Login Items).
    LoginItems,
    /// macOS: Privacy & Security > Full Disk Access.
    FullDiskAccess,
}

/// Where leftover detection should look and what it must never touch.
#[derive(Debug, Clone, Default)]
pub struct LeftoverRoots {
    /// Folders whose direct children are per-vendor/per-app data folders
    /// (`%APPDATA%`, `%LOCALAPPDATA%`, `~/Library/Application Support`, ...).
    pub data_roots: Vec<PathBuf>,
    /// Folders under these roots need elevation to delete.
    pub admin_roots: Vec<PathBuf>,
    /// Child folder names (normalised, see `util::normalize_name`) that
    /// belong to the OS itself or to drivers and are never leftovers.
    pub ignore_names: Vec<String>,
    /// Child folder name prefixes that are never leftovers (`com.apple.`
    /// on macOS). Compared case-insensitively, as written.
    pub ignore_prefixes: Vec<String>,
}

pub trait Platform: Send + Sync {
    fn os(&self) -> OsFamily;

    /// Human readable OS name/version, e.g. "Windows 11 Pro 23H2".
    fn os_label(&self) -> String;

    /// Variables available to rule path templates (`%LOCALAPPDATA%`, `~`).
    /// Keys are upper-case.
    fn path_vars(&self) -> HashMap<String, PathBuf>;

    /// Variables with several values; a rule path using one is tried with
    /// each. `DRIVES` must list every fixed drive so rules can cover all
    /// disks, not just the system one.
    fn path_lists(&self) -> HashMap<String, Vec<PathBuf>> {
        HashMap::new()
    }

    /// Paths that must never be deleted or descended into for deletion,
    /// whatever a rule says.
    fn protected_paths(&self) -> Vec<PathBuf>;

    /// Folders the large-file search skips (OS, installed programs, games).
    fn large_file_skip(&self) -> Vec<PathBuf> {
        Vec::new()
    }

    fn is_elevated(&self) -> bool;

    /// macOS: whether the app has Full Disk Access. Without it the Trash,
    /// Safari, Mail and some app containers cannot be read. `None` where
    /// the OS has no such permission.
    fn full_disk_access(&self) -> Option<bool> {
        None
    }

    /// Restart the current executable with elevated rights.
    fn relaunch_elevated(&self) -> Result<()> {
        Err(crate::Error::Unsupported("relaunch_elevated"))
    }

    fn installed_apps(&self) -> Result<Vec<InstalledApp>>;

    fn leftover_roots(&self) -> LeftoverRoots;

    /// Starts the app's own uninstaller (Windows) or moves the app to the
    /// Trash (macOS). The OS or the uninstaller asks the user.
    fn uninstall_app(&self, _id: &str) -> Result<UninstallOutcome> {
        Err(crate::Error::Unsupported("uninstall_app"))
    }

    fn startup_items(&self) -> Result<Vec<StartupItem>>;

    fn set_startup_enabled(&self, id: &str, enabled: bool) -> Result<()>;

    /// Enabled startup items, for the health score. Must not ask the user
    /// anything (macOS: no Automation prompt at app start).
    fn startup_enabled_count(&self) -> Result<usize> {
        Ok(self.startup_items()?.iter().filter(|i| i.enabled).count())
    }

    /// Size of the OS trash in bytes and number of items, when the OS can tell.
    fn trash_info(&self) -> Result<Option<(u64, u64)>>;

    fn empty_trash(&self) -> Result<()>;

    /// Size and item count of something a `special` rule cleans through
    /// the OS (`None`: not present on this machine). Bytes may be 0 when
    /// the OS cannot tell (Time Machine snapshots).
    fn special_info(&self, special: Special) -> Result<Option<(u64, u64)>> {
        match special {
            Special::Trash => self.trash_info(),
            _ => Ok(None),
        }
    }

    fn special_clean(&self, special: Special) -> Result<()> {
        match special {
            Special::Trash => self.empty_trash(),
            _ => Err(crate::Error::Unsupported("special cleaning")),
        }
    }

    /// Moves files and folders to the OS trash instead of deleting them, so
    /// the user can put them back.
    fn move_to_trash(&self, _paths: &[PathBuf]) -> Result<()> {
        Err(crate::Error::Unsupported("move_to_trash"))
    }

    /// macOS memory pressure, the measure macOS itself uses: memory "used"
    /// by caches is free for the asking there, so a high used share means
    /// little.
    fn memory_pressure(&self) -> Option<MemoryPressure> {
        None
    }

    /// Installed version and what the OS update service has waiting. Slow:
    /// it asks the update service, which goes online.
    fn update_status(&self) -> Result<UpdateStatus> {
        Err(crate::Error::Unsupported("update_status"))
    }

    /// Device drivers, plus devices that report a problem.
    fn drivers(&self) -> Result<Vec<DriverInfo>> {
        Err(crate::Error::Unsupported("drivers"))
    }

    fn open_system_page(&self, _page: SystemPage) -> Result<()> {
        Err(crate::Error::Unsupported("open_system_page"))
    }
}
