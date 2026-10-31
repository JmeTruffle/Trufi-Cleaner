mod apps;
mod drivers;
mod powershell;
mod shell;
mod startup;
mod updates;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use trufi_core::drivers::DriverInfo;
use trufi_core::model::{InstalledApp, StartupItem, UninstallOutcome};
use trufi_core::platform::{LeftoverRoots, OsFamily, Platform, SystemPage};
use trufi_core::updates::UpdateStatus;
use trufi_core::Result;

pub struct WindowsPlatform {
    vars: HashMap<String, PathBuf>,
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from)
}

/// Expands `%VAR%` with the process environment (REG_EXPAND_SZ values).
pub(crate) fn expand_env(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(start) = rest.find('%') {
        let tail = &rest[start + 1..];
        let Some(end) = tail.find('%') else { break };
        out.push_str(&rest[..start]);
        match std::env::var(&tail[..end]) {
            Ok(v) => out.push_str(&v),
            Err(_) => out.push_str(&rest[start..start + end + 2]),
        }
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    out
}

impl WindowsPlatform {
    pub fn new() -> Self {
        let mut vars = HashMap::new();
        // %TEMP% is deliberately absent: it is often an 8.3 short path that
        // duplicates %LOCALAPPDATA%\Temp. Rules use the latter.
        for name in [
            "USERPROFILE",
            "LOCALAPPDATA",
            "APPDATA",
            "PROGRAMDATA",
            "PROGRAMFILES",
            "PROGRAMFILES(X86)",
            "PUBLIC",
        ] {
            if let Some(p) = env_path(name) {
                vars.insert(name.to_string(), p);
            }
        }
        if let Some(p) = env_path("SystemRoot").or_else(|| env_path("WINDIR")) {
            vars.insert("WINDIR".into(), p);
        }
        if let Some(d) = env_path("SystemDrive") {
            // "C:" -> "C:\" so that "%SYSTEMDRIVE%/AMD" is absolute.
            let mut s = d.into_os_string();
            s.push("\\");
            vars.insert("SYSTEMDRIVE".into(), PathBuf::from(s));
        }
        if let Some(home) = vars.get("USERPROFILE").cloned() {
            vars.insert("HOME".into(), home);
        }
        if let Some(steam) = apps::steam_path() {
            vars.insert("STEAM".into(), steam);
        }
        WindowsPlatform { vars }
    }

    fn var(&self, name: &str) -> Option<&Path> {
        self.vars.get(name).map(PathBuf::as_path)
    }

    fn join_all(&self, base: &str, children: &[&str]) -> Vec<PathBuf> {
        self.var(base)
            .map(|b| children.iter().map(|c| b.join(c)).collect())
            .unwrap_or_default()
    }
}

impl Platform for WindowsPlatform {
    fn os(&self) -> OsFamily {
        OsFamily::Windows
    }

    fn os_label(&self) -> String {
        sysinfo::System::long_os_version().unwrap_or_else(|| "Windows".into())
    }

    fn path_vars(&self) -> HashMap<String, PathBuf> {
        self.vars.clone()
    }

    fn path_lists(&self) -> HashMap<String, Vec<PathBuf>> {
        let mut lists = HashMap::from([("DRIVES".to_string(), shell::fixed_drives())]);
        if let Some(steam) = self.var("STEAM") {
            lists.insert("STEAMLIBS".into(), apps::steam_libraries(steam));
        }
        lists
    }

    fn large_file_skip(&self) -> Vec<PathBuf> {
        let mut skip: Vec<PathBuf> = ["WINDIR", "PROGRAMFILES", "PROGRAMFILES(X86)", "PROGRAMDATA"]
            .iter()
            .filter_map(|v| self.var(v).map(Path::to_path_buf))
            .collect();
        skip.extend(self.join_all("LOCALAPPDATA", &["Packages", "Microsoft"]));
        for drive in shell::fixed_drives() {
            for d in ["$Recycle.Bin", "System Volume Information", "Recovery", "$WinREAgent", "Windows.old"] {
                skip.push(drive.join(d));
            }
        }
        if let Some(steam) = self.var("STEAM") {
            for lib in apps::steam_libraries(steam) {
                skip.push(lib.join("steamapps"));
            }
        }
        skip
    }

    fn protected_paths(&self) -> Vec<PathBuf> {
        let mut p = Vec::new();
        p.extend(self.join_all(
            "WINDIR",
            &[
                "System32",
                "SysWOW64",
                "WinSxS",
                "servicing",
                "Installer",
                "Prefetch",
                "Boot",
                "Fonts",
                "assembly",
                "Microsoft.NET",
                "SystemApps",
                "SoftwareDistribution/DataStore",
            ],
        ));
        p.extend(self.join_all("SYSTEMDRIVE", &["System Volume Information", "$Recycle.Bin", "Recovery"]));
        p.extend(self.join_all(
            "PROGRAMDATA",
            &[
                "Package Cache",
                "NVIDIA/NGX",
                "Microsoft/Windows Defender",
                "Microsoft/Crypto",
                "chocolatey",
            ],
        ));
        p.extend(self.join_all("PROGRAMFILES", &["WindowsApps", "Windows Defender"]));
        p.extend(self.join_all(
            "LOCALAPPDATA",
            &["Microsoft/Windows/WebCache", "Microsoft/Credentials", "Claude-Data"],
        ));
        p.extend(self.join_all(
            "APPDATA",
            &[
                "Microsoft/Protect",
                "Microsoft/Credentials",
                "Microsoft/Crypto",
                "Microsoft/SystemCertificates",
                "Code/User",
                "Cursor/User",
                "PrismLauncher",
            ],
        ));
        p.extend(self.join_all(
            "USERPROFILE",
            &["Documents", "Desktop", "Pictures", "Videos", "Music", "OneDrive", "Saved Games"],
        ));
        p
    }

    fn is_elevated(&self) -> bool {
        shell::is_elevated()
    }

    fn relaunch_elevated(&self) -> Result<()> {
        shell::relaunch_elevated()
    }

    fn installed_apps(&self) -> Result<Vec<InstalledApp>> {
        Ok(apps::installed_apps())
    }

    fn uninstall_app(&self, id: &str) -> Result<UninstallOutcome> {
        apps::uninstall(id).map(|_| UninstallOutcome::UninstallerStarted)
    }

    fn leftover_roots(&self) -> LeftoverRoots {
        let mut data_roots: Vec<PathBuf> = ["APPDATA", "LOCALAPPDATA", "PROGRAMDATA"]
            .iter()
            .filter_map(|v| self.var(v).map(Path::to_path_buf))
            .collect();
        data_roots.extend(self.join_all("LOCALAPPDATA", &["Programs"]));
        data_roots.extend(self.join_all("USERPROFILE", &["AppData/LocalLow"]));
        LeftoverRoots {
            data_roots,
            admin_roots: self.var("PROGRAMDATA").map(Path::to_path_buf).into_iter().collect(),
            ignore_names: LEFTOVER_IGNORE.iter().map(|s| s.to_string()).collect(),
            ignore_prefixes: Vec::new(),
        }
    }

    fn startup_items(&self) -> Result<Vec<StartupItem>> {
        Ok(startup::list(&self.vars))
    }

    fn set_startup_enabled(&self, id: &str, enabled: bool) -> Result<()> {
        startup::set_enabled(id, enabled)
    }

    fn trash_info(&self) -> Result<Option<(u64, u64)>> {
        shell::recycle_bin_info().map(Some)
    }

    fn move_to_trash(&self, paths: &[PathBuf]) -> Result<()> {
        crate::move_to_trash(paths)
    }

    fn empty_trash(&self) -> Result<()> {
        shell::empty_recycle_bin()
    }

    fn update_status(&self) -> Result<UpdateStatus> {
        updates::status()
    }

    fn drivers(&self) -> Result<Vec<DriverInfo>> {
        drivers::list()
    }

    fn open_system_page(&self, page: SystemPage) -> Result<()> {
        let target = match page {
            SystemPage::SystemUpdate => "ms-settings:windowsupdate",
            SystemPage::OptionalUpdates => "ms-settings:windowsupdate-optionalupdates",
            SystemPage::DeviceManager => "devmgmt.msc",
            SystemPage::LoginItems => "ms-settings:startupapps",
            SystemPage::FullDiskAccess => return Err(trufi_core::Error::Unsupported("full disk access")),
        };
        shell::shell_execute("open", target, "")
    }
}

/// Folders under AppData/ProgramData that belong to Windows, drivers,
/// runtimes or tools that do not register as installed programs.
const LEFTOVER_IGNORE: &[&str] = &[
    "Microsoft", "Windows", "Packages", "Temp", "Programs", "Intel", "NVIDIA",
    "NVIDIA Corporation", "AMD", "ATI", "Realtek", "Common Files", "Comms",
    "ConnectedDevicesPlatform", "CrashDumps", "D3DSCache", "PeerDistRepub",
    "VirtualStore", "History", "Publishers", "PlaceholderTileLogoFolder",
    "Package Cache", "regid.1991-06.com.microsoft", "SoftwareDistribution",
    "ssh", "USOPrivate", "USOShared", "Desktop", "Documents", "Start Menu",
    "Templates", "Application Data", "Microsoft Help", "MicrosoftEdge", "Microsoft OneDrive",
    "WindowsHolographicDevices", "Sun", "Oracle", "Java", "npm", "npm-cache", "pip",
    "pnpm", "pnpm-cache", "Yarn", "NuGet", "go-build", "electron", "electron-builder",
    "ms-playwright", "JetBrains", "Squirrel", "SquirrelTemp", "Package Management",
    "IsolatedStorage", "fontconfig", "Claude", "Claude-Data", "AnthropicClaude",
    "PrismLauncher", "uv", "DBG", "Downloaded Installations", "InstallShield",
    "Uninstall Information", "CEF", "UnrealEngine", "CrashReportClient",
];
