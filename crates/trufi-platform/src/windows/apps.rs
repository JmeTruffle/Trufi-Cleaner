//! Installed programs from the Uninstall registry keys.

use super::expand_env;
use crate::split_command;
use std::path::{Path, PathBuf};
use trufi_core::model::InstalledApp;
use trufi_core::{Error, Result};
use winreg::enums::*;
use winreg::RegKey;

const UNINSTALL: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";

/// Registry views that hold Uninstall entries, by id prefix.
const SOURCES: &[&str] = &["HKLM64", "HKLM32", "HKCU"];

fn open(prefix: &str) -> Option<RegKey> {
    let (hive, view) = match prefix {
        "HKLM64" => (HKEY_LOCAL_MACHINE, KEY_WOW64_64KEY),
        "HKLM32" => (HKEY_LOCAL_MACHINE, KEY_WOW64_32KEY),
        "HKCU" => (HKEY_CURRENT_USER, 0),
        _ => return None,
    };
    RegKey::predef(hive)
        .open_subkey_with_flags(UNINSTALL, KEY_READ | view)
        .ok()
}

fn string(key: &RegKey, name: &str) -> Option<String> {
    key.get_value::<String, _>(name)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn path_missing(p: &Path) -> bool {
    p.is_absolute() && !p.exists()
}

/// Steam app ids that are installed in some library (`appmanifest_<id>.acf`).
fn steam_installed(libraries: &[PathBuf]) -> Vec<(String, InstalledApp)> {
    let mut out = Vec::new();
    for lib in libraries {
        let Ok(rd) = std::fs::read_dir(lib.join("steamapps")) else { continue };
        for e in rd.flatten() {
            let file = e.file_name().to_string_lossy().into_owned();
            let Some(id) = file.strip_prefix("appmanifest_").and_then(|f| f.strip_suffix(".acf")) else {
                continue;
            };
            let text = std::fs::read_to_string(e.path()).unwrap_or_default();
            let field = |key: &str| {
                text.lines().find_map(|l| {
                    let parts: Vec<&str> = l.split('"').filter(|p| !p.trim().is_empty()).collect();
                    match parts.as_slice() {
                        [k, v] if k.eq_ignore_ascii_case(key) => Some(v.to_string()),
                        _ => None,
                    }
                })
            };
            let dir = field("installdir");
            out.push((
                id.to_string(),
                InstalledApp {
                    id: format!("steam:{id}"),
                    name: field("name").unwrap_or_else(|| format!("Steam App {id}")),
                    publisher: Some("Steam".into()),
                    install_location: dir.map(|d| lib.join(r"steamapps\common").join(d)),
                    ..Default::default()
                },
            ));
        }
    }
    out
}

pub fn installed_apps() -> Vec<InstalledApp> {
    let steam_games = steam_path().map(|s| steam_installed(&steam_libraries(&s))).unwrap_or_default();
    let mut apps: Vec<InstalledApp> = Vec::new();
    for prefix in SOURCES {
        let Some(root) = open(prefix) else { continue };
        for sub in root.enum_keys().flatten() {
            let Ok(key) = root.open_subkey(&sub) else { continue };
            let Some(name) = string(&key, "DisplayName") else { continue };
            let system = key.get_value::<u32, _>("SystemComponent").unwrap_or(0) == 1;
            let is_update = string(&key, "ParentKeyName").is_some()
                || string(&key, "ReleaseType").is_some_and(|t| t.contains("Update") || t == "Hotfix");
            if system || is_update {
                continue;
            }
            let install_location = string(&key, "InstallLocation")
                .map(|s| PathBuf::from(expand_env(s.trim_matches('"'))));
            let uninstall = string(&key, "UninstallString");
            let uninstaller_missing = uninstall.as_deref().is_some_and(|u| {
                let (exe, _) = split_command(&expand_env(u));
                !exe.to_ascii_lowercase().ends_with("msiexec.exe") && path_missing(Path::new(&exe))
            });
            // Steam keeps the original path in the registry when a game is
            // moved to another library, so ask Steam's manifests instead.
            let steam_game = sub.strip_prefix("Steam App ").and_then(|id| steam_games.iter().find(|g| g.0 == id));
            let broken = match steam_game {
                Some(_) => false,
                None => install_location.as_deref().is_some_and(path_missing) || uninstaller_missing,
            };
            let install_location = steam_game.and_then(|g| g.1.install_location.clone()).or(install_location);
            apps.push(InstalledApp {
                id: format!("{prefix}\\{sub}"),
                name,
                publisher: string(&key, "Publisher"),
                version: string(&key, "DisplayVersion"),
                install_location,
                size_bytes: key
                    .get_value::<u32, _>("EstimatedSize")
                    .ok()
                    .map(|kb| kb as u64 * 1024),
                broken,
                can_uninstall: uninstall.is_some() && !uninstaller_missing,
            });
        }
    }
    // Games Steam knows about but the registry does not (helps leftover
    // detection recognise their data folders).
    for (_, game) in steam_games {
        if !apps.iter().any(|a| a.name.eq_ignore_ascii_case(&game.name)) {
            apps.push(game);
        }
    }
    // The same product often appears in several views.
    apps.sort_by_key(|x| x.name.to_lowercase());
    apps.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name) && a.version == b.version);
    apps
}

pub fn uninstall(id: &str) -> Result<()> {
    let (prefix, sub) = id
        .split_once('\\')
        .ok_or_else(|| Error::Platform(format!("bad app id {id}")))?;
    let key = open(prefix)
        .and_then(|r| r.open_subkey(sub).ok())
        .ok_or_else(|| Error::Platform("программа не найдена".into()))?;
    let cmd = string(&key, "UninstallString")
        .ok_or_else(|| Error::Platform("у программы нет деинсталлятора".into()))?;
    let (exe, args) = split_command(&expand_env(&cmd));
    // ShellExecute (unlike CreateProcess) shows the UAC prompt when the
    // uninstaller requires elevation.
    super::shell::shell_execute("open", &exe, &args)
}

pub fn steam_path() -> Option<PathBuf> {
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Valve\Steam")
        .ok()?;
    let p: PathBuf = string(&key, "SteamPath")?.replace('/', "\\").into();
    p.exists().then_some(p)
}

/// Steam library folders (the install itself plus extra drives), from
/// `steamapps/libraryfolders.vdf`.
pub fn steam_libraries(steam: &Path) -> Vec<PathBuf> {
    let mut libs = vec![steam.to_path_buf()];
    let Ok(vdf) = std::fs::read_to_string(steam.join(r"steamapps\libraryfolders.vdf")) else {
        return libs;
    };
    for line in vdf.lines() {
        let parts: Vec<&str> = line.split('"').filter(|p| !p.trim().is_empty()).collect();
        if let ["path", value] = parts.as_slice() {
            let p = PathBuf::from(value.replace(r"\\", r"\"));
            if p.exists() && !libs.iter().any(|l| l.as_os_str().eq_ignore_ascii_case(p.as_os_str())) {
                libs.push(p);
            }
        }
    }
    libs
}
