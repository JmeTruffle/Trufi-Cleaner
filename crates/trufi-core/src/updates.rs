//! Windows Update state: the installed version, whether it is still
//! serviced, and what Windows Update has waiting. The platform layer fills
//! [`UpdateStatus`]; this module only interprets it.

use crate::util::{days_from_civil, parse_date_days, today_days};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindowsVersion {
    /// "Windows 11" or "Windows 10", from the build number.
    pub name: String,
    /// Feature update, e.g. "24H2". Absent on very old builds.
    pub display_version: Option<String>,
    pub build: u32,
    /// Monthly patch level (the part after the dot in 26100.4351).
    pub revision: u32,
    /// "Professional", "Core", "Enterprise"...
    pub edition: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PendingUpdate {
    pub id: String,
    pub title: String,
    pub driver: bool,
    /// Offered only on request ("optional updates" in Settings).
    pub optional: bool,
    /// Microsoft Defender definitions: installed silently, not worth a mention.
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
    pub version: Option<WindowsVersion>,
    /// Last day of security updates for this version, "YYYY-MM-DD".
    pub servicing_ends: Option<String>,
    pub out_of_support: bool,
    pub pending: Vec<PendingUpdate>,
    /// Installed updates wait for a restart.
    pub reboot_required: bool,
    /// Date of the last successful install, "YYYY-MM-DD".
    pub last_install: Option<String>,
    pub days_since_install: Option<u32>,
    /// Why the Windows Update search failed (offline, service disabled).
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

fn format_days(days: i64) -> String {
    // Inverse of days_from_civil (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(build: u32, edition: &str) -> Option<WindowsVersion> {
        Some(WindowsVersion { build, edition: edition.into(), ..Default::default() })
    }

    #[test]
    fn formats_dates_back() {
        for (y, m, d) in [(1970, 1, 1), (2024, 2, 29), (2026, 10, 13), (2099, 12, 31)] {
            assert_eq!(format_days(days_from_civil(y, m, d)), format!("{y:04}-{m:02}-{d:02}"));
        }
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
