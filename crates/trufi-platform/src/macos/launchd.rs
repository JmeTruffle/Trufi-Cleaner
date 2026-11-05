//! Startup items on macOS: launchd agents and daemons installed by
//! applications. Turning one off uses `launchctl disable`, the same
//! override switch System Settings uses: the plist stays where it is and
//! `launchctl enable` brings it back. The change applies at the next login
//! or restart; running processes are left alone.
//!
//! Apps registered as Login Items through System Settings or SMAppService
//! are not listed here (macOS shows them only to root); the UI links to
//! System Settings for those.

use super::cmd;
use plist::Value;
use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use trufi_core::model::{StartupItem, StartupScope};
use trufi_core::{Error, Result};

const TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    /// Agents run in the user's GUI session; the user may switch them.
    Gui,
    /// Daemons run as root; switching them asks for the admin password.
    System,
}

impl Domain {
    fn as_str(self) -> &'static str {
        match self {
            Domain::Gui => "gui",
            Domain::System => "system",
        }
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct Job {
    pub label: String,
    pub args: Vec<String>,
    pub disabled: bool,
}

pub fn parse_job(value: &Value) -> Option<Job> {
    let d = value.as_dictionary()?;
    let label = d.get("Label")?.as_string()?.trim().to_string();
    let mut args: Vec<String> = d
        .get("ProgramArguments")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_string).map(String::from).collect())
        .unwrap_or_default();
    if let Some(program) = d.get("Program").and_then(Value::as_string) {
        if args.first().map(String::as_str) != Some(program) {
            args.insert(0, program.to_string());
        }
    }
    let disabled = d.get("Disabled").and_then(Value::as_boolean).unwrap_or(false);
    (!label.is_empty()).then_some(Job { label, args, disabled })
}

/// Parses `launchctl print-disabled`: label -> disabled. Recent macOS
/// prints `=> disabled` / `=> enabled`, older versions `=> true` / `=> false`.
pub fn parse_overrides(text: &str) -> HashMap<String, bool> {
    text.lines()
        .filter_map(|line| {
            let (label, state) = line.split_once("=>")?;
            let label = label.trim().trim_matches('"');
            let disabled = match state.trim() {
                "disabled" | "true" => true,
                "enabled" | "false" => false,
                _ => return None,
            };
            (!label.is_empty()).then(|| (label.to_string(), disabled))
        })
        .collect()
}

/// A name a person recognises: the app the job belongs to, else the
/// program's file name, else the label.
pub fn display_name(label: &str, args: &[String]) -> String {
    if let Some(program) = args.first() {
        let p = Path::new(program);
        let app = p
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .find(|c| c.to_lowercase().ends_with(".app"));
        if let Some(app) = app {
            return app[..app.len() - 4].to_string();
        }
        if let Some(name) = p.file_name() {
            return name.to_string_lossy().into_owned();
        }
    }
    label.to_string()
}

/// Labels are reverse-DNS names; anything else is refused before it gets
/// near a command line.
pub fn valid_label(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= 255
        && label.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

pub fn uid(home: &Path) -> Option<u32> {
    std::fs::metadata(home).ok().map(|m| m.uid())
}

fn overrides(domain: &str) -> HashMap<String, bool> {
    cmd::run_ok("/bin/launchctl", &["print-disabled", domain], TIMEOUT)
        .map(|t| parse_overrides(&t))
        .unwrap_or_default()
}

fn sources(home: &Path) -> [(PathBuf, Domain, StartupScope); 3] {
    [
        (home.join("Library/LaunchAgents"), Domain::Gui, StartupScope::CurrentUser),
        (PathBuf::from("/Library/LaunchAgents"), Domain::Gui, StartupScope::AllUsers),
        (PathBuf::from("/Library/LaunchDaemons"), Domain::System, StartupScope::AllUsers),
    ]
}

pub fn list(home: &Path) -> Vec<StartupItem> {
    let gui = uid(home).map(|u| overrides(&format!("gui/{u}"))).unwrap_or_default();
    let system = overrides("system");
    let mut items = Vec::new();
    for (dir, domain, scope) in sources(home) {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let path = e.path();
            if path.extension().is_none_or(|x| x != "plist") {
                continue;
            }
            let Some(job) = Value::from_file(&path).ok().as_ref().and_then(parse_job) else { continue };
            if job.label.starts_with("com.apple.") || !valid_label(&job.label) {
                continue;
            }
            let over = if domain == Domain::System { &system } else { &gui };
            let enabled = over.get(&job.label).map(|d| !d).unwrap_or(!job.disabled);
            let target_exists = job
                .args
                .first()
                .map(|p| !p.starts_with('/') || Path::new(p).exists())
                .unwrap_or(true);
            items.push(StartupItem {
                // The plist path rides along so the entry can be removed.
                id: format!("{}|{}|{}", domain.as_str(), job.label, path.display()),
                name: display_name(&job.label, &job.args),
                command: if job.args.is_empty() { job.label.clone() } else { job.args.join(" ") },
                source: if domain == Domain::System { "launch_daemon" } else { "launch_agent" }.into(),
                scope,
                enabled,
                requires_admin: domain == Domain::System,
                target_exists,
            });
        }
    }
    items.sort_by_key(|i| i.name.to_lowercase());
    items
}

/// Splits `domain|label|plist` (the plist part is optional for ids made
/// by older versions).
fn parse_id(id: &str) -> Result<(&str, &str, Option<&str>)> {
    let bad = || Error::Platform(format!("Неизвестная запись автозагрузки: {id}"));
    let mut it = id.splitn(3, '|');
    let domain = it.next().ok_or_else(bad)?;
    let label = it.next().ok_or_else(bad)?;
    if !valid_label(label) {
        return Err(bad());
    }
    Ok((domain, label, it.next()))
}

pub fn set_enabled(id: &str, enabled: bool, home: &Path) -> Result<()> {
    let (domain, label, _) = parse_id(id)?;
    let action = if enabled { "enable" } else { "disable" };
    match domain {
        "gui" => {
            let uid = uid(home).ok_or_else(|| Error::Platform("Не удалось определить пользователя.".into()))?;
            cmd::run_ok("/bin/launchctl", &[action, &format!("gui/{uid}/{label}")], TIMEOUT)?;
        }
        "system" => {
            // macOS shows its own password prompt.
            cmd::osascript(
                &[r#"do shell script "/bin/launchctl " & (item 1 of argv) & " system/" & quoted form of (item 2 of argv) with administrator privileges"#],
                &[action, label],
                Duration::from_secs(300),
            )?;
        }
        _ => return Err(Error::Platform(format!("Неизвестная запись автозагрузки: {id}"))),
    }
    Ok(())
}

/// The plist an id points at, if it is really one of ours to remove: in a
/// launchd folder, a .plist, and declaring that very label.
pub fn plist_for_removal(id: &str, home: &Path) -> Result<(Domain, String, PathBuf)> {
    let (domain, label, plist) = parse_id(id)?;
    let bad = || Error::Platform(format!("Неизвестная запись автозагрузки: {id}"));
    let path = PathBuf::from(plist.ok_or_else(bad)?);
    let domain = match domain {
        "gui" => Domain::Gui,
        "system" => Domain::System,
        _ => return Err(bad()),
    };
    let in_source = sources(home)
        .iter()
        .any(|(dir, d, _)| *d == domain && path.parent() == Some(dir.as_path()));
    if !in_source || path.extension().is_none_or(|x| x != "plist") {
        return Err(bad());
    }
    let job = Value::from_file(&path).ok().as_ref().and_then(parse_job).ok_or_else(bad)?;
    if job.label != label {
        return Err(bad());
    }
    Ok((domain, label.to_string(), path))
}

/// Stops the job and moves its plist to the Trash, so it can be put back.
/// Plists outside the home folder belong to root: macOS asks for the
/// administrator password.
pub fn remove(id: &str, home: &Path) -> Result<()> {
    let (domain, label, path) = plist_for_removal(id, home)?;
    let uid = uid(home).ok_or_else(|| Error::Platform("Не удалось определить пользователя.".into()))?;
    let target = match domain {
        Domain::Gui => format!("gui/{uid}/{label}"),
        Domain::System => format!("system/{label}"),
    };
    if path.starts_with(home) {
        // Not loaded (its program is gone) is fine.
        let _ = cmd::run("/bin/launchctl", &["bootout", &target], TIMEOUT);
        return crate::move_to_trash(std::slice::from_ref(&path));
    }
    if domain == Domain::Gui {
        let _ = cmd::run("/bin/launchctl", &["bootout", &target], TIMEOUT);
    }
    let trash = home.join(".Trash");
    cmd::osascript(
        &[r#"do shell script "/bin/launchctl bootout " & quoted form of (item 1 of argv) & " 2>/dev/null; /bin/mv -f " & quoted form of (item 2 of argv) & " " & quoted form of (item 3 of argv) with administrator privileges"#],
        &[&target, &path.to_string_lossy(), &trash.to_string_lossy()],
        Duration::from_secs(300),
    )
    .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removal_only_touches_real_launchd_plists() {
        let home = tempfile::tempdir().unwrap();
        let agents = home.path().join("Library/LaunchAgents");
        std::fs::create_dir_all(&agents).unwrap();
        let plist = agents.join("com.gone.helper.plist");
        std::fs::write(
            &plist,
            r#"<plist version="1.0"><dict><key>Label</key><string>com.gone.helper</string><key>Program</key><string>/Applications/Gone.app/Contents/MacOS/Gone</string></dict></plist>"#,
        )
        .unwrap();
        let id = format!("gui|com.gone.helper|{}", plist.display());
        let (domain, label, path) = plist_for_removal(&id, home.path()).unwrap();
        assert_eq!((domain, label.as_str(), path), (Domain::Gui, "com.gone.helper", plist.clone()));
        // Wrong label, wrong folder, wrong domain, old id without a path.
        assert!(plist_for_removal(&format!("gui|com.other|{}", plist.display()), home.path()).is_err());
        assert!(plist_for_removal("gui|com.gone.helper|/etc/hosts", home.path()).is_err());
        assert!(plist_for_removal(&format!("system|com.gone.helper|{}", plist.display()), home.path()).is_err());
        assert!(plist_for_removal("gui|com.gone.helper", home.path()).is_err());
        // set_enabled still understands both id forms.
        assert!(parse_id(&id).is_ok() && parse_id("gui|com.x").is_ok());
    }

    #[test]
    fn parses_jobs() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
  <key>Label</key><string>com.google.keystone.agent</string>
  <key>ProgramArguments</key><array>
    <string>/Users/u/Library/Google/GoogleSoftwareUpdate/GoogleSoftwareUpdate.bundle/Contents/Resources/GoogleSoftwareUpdateAgent.app/Contents/MacOS/GoogleSoftwareUpdateAgent</string>
    <string>-runMode</string><string>ifneeded</string>
  </array>
  <key>RunAtLoad</key><true/>
</dict></plist>"#;
        let job = parse_job(&Value::from_reader_xml(xml.as_bytes()).unwrap()).unwrap();
        assert_eq!(job.label, "com.google.keystone.agent");
        assert_eq!(job.args.len(), 3);
        assert!(!job.disabled);
        assert_eq!(display_name(&job.label, &job.args), "GoogleSoftwareUpdateAgent");

        let xml = r#"<plist version="1.0"><dict>
  <key>Label</key><string>org.example.tool</string>
  <key>Program</key><string>/usr/local/bin/tool</string>
  <key>Disabled</key><true/>
</dict></plist>"#;
        let job = parse_job(&Value::from_reader_xml(xml.as_bytes()).unwrap()).unwrap();
        assert_eq!(job.args, ["/usr/local/bin/tool"]);
        assert!(job.disabled);
        assert_eq!(display_name(&job.label, &job.args), "tool");
        assert_eq!(display_name("com.x.y", &[]), "com.x.y");
    }

    #[test]
    fn parses_overrides_old_and_new() {
        let text = "disabled services = {\n\t\"com.docker.helper\" => disabled\n\t\"us.zoom.updater\" => enabled\n\t\"com.old\" => true\n}\n";
        let o = parse_overrides(text);
        assert_eq!(o.get("com.docker.helper"), Some(&true));
        assert_eq!(o.get("us.zoom.updater"), Some(&false));
        assert_eq!(o.get("com.old"), Some(&true));
        assert_eq!(o.len(), 3);
    }

    #[test]
    fn validates_labels() {
        assert!(valid_label("com.docker.vmnetd"));
        assert!(valid_label("us.zoom.ZoomDaemon_v2-x"));
        assert!(!valid_label("a\"; rm -rf ~"));
        assert!(!valid_label("x/y"));
        assert!(!valid_label(""));
        assert!(set_enabled("system|bad label", true, Path::new("/")).is_err());
        assert!(set_enabled("other|com.x", true, Path::new("/")).is_err());
    }
}
