//! Things macOS cleans through its own tools: local Time Machine
//! snapshots, Xcode simulators whose runtime is gone, Docker build cache.

use super::{apps, cmd};
use std::path::{Path, PathBuf};
use std::time::Duration;
use trufi_core::{Error, Result};

// ---------- Time Machine local snapshots ----------

/// Snapshot dates from `tmutil listlocalsnapshots /`
/// (`com.apple.TimeMachine.2026-10-30-120000.local`).
pub fn parse_snapshots(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| l.trim().strip_prefix("com.apple.TimeMachine."))
        .filter_map(|r| r.strip_suffix(".local"))
        .filter(|d| !d.is_empty() && d.chars().all(|c| c.is_ascii_digit() || c == '-'))
        .map(String::from)
        .collect()
}

fn snapshots() -> Vec<String> {
    cmd::run_ok("/usr/bin/tmutil", &["listlocalsnapshots", "/"], Duration::from_secs(30))
        .map(|t| parse_snapshots(&t))
        .unwrap_or_default()
}

pub fn tm_info() -> Option<(u64, u64)> {
    let n = snapshots().len() as u64;
    // macOS does not report how much space a snapshot holds.
    (n > 0).then_some((0, n))
}

pub fn tm_clean() -> Result<()> {
    let dates = snapshots();
    if dates.is_empty() {
        return Ok(());
    }
    // Dates are digits and dashes only (see parse_snapshots).
    let script = dates
        .iter()
        .map(|d| format!("/usr/bin/tmutil deletelocalsnapshots {d}"))
        .collect::<Vec<_>>()
        .join("; ");
    cmd::osascript(
        &[r#"do shell script (item 1 of argv) with administrator privileges"#],
        &[&script],
        Duration::from_secs(600),
    )
    .map(|_| ())
}

// ---------- Xcode simulators ----------

/// `xcrun` without Xcode pops up the "install developer tools" dialog, so
/// it is only run when Xcode itself is selected.
fn xcode_present() -> bool {
    cmd::run_ok("/usr/bin/xcode-select", &["-p"], Duration::from_secs(10))
        .is_ok_and(|p| p.contains(".app/"))
}

/// Device folders of unavailable simulators from
/// `xcrun simctl list devices unavailable -j`.
pub fn parse_unavailable(json: &str) -> Vec<PathBuf> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else { return Vec::new() };
    let Some(devices) = v.get("devices").and_then(|d| d.as_object()) else { return Vec::new() };
    devices
        .values()
        .filter_map(|list| list.as_array())
        .flatten()
        .filter(|d| d.get("isAvailable").and_then(|a| a.as_bool()) == Some(false))
        .filter_map(|d| d.get("dataPath").and_then(|p| p.as_str()))
        .filter_map(|p| Path::new(p).parent().map(Path::to_path_buf))
        .collect()
}

pub fn simulators_info() -> Option<(u64, u64)> {
    if !xcode_present() {
        return None;
    }
    let json = cmd::run_ok("/usr/bin/xcrun", &["simctl", "list", "devices", "unavailable", "-j"], Duration::from_secs(60)).ok()?;
    let dirs = parse_unavailable(&json);
    if dirs.is_empty() {
        return None;
    }
    let bytes = dirs.iter().map(|d| apps::tree_size(d)).sum();
    Some((bytes, dirs.len() as u64))
}

pub fn simulators_clean() -> Result<()> {
    cmd::run_ok("/usr/bin/xcrun", &["simctl", "delete", "unavailable"], Duration::from_secs(300)).map(|_| ())
}

// ---------- Docker build cache ----------

fn docker(home: &Path) -> Option<PathBuf> {
    [
        PathBuf::from("/usr/local/bin/docker"),
        PathBuf::from("/opt/homebrew/bin/docker"),
        home.join(".docker/bin/docker"),
        PathBuf::from("/Applications/Docker.app/Contents/Resources/bin/docker"),
    ]
    .into_iter()
    .find(|p| p.exists())
}

/// Docker's sizes are decimal: "1.234GB", "512MB", "12.5kB", "0B".
pub fn parse_docker_size(s: &str) -> u64 {
    let s = s.trim();
    let split = s.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(s.len());
    let n: f64 = s[..split].parse().unwrap_or(0.0);
    let mult = match s[split..].trim().to_ascii_uppercase().as_str() {
        "KB" => 1e3,
        "MB" => 1e6,
        "GB" => 1e9,
        "TB" => 1e12,
        _ => 1.0,
    };
    (n * mult) as u64
}

/// Reclaimable build cache from `docker system df --format '{{json .}}'`.
pub fn parse_docker_df(text: &str) -> Option<(u64, u64)> {
    text.lines().find_map(|l| {
        let v: serde_json::Value = serde_json::from_str(l).ok()?;
        if v.get("Type")?.as_str()? != "Build Cache" {
            return None;
        }
        let reclaimable = v.get("Reclaimable")?.as_str()?;
        // "1.2GB" or "1.2GB (100%)"
        let bytes = parse_docker_size(reclaimable.split_whitespace().next().unwrap_or("0B"));
        let count = v.get("TotalCount").and_then(|c| c.as_str()).and_then(|c| c.parse().ok()).unwrap_or(0);
        Some((bytes, count))
    })
}

pub fn docker_info(home: &Path) -> Option<(u64, u64)> {
    let bin = docker(home)?;
    // Fails quickly when Docker Desktop is not running: nothing to report.
    let out = cmd::run_ok(&bin.to_string_lossy(), &["system", "df", "--format", "{{json .}}"], Duration::from_secs(20)).ok()?;
    parse_docker_df(&out).filter(|(b, _)| *b > 0)
}

pub fn docker_clean(home: &Path) -> Result<()> {
    let bin = docker(home).ok_or_else(|| Error::Platform("Docker не найден.".into()))?;
    cmd::run_ok(&bin.to_string_lossy(), &["builder", "prune", "-f"], Duration::from_secs(600)).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_snapshots() {
        let out = "Snapshots for disk /:\ncom.apple.TimeMachine.2026-10-30-120000.local\ncom.apple.TimeMachine.2026-10-31-090102.local\ncom.apple.os.update-ABC\ncom.apple.TimeMachine.bad;rm.local\n";
        assert_eq!(parse_snapshots(out), ["2026-10-30-120000", "2026-10-31-090102"]);
    }

    #[test]
    fn parses_unavailable_simulators() {
        let json = r#"{"devices":{"com.apple.CoreSimulator.SimRuntime.iOS-16-0":[
            {"dataPath":"/Users/u/Library/Developer/CoreSimulator/Devices/AAA/data","isAvailable":false,"name":"iPhone 14"},
            {"dataPath":"/Users/u/Library/Developer/CoreSimulator/Devices/BBB/data","isAvailable":true,"name":"iPhone 15"}]}}"#;
        assert_eq!(parse_unavailable(json), [PathBuf::from("/Users/u/Library/Developer/CoreSimulator/Devices/AAA")]);
        assert!(parse_unavailable("not json").is_empty());
    }

    #[test]
    fn parses_docker() {
        assert_eq!(parse_docker_size("1.5GB"), 1_500_000_000);
        assert_eq!(parse_docker_size("12.5kB"), 12_500);
        assert_eq!(parse_docker_size("0B"), 0);
        let df = r#"{"Active":"2","Reclaimable":"1.2GB (100%)","Size":"1.2GB","TotalCount":"5","Type":"Images"}
{"Active":"0","Reclaimable":"3.4GB","Size":"3.4GB","TotalCount":"42","Type":"Build Cache"}"#;
        assert_eq!(parse_docker_df(df), Some((3_400_000_000, 42)));
    }
}
