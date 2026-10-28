//! Installed device drivers and devices that report a problem. The
//! platform layer lists them; this module links them to driver updates
//! offered by Windows Update and sums up what matters for health.

use crate::updates::PendingUpdate;
use crate::util::{parse_date_days, today_days};
use serde::{Deserialize, Serialize};

/// Device Manager codes that are not faults: 22 is "disabled by the user",
/// 45 is "not connected right now".
const BENIGN_CODES: &[u32] = &[22, 45];

/// A third-party driver older than this is worth a look.
const OLD_DRIVER_DAYS: u32 = 3 * 365;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DriverInfo {
    /// Device instance id (PCI\VEN_10DE&DEV_2484\...).
    pub id: String,
    pub device: String,
    /// Device class, e.g. "DISPLAY", "NET", "MEDIA".
    pub class: Option<String>,
    pub provider: Option<String>,
    pub version: Option<String>,
    /// "YYYY-MM-DD".
    pub date: Option<String>,
    pub inf: Option<String>,
    /// None when Windows does not say (devices without a driver).
    pub signed: Option<bool>,
    /// Device Manager problem code; None when the device works.
    pub problem_code: Option<u32>,
    pub hardware_id: Option<String>,

    // Filled by `annotate`.
    /// Ships with Windows (provider is Microsoft).
    #[serde(default)]
    pub inbox: bool,
    #[serde(default)]
    pub age_days: Option<u32>,
    /// Third-party and older than three years.
    #[serde(default)]
    pub old: bool,
    /// A newer driver Windows Update offers for this device.
    #[serde(default)]
    pub update: Option<DriverUpdate>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DriverUpdate {
    pub title: String,
    pub date: Option<String>,
    pub optional: bool,
}

impl DriverInfo {
    /// A real fault, not a device the user turned off or unplugged.
    pub fn has_problem(&self) -> bool {
        self.problem_code.is_some_and(|c| c != 0 && !BENIGN_CODES.contains(&c))
    }

    pub fn is_unsigned(&self) -> bool {
        self.signed == Some(false) && self.inf.is_some()
    }
}

/// Fills the derived fields and links Windows Update driver offers to the
/// devices they are for. Problem devices come first, then updatable ones.
pub fn annotate(drivers: &mut [DriverInfo], pending: &[PendingUpdate]) {
    let today = today_days();
    for d in drivers.iter_mut() {
        d.inbox = d.provider.as_deref().is_some_and(|p| p.trim().eq_ignore_ascii_case("microsoft"));
        d.age_days = d.date.as_deref().and_then(parse_date_days).map(|x| (today - x).max(0) as u32);
        d.old = !d.inbox && d.age_days.is_some_and(|a| a >= OLD_DRIVER_DAYS);
        d.update = d.hardware_id.as_deref().and_then(|hw| {
            pending
                .iter()
                .filter(|u| u.driver)
                .find(|u| u.driver_hardware_id.as_deref().is_some_and(|x| x.eq_ignore_ascii_case(hw)))
                .map(|u| DriverUpdate { title: u.title.clone(), date: u.driver_date.clone(), optional: u.optional })
        });
    }
    drivers.sort_by(|a, b| {
        let rank = |d: &DriverInfo| (!d.has_problem(), d.update.is_none(), !d.is_unsigned(), d.inbox);
        rank(a).cmp(&rank(b)).then_with(|| a.device.to_lowercase().cmp(&b.device.to_lowercase()))
    });
}

/// What the health score needs to know about drivers.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct DriverHealth {
    pub problems: u32,
    pub unsigned: u32,
    pub updates: u32,
}

pub fn health(drivers: &[DriverInfo], pending: &[PendingUpdate]) -> DriverHealth {
    DriverHealth {
        problems: drivers.iter().filter(|d| d.has_problem()).count() as u32,
        unsigned: drivers.iter().filter(|d| d.is_unsigned()).count() as u32,
        // Every driver Windows Update offers counts, matched to a device or not.
        updates: pending.iter().filter(|u| u.driver).count() as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn driver(device: &str, provider: &str, date: &str) -> DriverInfo {
        DriverInfo {
            id: device.into(),
            device: device.into(),
            provider: Some(provider.into()),
            date: Some(date.into()),
            inf: Some("oem1.inf".into()),
            signed: Some(true),
            ..Default::default()
        }
    }

    #[test]
    fn disabled_and_unplugged_devices_are_fine() {
        let mut d = driver("x", "Intel", "2025-01-01");
        d.problem_code = Some(22);
        assert!(!d.has_problem());
        d.problem_code = Some(45);
        assert!(!d.has_problem());
        d.problem_code = Some(43);
        assert!(d.has_problem());
    }

    #[test]
    fn annotates_and_sorts() {
        let mut gpu = driver("GPU", "NVIDIA", "2025-06-01");
        gpu.hardware_id = Some("PCI\\VEN_10DE&DEV_2484".into());
        let mut broken = driver("Audio", "Realtek", "2024-01-01");
        broken.problem_code = Some(10);
        let mut list = vec![
            driver("Old printer", "HP", "2012-05-01"),
            driver("USB hub", "Microsoft", "2006-06-21"),
            gpu,
            broken,
        ];
        let offer = PendingUpdate {
            title: "NVIDIA - Display - 32.0.15.6094".into(),
            driver: true,
            driver_hardware_id: Some("pci\\ven_10de&dev_2484".into()),
            ..Default::default()
        };
        annotate(&mut list, std::slice::from_ref(&offer));
        let names: Vec<&str> = list.iter().map(|d| d.device.as_str()).collect();
        assert_eq!(names, ["Audio", "GPU", "Old printer", "USB hub"]);
        assert!(list[1].update.is_some());
        assert!(list[2].old, "old third-party driver");
        assert!(!list[3].old, "inbox drivers carry a fixed 2006 date");
        assert!(list[3].inbox);

        let h = health(&list, &[offer]);
        assert_eq!((h.problems, h.unsigned, h.updates), (1, 0, 1));
    }
}
