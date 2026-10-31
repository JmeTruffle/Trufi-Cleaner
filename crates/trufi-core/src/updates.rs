//! OS update state: the installed version, whether it is still serviced,
//! and what the update service (Windows Update, macOS Software Update) has
//! waiting. The platform layer fills [`UpdateStatus`]; this module only
//! interprets it.

use crate::util::{days_from_civil, format_days, parse_date_days, today_days};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OsVersion {
    /// "Windows 11", "Windows 10", "macOS Sequoia".
    pub name: String,
    /// Windows feature update ("24H2") or macOS version ("15.1.1").
    pub display_version: Option<String>,
    /// Windows build number; 0 on macOS.
    pub build: u32,
    /// Windows monthly patch level (the part after the dot in 26100.4351).
    pub revision: u32,
    /// Windows edition: "Professional", "Core", "Enterprise"... Empty on macOS.
    pub edition: String,
    /// Build as the OS shows it when it is not a number ("24B91" on macOS).
    #[serde(default)]
    pub build_label: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PendingUpdate {
    pub id: String,
    pub title: String,
    pub driver: bool,
    /// Offered only on request ("optional updates" in Settings; updates
    /// macOS does not recommend).
    pub optional: bool,
    /// Microsoft Defender definitions, XProtect/MRT data on macOS: installed
    /// silently, not worth a mention.
    pub definition: bool,
    /// MSRC rating: "Critical", "Important", "Moderate", "Low".
    pub severity: Option<String>,
    pub kb: Option<String>,
    pub size_bytes: u64,
    pub driver_class: Option<String>,
    pub driver_provider: Option<String>,
    pub driver_hardware_id: Option<String>,
    pub driver_date: Option<String>,
}

impl PendingUpdate {
    pub fn is_security(&self) -> bool {
        matches!(self.severity.as_deref(), Some("Critical" | "Important"))
    }

    /// A Windows/software update the user should install (not a driver,
    /// not optional, not a definition refresh).
    pub fn counts(&self) -> bool {
        !self.driver && !self.optional && !self.definition
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateStatus {
    pub version: Option<OsVersion>,
    /// Last day of security updates for this version, "YYYY-MM-DD".
    pub servicing_ends: Option<String>,
    pub out_of_support: bool,
    pub pending: Vec<PendingUpdate>,
    /// Installed updates wait for a restart.
    pub reboot_required: bool,
    /// Date of the last successful install, "YYYY-MM-DD".
    pub last_install: Option<String>,
    pub days_since_install: Option<u32>,
    /// Why the update search failed (offline, service disabled).
    pub error: Option<String>,
    pub checked_at_ms: u64,
}

impl UpdateStatus {
    /// Fills the fields derived from the raw data and the current date.
    pub fn finish(&mut self) {
        let today = today_days();
        if let Some(v) = &self.version {
            if let Some(end) = servicing_end(v.build, &v.edition) {
                self.out_of_support = today > end;
                self.servicing_ends = Some(format_days(end));
            }
        }
        self.days_since_install = self
            .last_install
            .as_deref()
            .and_then(parse_date_days)
            .map(|d| (today - d).max(0) as u32);
    }

    pub fn health(&self) -> UpdateHealth {
        let counted = self.pending.iter().filter(|u| u.counts());
        UpdateHealth {
            pending: counted.clone().count() as u32,
            pending_security: counted.filter(|u| u.is_security()).count() as u32,
            reboot_required: self.reboot_required,
            // A failed search says nothing about how stale the system is.
            days_since_install: self.days_since_install.filter(|_| self.error.is_none()),
            unsupported_build: self.version.as_ref().filter(|_| self.out_of_support).map(|v| v.build),
        }
    }
}

/// What the health score needs to know about updates.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct UpdateHealth {
    pub pending: u32,
    pub pending_security: u32,
    pub reboot_required: bool,
    pub days_since_install: Option<u32>,
    pub unsupported_build: Option<u32>,
}

/// End of security updates for Home/Pro editions of a feature update, by
/// build. Enterprise and Education get a year more. LTSC, IoT and Server
/// follow other calendars and are not judged. Unknown (newer) builds are
/// assumed to be serviced. Source: Microsoft lifecycle pages for
/// Windows 10/11 Home and Pro.
fn servicing_end(build: u32, edition: &str) -> Option<i64> {
    let e = edition.to_ascii_lowercase();
    if e.contains("enterprises") || e.contains("iot") || e.contains("server") {
        return None;
    }
    let long = e.starts_with("enterprise") || e.starts_with("education");
    let (home_pro, ent_edu) = match build {
        0 => return None,
        // Windows 10 ends with 22H2 for every edition.
        1..=21999 => ((2025, 10, 14), (2025, 10, 14)),
        22000 => ((2023, 10, 10), (2024, 10, 8)),
        22621 => ((2024, 10, 8), (2025, 10, 14)),
        22631 => ((2025, 11, 11), (2026, 11, 10)),
        26100 => ((2026, 10, 13), (2027, 10, 12)),
        26200 => ((2027, 10, 12), (2028, 10, 10)),
        _ => return None,
    };
    let (y, m, d) = if long { ent_edu } else { home_pro };
    Some(days_from_civil(y, m, d))
}

/// Oldest macOS major version that still gets security updates. Apple
/// publishes no end dates; in practice it patches the current release and
/// the two before it, and drops the oldest when a new one ships in the
/// autumn. Bump this when that happens (macOS 26 Tahoe, autumn 2025, ended
/// Ventura).
pub const MACOS_OLDEST_SUPPORTED: u32 = 14;

/// Marketing name for a macOS major version.
pub fn macos_name(major: u32) -> &'static str {
    match major {
        11 => "macOS Big Sur",
        12 => "macOS Monterey",
        13 => "macOS Ventura",
        14 => "macOS Sonoma",
        15 => "macOS Sequoia",
        26 => "macOS Tahoe",
        _ => "macOS",
    }
}

/// Whether a macOS major version no longer gets security updates. 10.x
/// reports major 10.
pub fn macos_out_of_support(major: u32) -> bool {
    major > 0 && major < MACOS_OLDEST_SUPPORTED
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(build: u32, edition: &str) -> Option<OsVersion> {
        Some(OsVersion { build, edition: edition.into(), ..Default::default() })
    }

    #[test]
    fn macos_support_window() {
        assert!(macos_out_of_support(10));
        assert!(macos_out_of_support(13));
        assert!(!macos_out_of_support(14));
        assert!(!macos_out_of_support(26));
        assert!(!macos_out_of_support(0));
        assert_eq!(macos_name(15), "macOS Sequoia");
    }

    #[test]
    fn macos_status_keeps_its_own_support_flag() {
        // build 0: the Windows calendar must not override what the macOS
        // layer decided.
        let mut s = UpdateStatus {
            version: Some(OsVersion { name: "macOS Ventura".into(), ..Default::default() }),
            out_of_support: true,
            ..Default::default()
        };
        s.finish();
        assert!(s.out_of_support);
        assert_eq!(s.servicing_ends, None);
    }

    #[test]
    fn windows_10_is_out_of_support() {
        let mut s = UpdateStatus { version: version(19045, "Professional"), ..Default::default() };
        s.finish();
        assert!(s.out_of_support);
        assert_eq!(s.servicing_ends.as_deref(), Some("2025-10-14"));
        assert_eq!(s.health().unsupported_build, Some(19045));
    }

    #[test]
    fn enterprise_gets_longer_support_and_ltsc_is_not_judged() {
        assert!(servicing_end(22631, "Enterprise") > servicing_end(22631, "Professional"));
        assert_eq!(servicing_end(19044, "EnterpriseS"), None);
        assert_eq!(servicing_end(99999, "Professional"), None);
    }

    #[test]
    fn only_real_updates_count() {
        let u = |driver, optional, definition, severity: Option<&str>| PendingUpdate {
            driver,
            optional,
            definition,
            severity: severity.map(Into::into),
            ..Default::default()
        };
        let s = UpdateStatus {
            pending: vec![
                u(false, false, false, Some("Critical")),
                u(false, false, false, None),
                u(false, true, false, None),
                u(false, false, true, None),
                u(true, false, false, None),
            ],
            ..Default::default()
        };
        let h = s.health();
        assert_eq!(h.pending, 2);
        assert_eq!(h.pending_security, 1);
    }
}
