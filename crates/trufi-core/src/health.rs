//! A single 0-100 "health" score for the dashboard, explained by factors.
//! Factors carry ids and numbers, not text: wording lives in the UI's
//! translations.

use crate::drivers::DriverHealth;
use crate::monitor::{PressureLevel, SystemSnapshot};
use crate::updates::UpdateHealth;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct HealthInput {
    /// CPU usage averaged over the last minute or so, 0-100.
    pub cpu_avg_percent: f32,
    /// Bytes the last scan found, if a scan has run.
    pub junk_bytes: Option<u64>,
    pub startup_enabled: Option<usize>,
    /// Windows Update state, once it has been checked.
    pub updates: Option<UpdateHealth>,
    /// Driver state, once drivers have been listed.
    pub drivers: Option<DriverHealth>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthFactor {
    pub id: String,
    pub penalty: u8,
    /// The measured value the factor is about (percent, bytes, count, days).
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Health {
    pub score: u8,
    /// "good", "fair" or "poor".
    pub level: String,
    pub factors: Vec<HealthFactor>,
}

const GB: u64 = 1024 * 1024 * 1024;

fn tier(value: f64, tiers: &[(f64, u8)]) -> u8 {
    tiers.iter().find(|(limit, _)| value >= *limit).map(|t| t.1).unwrap_or(0)
}

pub fn evaluate(s: &SystemSnapshot, input: &HealthInput) -> Health {
    let mut factors = Vec::new();
    let mut push = |id: &str, penalty: u8, value: f64| {
        if penalty > 0 {
            factors.push(HealthFactor { id: id.into(), penalty, value });
        }
    };

    let cpu = input.cpu_avg_percent as f64;
    push("cpu_high", tier(cpu, &[(90.0, 15), (75.0, 8)]), cpu);

    // macOS keeps memory full of caches on purpose; there the OS's own
    // pressure level says whether memory is short.
    if let Some(p) = s.memory_pressure {
        let penalty = match p.level {
            PressureLevel::Normal => 0,
            PressureLevel::Warning => 12,
            PressureLevel::Critical => 20,
        };
        push("memory_pressure", penalty, p.percent as f64);
    } else if s.memory_total > 0 {
        let mem = s.memory_used as f64 / s.memory_total as f64 * 100.0;
        push("memory_high", tier(mem, &[(92.0, 20), (85.0, 12), (75.0, 4)]), mem);
    }

    for d in &s.disks {
        if d.total_bytes == 0 || d.removable {
            continue;
        }
        let used = 100.0 - d.available_bytes as f64 / d.total_bytes as f64 * 100.0;
        if d.is_system {
            push("system_disk_full", tier(used, &[(95.0, 25), (90.0, 15), (80.0, 5)]), used);
        } else {
            push("disk_full", tier(used, &[(95.0, 5)]), used);
        }
    }

    if let Some(junk) = input.junk_bytes {
        let gb = junk as f64 / GB as f64;
        push("junk", tier(gb, &[(10.0, 15), (3.0, 8), (1.0, 4)]), junk as f64);
    }
    if let Some(n) = input.startup_enabled {
        push("startup_many", tier(n as f64, &[(15.0, 10), (8.0, 5)]), n as f64);
    }
    let days = s.uptime_secs as f64 / 86400.0;
    push("uptime_long", tier(days, &[(14.0, 8), (7.0, 4)]), days);

    if let Some(u) = input.updates {
        if let Some(build) = u.unsupported_build {
            push("os_unsupported", 15, build as f64);
        }
        let pending = u.pending as f64;
        let penalty = if u.pending_security > 0 { 12 } else { tier(pending, &[(1.0, 6)]) };
        push("updates_pending", penalty, pending);
        push("updates_restart", if u.reboot_required { 5 } else { 0 }, 0.0);
        if let Some(d) = u.days_since_install {
            push("updates_stale", tier(d as f64, &[(60.0, 8), (35.0, 4)]), d as f64);
        }
    }
    if let Some(d) = input.drivers {
        push("driver_problems", tier(d.problems as f64, &[(3.0, 20), (1.0, 12)]), d.problems as f64);
        push("driver_unsigned", tier(d.unsigned as f64, &[(1.0, 6)]), d.unsigned as f64);
        push("driver_updates", tier(d.updates as f64, &[(1.0, 4)]), d.updates as f64);
    }

    let penalty: u32 = factors.iter().map(|f| f.penalty as u32).sum();
    let score = 100u32.saturating_sub(penalty).max(5) as u8;
    factors.sort_by_key(|x| std::cmp::Reverse(x.penalty));
    Health {
        score,
        level: match score {
            80.. => "good",
            55..=79 => "fair",
            _ => "poor",
        }
        .into(),
        factors,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::monitor::DiskInfo;

    pub fn snapshot(mem_used_pct: u64, disk_used_pct: u64, uptime_days: u64) -> SystemSnapshot {
        SystemSnapshot {
            timestamp_ms: 0,
            cpu_percent: 10.0,
            cpu_per_core: vec![10.0],
            cpu_brand: "cpu".into(),
            cpu_cores: 1,
            cpu_frequency_mhz: 0,
            memory_total: 100,
            memory_used: mem_used_pct,
            swap_total: 0,
            swap_used: 0,
            disks: vec![DiskInfo {
                name: "C".into(),
                mount: "C:\\".into(),
                file_system: "NTFS".into(),
                total_bytes: 100,
                available_bytes: 100 - disk_used_pct,
                kind: "ssd".into(),
                removable: false,
                is_system: true,
            }],
            net_rx_bps: 0,
            net_tx_bps: 0,
            uptime_secs: uptime_days * 86400,
            process_count: 1,
            top_by_cpu: vec![],
            top_by_memory: vec![],
            host_name: "pc".into(),
            memory_pressure: None,
        }
    }

    #[test]
    fn healthy_machine_scores_100() {
        let h = evaluate(&snapshot(40, 50, 1), &HealthInput::default());
        assert_eq!(h.score, 100);
        assert_eq!(h.level, "good");
        assert!(h.factors.is_empty());
    }

    #[test]
    fn stressed_machine_is_poor() {
        let input = HealthInput {
            cpu_avg_percent: 95.0,
            junk_bytes: Some(12 * GB),
            startup_enabled: Some(20),
            ..Default::default()
        };
        let h = evaluate(&snapshot(95, 97, 20), &input);
        assert_eq!(h.level, "poor");
        assert_eq!(h.factors[0].id, "system_disk_full");
    }

    #[test]
    fn updates_and_drivers_lower_the_score() {
        let input = HealthInput {
            updates: Some(UpdateHealth { pending: 3, pending_security: 1, reboot_required: true, ..Default::default() }),
            drivers: Some(DriverHealth { problems: 1, unsigned: 0, updates: 2 }),
            ..Default::default()
        };
        let h = evaluate(&snapshot(40, 50, 1), &input);
        assert_eq!(h.score, 100 - 12 - 5 - 12 - 4);
        let ids: Vec<&str> = h.factors.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids, ["updates_pending", "driver_problems", "updates_restart", "driver_updates"]);
    }

    #[test]
    fn mac_memory_is_judged_by_pressure() {
        use crate::monitor::MemoryPressure;
        // 95% "used" but normal pressure: nothing to report on a Mac.
        let mut s = snapshot(95, 50, 1);
        s.memory_pressure = Some(MemoryPressure { level: PressureLevel::Normal, percent: 40.0 });
        assert_eq!(evaluate(&s, &HealthInput::default()).score, 100);
        s.memory_pressure = Some(MemoryPressure { level: PressureLevel::Critical, percent: 90.0 });
        let h = evaluate(&s, &HealthInput::default());
        assert_eq!(h.factors[0].id, "memory_pressure");
        assert_eq!(h.score, 80);
    }

    #[test]
    fn up_to_date_system_has_no_update_factors() {
        let input = HealthInput {
            updates: Some(UpdateHealth { days_since_install: Some(10), ..Default::default() }),
            drivers: Some(DriverHealth::default()),
            ..Default::default()
        };
        assert_eq!(evaluate(&snapshot(40, 50, 1), &input).score, 100);
    }
}
