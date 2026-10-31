//! Login Items: apps that open when the user logs in, as System Events
//! lists them (added in System Settings or by the app through the legacy
//! API). Apps registered with SMAppService are shown only to root; the UI
//! links to System Settings for those.
//!
//! macOS has no "disabled" state for these, so turning one off removes it
//! and remembers its path and "hidden" flag in our own file; turning it
//! back on adds it again. That keeps the switch reversible like every
//! other startup item.

use super::cmd;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use trufi_core::model::{StartupItem, StartupScope};
use trufi_core::{Error, Result};

const TIMEOUT: Duration = Duration::from_secs(30);
pub const ID_PREFIX: &str = "login|";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoginItem {
    pub name: String,
    pub path: String,
    pub hidden: bool,
}

/// Parses the tab-separated lines the listing script prints.
pub fn parse_list(text: &str) -> Vec<LoginItem> {
    text.lines()
        .filter_map(|l| {
            let mut f = l.split('\t');
            let name = f.next()?.trim().to_string();
            let path = f.next()?.trim().to_string();
            let hidden = f.next().map(str::trim) == Some("true");
            (!path.is_empty() && path != "missing value").then_some(LoginItem { name, path, hidden })
        })
        .collect()
}

fn store_path(data_dir: &Path) -> PathBuf {
    data_dir.join("disabled-login-items.json")
}

fn load_disabled(data_dir: &Path) -> Vec<LoginItem> {
    std::fs::read(store_path(data_dir))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn save_disabled(data_dir: &Path, items: &[LoginItem]) -> Result<()> {
    std::fs::create_dir_all(data_dir)?;
    let json = serde_json::to_vec_pretty(items).map_err(|e| Error::Platform(e.to_string()))?;
    std::fs::write(store_path(data_dir), json)?;
    Ok(())
}

fn system_events_list() -> Result<Vec<LoginItem>> {
    let out = cmd::osascript(
        &[
            r#"set out to """#,
            r#"tell application "System Events""#,
            r#"repeat with li in login items"#,
            r#"set out to out & (name of li) & tab & (path of li) & tab & (hidden of li) & linefeed"#,
            r#"end repeat"#,
            r#"end tell"#,
            r#"return out"#,
        ],
        &[],
        TIMEOUT,
    )?;
    Ok(parse_list(&out))
}

fn to_item(li: &LoginItem, enabled: bool) -> StartupItem {
    StartupItem {
        id: format!("{ID_PREFIX}{}", li.path),
        name: if li.name.is_empty() {
            Path::new(&li.path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
        } else {
            li.name.trim_end_matches(".app").to_string()
        },
        command: li.path.clone(),
        source: "login_item".into(),
        scope: StartupScope::CurrentUser,
        enabled,
        requires_admin: false,
        target_exists: Path::new(&li.path).exists(),
    }
}

/// Active login items plus the ones we turned off. Errors (the user did not
/// allow controlling System Events) leave the list empty rather than
/// hiding launchd items too.
pub fn list(data_dir: &Path) -> Vec<StartupItem> {
    let active = match system_events_list() {
        Ok(a) => a,
        Err(e) => {
            log::warn!("login items: {e}");
            Vec::new()
        }
    };
    let mut items: Vec<StartupItem> = active.iter().map(|li| to_item(li, true)).collect();
    for li in load_disabled(data_dir) {
        if !active.iter().any(|a| a.path == li.path) {
            items.push(to_item(&li, false));
        }
    }
    items
}

pub fn set_enabled(id: &str, enabled: bool, data_dir: &Path) -> Result<()> {
    let path = id.strip_prefix(ID_PREFIX).ok_or_else(|| Error::Platform(format!("Неизвестная запись: {id}")))?;
    let mut disabled = load_disabled(data_dir);
    if enabled {
        let li = disabled
            .iter()
            .find(|l| l.path == path)
            .cloned()
            .unwrap_or(LoginItem { name: String::new(), path: path.to_string(), hidden: false });
        if !Path::new(&li.path).exists() {
            return Err(Error::Platform(format!("Программа не найдена: {}", li.path)));
        }
        cmd::osascript(
            &[r#"tell application "System Events" to make login item at end with properties {path:(item 1 of argv), hidden:((item 2 of argv) is "true")}"#],
            &[&li.path, if li.hidden { "true" } else { "false" }],
            TIMEOUT,
        )?;
        disabled.retain(|l| l.path != path);
    } else {
        let current = system_events_list()?;
        let li = current
            .into_iter()
            .find(|l| l.path == path)
            .ok_or_else(|| Error::Platform("Объект входа уже удалён.".into()))?;
        cmd::osascript(
            &[r#"tell application "System Events" to delete (every login item whose path is (item 1 of argv))"#],
            &[path],
            TIMEOUT,
        )?;
        disabled.retain(|l| l.path != path);
        disabled.push(li);
    }
    save_disabled(data_dir, &disabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_system_events_output() {
        let out = "Telegram\t/Applications/Telegram.app\tfalse\nDropbox\t/Applications/Dropbox.app\ttrue\nBroken\tmissing value\tfalse\n";
        let items = parse_list(out);
        assert_eq!(items.len(), 2);
        assert_eq!(items[1], LoginItem { name: "Dropbox".into(), path: "/Applications/Dropbox.app".into(), hidden: true });
    }

    #[test]
    fn remembers_disabled_items() {
        let dir = tempfile::tempdir().unwrap();
        let li = LoginItem { name: "Telegram".into(), path: "/Applications/Telegram.app".into(), hidden: false };
        save_disabled(dir.path(), std::slice::from_ref(&li)).unwrap();
        assert_eq!(load_disabled(dir.path()), vec![li.clone()]);
        let item = to_item(&li, false);
        assert_eq!(item.id, "login|/Applications/Telegram.app");
        assert!(!item.enabled);
        assert!(set_enabled("bad", true, dir.path()).is_err());
    }
}
