//! macOS Software Update: the installed version, what `softwareupdate`
//! offers, and when updates were last installed
//! (`/Library/Receipts/InstallHistory.plist`, readable by every user).

use super::cmd;
use plist::Value;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use trufi_core::updates::{macos_name, macos_out_of_support, OsVersion, PendingUpdate, UpdateStatus};
use trufi_core::util::{days_of, format_days};
use trufi_core::Result;

/// `softwareupdate --list` asks Apple's servers and can take a while.
const LIST_TIMEOUT: Duration = Duration::from_secs(180);
const HISTORY: &str = "/Library/Receipts/InstallHistory.plist";

/// Background security data (XProtect, MRT, Gatekeeper rules...): macOS
/// installs it silently, it says nothing about how up to date the system is.
fn is_background_data(title: &str) -> bool {
    let t = title.to_ascii_lowercase();
    ["xprotect", "mrt", "gatekeeper", "configdata", "config data", "compatibility", "kext exclude", "incompatible"]
        .iter()
        .any(|k| t.contains(k))
}

pub fn major_of(version: &str) -> u32 {
    version.split('.').next().and_then(|m| m.trim().parse().ok()).unwrap_or(0)
}

pub fn version() -> Option<OsVersion> {
    let v = sysinfo::System::os_version()?;
    let major = major_of(&v);
    let build = cmd::run_ok("/usr/bin/sw_vers", &["-buildVersion"], Duration::from_secs(10))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    Some(OsVersion {
        name: macos_name(major).into(),
        display_version: Some(v),
        build_label: build,
        ..Default::default()
    })
}

/// Size in `softwareupdate` notation: "5638616K", "1234567KiB".
fn parse_size(s: &str) -> u64 {
    let s = s.trim();
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    let n: u64 = digits.parse().unwrap_or(0);
    let unit = s[digits.len()..].trim().to_ascii_uppercase();
    match unit.chars().next() {
        Some('K') => n * 1024,
        Some('M') => n * 1024 * 1024,
        Some('G') => n * 1024 * 1024 * 1024,
        _ => n,
    }
}

/// Parses `softwareupdate --list` (macOS 10.15+ format):
///
/// ```text
/// * Label: macOS Sequoia 15.1.1-24B91
///     Title: macOS Sequoia 15.1.1, Version: 15.1.1, Size: 5638616K, Recommended: YES, Action: restart,
/// ```
pub fn parse_list(text: &str) -> Vec<PendingUpdate> {
    let mut out: Vec<PendingUpdate> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(label) = line.strip_prefix("* Label:") {
            out.push(PendingUpdate { id: label.trim().to_string(), title: label.trim().to_string(), ..Default::default() });
            continue;
        }
        let Some(u) = out.last_mut() else { continue };
        if !line.starts_with("Title:") {
            continue;
        }
        let mut recommended = false;
        for field in line.split(',') {
            let Some((k, v)) = field.split_once(':') else { continue };
            let v = v.trim();
            match k.trim() {
                "Title" if !v.is_empty() => u.title = v.to_string(),
                "Size" => u.size_bytes = parse_size(v),
                "Recommended" => recommended = v.eq_ignore_ascii_case("yes"),
                _ => {}
            }
        }
        u.optional = !recommended;
        u.definition = is_background_data(&u.title);
        // Apple ships security fixes in every recommended update and does
        // not rate them separately.
        if recommended && !u.definition {
            u.severity = Some("Important".into());
        }
    }
    out
}

/// Date ("YYYY-MM-DD") of the last real system update in the install
/// history: macOS updates, Safari, security responses. Background data and
/// App Store apps do not count.
pub fn parse_last_install(history: &Value) -> Option<String> {
    let entries = history.as_array()?;
    entries
        .iter()
        .filter_map(|e| {
            let d = e.as_dictionary()?;
            let name = d.get("displayName").and_then(Value::as_string).unwrap_or_default();
            let process = d.get("processName").and_then(Value::as_string).unwrap_or_default();
            let content = d.get("contentType").and_then(Value::as_string).unwrap_or_default();
            let system = process == "softwareupdated" || process == "Software Update" || name.starts_with("macOS");
            if !system || content == "config-data" || is_background_data(name) {
                return None;
            }
            let date: SystemTime = d.get("date")?.as_date()?.into();
            Some(days_of(date))
        })
        .max()
        .map(format_days)
}

pub fn status() -> Result<UpdateStatus> {
    let version = version();
    let major = version.as_ref().and_then(|v| v.display_version.as_deref()).map(major_of).unwrap_or(0);
    let mut status = UpdateStatus {
        version,
        out_of_support: macos_out_of_support(major),
        last_install: Value::from_file(Path::new(HISTORY)).ok().as_ref().and_then(parse_last_install),
        checked_at_ms: SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0),
        ..Default::default()
    };
    match cmd::run("/usr/sbin/softwareupdate", &["--list"], LIST_TIMEOUT) {
        Ok(o) => {
            status.pending = parse_list(&o.stdout);
            let err = o.stderr.trim();
            let nothing_new = err.contains("No new software available");
            let failed = !o.success || err.to_ascii_lowercase().contains("connect");
            if status.pending.is_empty() && !nothing_new && failed && !err.is_empty() {
                status.error = Some(err.lines().last().unwrap_or(err).to_string());
            }
        }
        Err(e) => status.error = Some(e.to_string()),
    }
    status.finish();
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = "Software Update Tool

Finding available software
Software Update found the following new or updated software:
* Label: Command Line Tools for Xcode-16.1
\tTitle: Command Line Tools for Xcode, Version: 16.1, Size: 744661KiB, Recommended: YES, 
* Label: macOS Sequoia 15.1.1-24B91
\tTitle: macOS Sequoia 15.1.1, Version: 15.1.1, Size: 1652590K, Recommended: YES, Action: restart, 
* Label: XProtectPlistConfigData_10_15-5280
\tTitle: XProtectPlistConfigData, Version: 5280, Size: 1234K, Recommended: NO, 
* Label: Safari Technology Preview
\tTitle: Safari Technology Preview, Version: 207, Size: 90000K, Recommended: NO, 
";

    #[test]
    fn parses_software_update_list() {
        let u = parse_list(LIST);
        assert_eq!(u.len(), 4);
        assert_eq!(u[1].id, "macOS Sequoia 15.1.1-24B91");
        assert_eq!(u[1].title, "macOS Sequoia 15.1.1");
        assert_eq!(u[1].size_bytes, 1_652_590 * 1024);
        assert!(u[1].is_security() && u[1].counts());
        assert!(u[2].definition && !u[2].counts());
        assert!(u[3].optional && !u[3].counts());
        assert!(parse_list("No new software available.\n").is_empty());
    }

    #[test]
    fn last_install_skips_background_data_and_apps() {
        let xml = r#"<plist version="1.0"><array>
  <dict><key>date</key><date>2026-03-02T10:00:00Z</date><key>displayName</key><string>macOS Sequoia 15.3.2</string><key>processName</key><string>softwareupdated</string></dict>
  <dict><key>date</key><date>2026-05-20T10:00:00Z</date><key>displayName</key><string>XProtectPayloads</string><key>processName</key><string>softwareupdated</string><key>contentType</key><string>config-data</string></dict>
  <dict><key>date</key><date>2026-06-01T10:00:00Z</date><key>displayName</key><string>Telegram</string><key>processName</key><string>appstoreagent</string></dict>
  <dict><key>date</key><date>2026-04-10T08:00:00Z</date><key>displayName</key><string>Safari</string><key>processName</key><string>softwareupdated</string></dict>
</array></plist>"#;
        let v = Value::from_reader_xml(xml.as_bytes()).unwrap();
        assert_eq!(parse_last_install(&v).as_deref(), Some("2026-04-10"));
    }

    #[test]
    fn majors() {
        assert_eq!(major_of("15.1.1"), 15);
        assert_eq!(major_of("10.15.7"), 10);
        assert_eq!(major_of(""), 0);
    }
}
