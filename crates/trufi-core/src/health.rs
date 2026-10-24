//! A single 0-100 "health" score for the dashboard, explained by factors.
//! Factors carry ids and numbers, not text: wording lives in the UI's
//! translations.

use crate::monitor::SystemSnapshot;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct HealthInput {
    /// CPU usage averaged over the last minute or so, 0-100.
    pub cpu_avg_percent: f32,
    /// Bytes the last scan found, if a scan has run.
    pub junk_bytes: Option<u64>,
    pub startup_enabled: Option<usize>,
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

    if s.memory_total > 0 {
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
        let input = HealthInput { cpu_avg_percent: 95.0, junk_bytes: Some(12 * GB), startup_enabled: Some(20) };
        let h = evaluate(&snapshot(95, 97, 20), &input);
        assert_eq!(h.level, "poor");
        assert_eq!(h.factors[0].id, "system_disk_full");
    }
}
