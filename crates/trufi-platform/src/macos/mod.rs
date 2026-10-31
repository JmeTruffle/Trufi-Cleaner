//! macOS: `.app` bundles, launchd agents, Software Update, the Trash and
//! Full Disk Access. No elevation of the whole app: the few actions that
//! need an administrator (system daemons, apps owned by root) ask for the
//! password through macOS itself.

mod apps;
mod cmd;
mod launchd;
mod login_items;
mod specials;
mod updates;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use trufi_core::model::{InstalledApp, StartupItem, UninstallOutcome};
use trufi_core::monitor::{MemoryPressure, PressureLevel};
use trufi_core::platform::{LeftoverRoots, OsFamily, Platform, SystemPage};
use trufi_core::rules::Special;
use trufi_core::updates::{macos_name, UpdateStatus};
use trufi_core::Result;

pub struct MacPlatform {
    home: PathBuf,
    /// Per-user folder that holds `$TMPDIR` (`/private/var/folders/xx/…`).
    user_dir: Option<PathBuf>,
    major: u32,
    app_sizes: apps::SizeCache,
    /// Enabled login items at the last listing. Listing asks for permission
    /// to control System Events, so the health score reuses this.
    login_enabled: std::sync::Mutex<Option<usize>>,
}

impl MacPlatform {
    pub fn new() -> Self {
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| "/".into());
        let user_dir = std::env::var_os("TMPDIR")
            .map(PathBuf::from)
            .and_then(|t| std::fs::canonicalize(t).ok())
            .and_then(|t| t.parent().map(Path::to_path_buf))
            .filter(|d| d.starts_with("/private/var/folders"));
        let major = sysinfo::System::os_version().map(|v| updates::major_of(&v)).unwrap_or(0);
        MacPlatform { home, user_dir, major, app_sizes: Default::default(), login_enabled: Default::default() }
    }

    fn lib(&self, rel: &str) -> PathBuf {
        self.home.join("Library").join(rel)
    }

    fn trash_dir(&self) -> PathBuf {
        self.home.join(".Trash")
    }

    /// Our own data (the Tauri app config dir uses the same path).
    fn data_dir(&self) -> PathBuf {
        self.lib("Application Support/app.trufi.cleaner")
    }
}

#[cfg(target_os = "macos")]
fn sysctl_i32(name: &str) -> Option<i32> {
    let name = std::ffi::CString::new(name).ok()?;
    let mut value: i32 = 0;
    let mut len = std::mem::size_of::<i32>();
    // SAFETY: the buffer is an i32 and `len` says so; sysctlbyname writes at
    // most `len` bytes.
    let r = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            (&mut value as *mut i32).cast(),
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    (r == 0).then_some(value)
}

#[cfg(not(target_os = "macos"))]
fn sysctl_i32(_name: &str) -> Option<i32> {
    None
}

/// `kern.memorystatus_vm_pressure_level`: 1 normal, 2 warning, 4 critical.
/// `kern.memorystatus_level`: percent of memory macOS considers free.
pub fn pressure_from(level: i32, free_percent: i32) -> Option<MemoryPressure> {
    let level = match level {
        1 => PressureLevel::Normal,
        2 => PressureLevel::Warning,
        4 => PressureLevel::Critical,
        _ => return None,
    };
    Some(MemoryPressure { level, percent: (100 - free_percent.clamp(0, 100)) as f32 })
}

/// Size and file count of a folder; `None` when macOS refuses to list it
/// (no Full Disk Access).
fn dir_size(dir: &Path) -> Option<(u64, u64)> {
    std::fs::read_dir(dir).ok()?;
    let (mut bytes, mut files) = (0, 0);
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_dir() {
                stack.push(e.path());
            } else if let Ok(m) = e.metadata() {
                bytes += m.len();
                files += 1;
            }
        }
    }
    Some((bytes, files))
}

impl Platform for MacPlatform {
    fn os(&self) -> OsFamily {
        OsFamily::Macos
    }

    fn os_label(&self) -> String {
        match sysinfo::System::os_version() {
            Some(v) => format!("{} {v}", macos_name(self.major)),
            None => "macOS".into(),
        }
    }

    fn path_vars(&self) -> HashMap<String, PathBuf> {
        let mut vars = HashMap::from([("HOME".to_string(), self.home.clone())]);
        if let Some(d) = &self.user_dir {
            vars.insert("DARWIN_USER_DIR".into(), d.clone());
        }
        // Other apps' sandbox containers. Without Full Disk Access, macOS 14+
        // asks "allow access to data from other apps" on every scan, so rules
        // that use %CONTAINERS% apply only once the app has that access.
        if self.full_disk_access() == Some(true) {
            vars.insert("CONTAINERS".into(), self.lib("Containers"));
        }
        vars
    }

    fn path_lists(&self) -> HashMap<String, Vec<PathBuf>> {
        // Large-file search covers the home folder: the system volume is
        // read-only and apps are listed on the Optimize page.
        HashMap::from([("DRIVES".into(), vec![self.home.clone()])])
    }

    fn large_file_skip(&self) -> Vec<PathBuf> {
        let mut skip = vec![self.home.join("Library"), self.trash_dir(), self.home.join("Applications")];
        // Protected anyway; skipping them saves walking photo libraries.
        for d in ["Documents", "Desktop", "Pictures", "Movies", "Music"] {
            skip.push(self.home.join(d));
        }
        skip
    }

    fn protected_paths(&self) -> Vec<PathBuf> {
        let mut p: Vec<PathBuf> = [
            "/System", "/usr", "/bin", "/sbin", "/opt", "/Library", "/Applications", "/private/etc",
            "/private/var/db", "/private/var/root", "/Volumes",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();
        for d in ["Documents", "Desktop", "Pictures", "Movies", "Music", ".ssh", ".gnupg", "Applications"] {
            p.push(self.home.join(d));
        }
        for d in [
            "Keychains",
            "Mobile Documents", // iCloud Drive
            "CloudStorage",     // Dropbox, OneDrive, Google Drive: deleting there deletes in the cloud
            "Mail",
            "Messages",
            "Safari",
            "Photos",
            "Calendars",
            "Reminders",
            "Notes",
            "Preferences",
            "Cookies",
            "Accounts",
            "PersonalizationPortrait",
            "Group Containers",
            "Application Support/AddressBook",
            "Application Support/com.apple.TCC",
            "Application Support/Knowledge",
            "Application Support/CallHistoryDB",
        ] {
            p.push(self.lib(d));
        }
        p
    }

    fn is_elevated(&self) -> bool {
        false
    }

    fn full_disk_access(&self) -> Option<bool> {
        // Readable only with Full Disk Access; a common, cheap probe.
        let tcc = self.lib("Application Support/com.apple.TCC/TCC.db");
        if std::fs::File::open(&tcc).is_ok() {
            return Some(true);
        }
        Some(std::fs::read_dir(self.lib("Safari")).is_ok())
    }

    fn installed_apps(&self) -> Result<Vec<InstalledApp>> {
        Ok(apps::installed_apps(&self.home, &self.app_sizes))
    }

    fn uninstall_app(&self, id: &str) -> Result<UninstallOutcome> {
        apps::uninstall(id, &self.home)
    }

    fn leftover_roots(&self) -> LeftoverRoots {
        let mut data_roots = vec![self.lib("Application Support")];
        // See path_vars: listing other apps' containers needs Full Disk Access.
        if self.full_disk_access() == Some(true) {
            data_roots.push(self.lib("Containers"));
        }
        LeftoverRoots {
            data_roots,
            admin_roots: Vec::new(),
            ignore_names: LEFTOVER_IGNORE.iter().map(|s| s.to_string()).collect(),
            ignore_prefixes: vec!["com.apple.".into(), "group.com.apple.".into()],
        }
    }

    fn startup_items(&self) -> Result<Vec<StartupItem>> {
        let mut items = login_items::list(&self.data_dir());
        *self.login_enabled.lock().unwrap() = Some(items.iter().filter(|i| i.enabled).count());
        items.extend(launchd::list(&self.home));
        Ok(items)
    }

    fn startup_enabled_count(&self) -> Result<usize> {
        let agents = launchd::list(&self.home).iter().filter(|i| i.enabled).count();
        Ok(agents + self.login_enabled.lock().unwrap().unwrap_or(0))
    }

    fn set_startup_enabled(&self, id: &str, enabled: bool) -> Result<()> {
        if id.starts_with(login_items::ID_PREFIX) {
            login_items::set_enabled(id, enabled, &self.data_dir())
        } else {
            launchd::set_enabled(id, enabled, &self.home)
        }
    }

    fn special_info(&self, special: Special) -> Result<Option<(u64, u64)>> {
        Ok(match special {
            Special::Trash => dir_size(&self.trash_dir()),
            Special::TmSnapshots => specials::tm_info(),
            Special::Simulators => specials::simulators_info(),
            Special::DockerBuildCache => specials::docker_info(&self.home),
        })
    }

    fn special_clean(&self, special: Special) -> Result<()> {
        match special {
            Special::Trash => self.empty_trash(),
            Special::TmSnapshots => specials::tm_clean(),
            Special::Simulators => specials::simulators_clean(),
            Special::DockerBuildCache => specials::docker_clean(&self.home),
        }
    }

    fn memory_pressure(&self) -> Option<MemoryPressure> {
        pressure_from(
            sysctl_i32("kern.memorystatus_vm_pressure_level")?,
            sysctl_i32("kern.memorystatus_level")?,
        )
    }

    fn trash_info(&self) -> Result<Option<(u64, u64)>> {
        // `None` without Full Disk Access: the UI explains how to grant it.
        Ok(dir_size(&self.trash_dir()))
    }

    fn move_to_trash(&self, paths: &[PathBuf]) -> Result<()> {
        crate::move_to_trash(paths)
    }

    fn empty_trash(&self) -> Result<()> {
        let Ok(rd) = std::fs::read_dir(self.trash_dir()) else { return Ok(()) };
        for e in rd.flatten() {
            let p = e.path();
            let is_dir = e.file_type().is_ok_and(|t| t.is_dir());
            let _ = if is_dir { std::fs::remove_dir_all(&p) } else { std::fs::remove_file(&p) };
        }
        Ok(())
    }

    fn update_status(&self) -> Result<UpdateStatus> {
        updates::status()
    }

    fn open_system_page(&self, page: SystemPage) -> Result<()> {
        // Ventura (13) replaced System Preferences with System Settings and
        // renamed most panes; the privacy anchors kept working.
        let modern = self.major >= 13;
        let url = match page {
            SystemPage::SystemUpdate if modern => "x-apple.systempreferences:com.apple.Software-Update-Settings.extension",
            SystemPage::SystemUpdate => "x-apple.systempreferences:com.apple.preferences.softwareupdate",
            SystemPage::LoginItems if modern => "x-apple.systempreferences:com.apple.LoginItems-Settings.extension",
            SystemPage::LoginItems => "x-apple.systempreferences:com.apple.preferences.users",
            SystemPage::FullDiskAccess => "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles",
            SystemPage::OptionalUpdates | SystemPage::DeviceManager => {
                return Err(trufi_core::Error::Unsupported("system page"))
            }
        };
        cmd::open(url)
    }
}

/// Folders in Application Support that belong to macOS, to Apple apps, or
/// to tools that do not live in /Applications, and are never leftovers.
const LEFTOVER_IGNORE: &[&str] = &[
    "Apple", "AddressBook", "Accounts", "AccessibilityBundles", "Animoji", "App Store", "CallHistoryDB",
    "CallHistoryTransactions", "CloudDocs", "CrashReporter", "DifferentialPrivacy", "Dock", "FaceTime",
    "FileProvider", "Knowledge", "MobileSync", "Quick Look", "SyncServices", "TrustedPeersHelper", "iCloud",
    "icdd", "networkserviceproxy", "locationaccessstored", "DiskImages", "Spotlight", "Mail", "Safari",
    "Photos", "Music", "TV", "Podcasts", "Books", "Maps", "Notes", "Reminders", "Stocks", "Weather",
    "Siri", "Preview", "Shortcuts", "homeenergyd", "identityservicesd", "videosubscriptionsd", "Wallpaper",
    "ControlCenter", "NotificationCenter", "Screen Time", "AutomaticAssessmentConfiguration", "Caches",
    "Containers", "Logs", "Preferences", "Temp", "tmp", "CEF", "Electron", "Crashpad", "Microsoft",
    "Microsoft AutoUpdate", "JetBrains", "Google", "Mozilla", "Firefox", "BraveSoftware", "Steam",
    "minecraft", "Code", "Docker", "Homebrew", "pip", "npm", "node", "uv", "Python", "Java", "Oracle",
    "GitHub Desktop", "Claude", "Claude-Data", "Anthropic", "com.trufi.cleaner", "app.trufi.cleaner",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_pressure_levels() {
        assert_eq!(pressure_from(1, 70), Some(MemoryPressure { level: PressureLevel::Normal, percent: 30.0 }));
        assert_eq!(pressure_from(4, 5).unwrap().level, PressureLevel::Critical);
        assert_eq!(pressure_from(3, 50), None);
    }

    #[test]
    fn protection_and_roots() {
        let p = MacPlatform {
            home: "/Users/u".into(),
            user_dir: Some("/private/var/folders/ab/xyz".into()),
            major: 15,
            app_sizes: Default::default(),
            login_enabled: Default::default(),
        };
        let vars = p.path_vars();
        let guard = trufi_core::safety::SafetyGuard::new(p.protected_paths(), vars.values().cloned().collect());
        assert!(guard.allows_root(Path::new("/Users/u/Library/Caches")));
        assert!(guard.allows_root(Path::new("/private/var/folders/ab/xyz/T")));
        assert!(!guard.allows_root(Path::new("/private/var/folders/ab/xyz")));
        assert!(!guard.allows_root(Path::new("/Users/u/Library/Mobile Documents/x")));
        assert!(!guard.allows_root(Path::new("/Users/u/Library/CloudStorage/Dropbox")));
        assert!(!guard.allows_root(Path::new("/Users/u/Library")));
        assert!(!guard.allows_root(Path::new("/System/Library/Caches")));
        assert!(!guard.allows_root(Path::new("/Users/u")));
    }

    fn write(path: &Path, bytes: usize, age_hours: u64) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, vec![1u8; bytes]).unwrap();
        let t = std::time::SystemTime::now() - std::time::Duration::from_secs(age_hours * 3600);
        std::fs::File::options().write(true).open(path).unwrap().set_modified(t).unwrap();
    }

    /// The built-in macOS rule packs against a fake home folder.
    #[test]
    fn builtin_rules_on_a_fake_mac() {
        use trufi_core::rules::RuleSet;
        use trufi_core::scanner::{ScanOptions, Scanner};

        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().to_path_buf();
        let lib = home.join("Library");
        write(&lib.join("Caches/com.example.app/data.bin"), 1000, 48);
        write(&lib.join("Caches/com.example.app/fresh.bin"), 1000, 1);
        write(&lib.join("Caches/Google/Chrome/Default/Cache/Cache_Data/f_0001"), 700, 1);
        write(&lib.join("Application Support/Google/Chrome/Default/Code Cache/js/a"), 300, 48);
        write(&lib.join("Application Support/Google/Chrome/Default/History"), 300, 48);
        write(&lib.join("Caches/com.apple.Safari/WebKitCache/x"), 200, 48);
        write(&lib.join("Containers/com.apple.mail/Data/Library/Mail Downloads/a.pdf"), 200, 48);
        write(&lib.join("Developer/Xcode/iOS DeviceSupport/iPhone 17.0/Symbols/a"), 400, 900);
        write(&lib.join("Developer/Xcode/iOS DeviceSupport/iPhone 18.0/Symbols/a"), 500, 10);
        // keep_newest goes by the folder's own date.
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(900 * 3600);
        std::fs::File::open(lib.join("Developer/Xcode/iOS DeviceSupport/iPhone 17.0"))
            .unwrap()
            .set_modified(old)
            .unwrap();
        write(&lib.join("Mobile Documents/com~apple~CloudDocs/doc.pages"), 100, 48);
        write(&home.join("Downloads/Setup.dmg"), 900, 48);
        write(&home.join("Downloads/photo.jpg"), 900, 48);

        let platform = MacPlatform {
            home: home.clone(),
            user_dir: None,
            major: 15,
            app_sizes: Default::default(),
            login_enabled: Default::default(),
        };
        let rules = RuleSet::builtin("macos").unwrap();
        let scanner = Scanner::new(&platform);
        let summary = scanner.scan(&rules.rules, ScanOptions::default(), &|_| {}).summary();
        let get = |id: &str| summary.findings.iter().find(|f| f.rule_id == id);

        // Fresh files stay; Chrome's own rule owns its cache, without the age limit.
        assert_eq!(get("mac.user_caches").unwrap().bytes, 1000);
        let chrome = get("mac.browser.chrome.cache").unwrap();
        assert_eq!(chrome.bytes, 700 + 300, "History must not be touched");
        assert_eq!(chrome.running_processes, Vec::<String>::new());
        // Excluded from the generic rule, owned by the Safari rule.
        assert_eq!(get("mac.browser.safari.cache").unwrap().bytes, 200);
        // Containers need Full Disk Access, which a fake home does not have.
        assert!(get("mac.mail_downloads").is_none());
        let installers = get("mac.downloads_installers").unwrap();
        assert!(installers.itemized);
        assert_eq!(installers.bytes, 900);
        // Newest device support stays.
        let ds = get("mac.dev.xcode.device_support").unwrap();
        assert_eq!(ds.bytes, 400);
        assert!(ds.top_entries.iter().all(|e| e.path.ends_with("iPhone 17.0")));
        // Nothing inside iCloud Drive, ever.
        assert!(summary
            .findings
            .iter()
            .flat_map(|f| &f.top_entries)
            .all(|e| !e.path.starts_with(lib.join("Mobile Documents"))));
    }
}
