//! Turns measurements into a short, prioritised list of suggestions.
//! Like health factors, advice is ids + numbers; the UI words it.

use crate::health::Health;
use crate::monitor::SystemSnapshot;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

/// What the UI button next to the advice does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdviceAction {
    Scan,
    Clean,
    Startup,
    Processes,
    Restart,
    /// Opens Windows Update.
    Updates,
    /// Opens the drivers page.
    Drivers,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Advice {
    pub id: String,
    pub severity: Severity,
    pub action: Option<AdviceAction>,
    pub value: f64,
    /// Optional subject, e.g. the process name.
    pub subject: Option<String>,
}

const MAX_ADVICE: usize = 5;

pub fn advise(s: &SystemSnapshot, health: &Health, scanned: bool) -> Vec<Advice> {
    let mut out = Vec::new();
    for f in &health.factors {
        let (severity, action) = match f.id.as_str() {
            "system_disk_full" if f.penalty >= 15 => (Severity::Critical, Some(AdviceAction::Scan)),
            "system_disk_full" => (Severity::Warning, Some(AdviceAction::Scan)),
            "disk_full" => (Severity::Info, None),
            "memory_high" => (Severity::Warning, Some(AdviceAction::Processes)),
            "memory_pressure" if f.penalty >= 20 => (Severity::Critical, Some(AdviceAction::Processes)),
            "memory_pressure" => (Severity::Warning, Some(AdviceAction::Processes)),
            "cpu_high" => (Severity::Warning, Some(AdviceAction::Processes)),
            "junk" => (Severity::Warning, Some(AdviceAction::Clean)),
            "startup_many" => (Severity::Info, Some(AdviceAction::Startup)),
            "uptime_long" => (Severity::Info, Some(AdviceAction::Restart)),
            "os_unsupported" => (Severity::Critical, Some(AdviceAction::Updates)),
            "updates_pending" if f.penalty >= 12 => (Severity::Critical, Some(AdviceAction::Updates)),
            "updates_pending" | "updates_stale" => (Severity::Warning, Some(AdviceAction::Updates)),
            "updates_restart" => (Severity::Warning, Some(AdviceAction::Restart)),
            "driver_problems" if f.penalty >= 20 => (Severity::Critical, Some(AdviceAction::Drivers)),
            "driver_problems" => (Severity::Warning, Some(AdviceAction::Drivers)),
            "driver_unsigned" | "driver_updates" => (Severity::Info, Some(AdviceAction::Drivers)),
            _ => (Severity::Info, None),
        };
        let subject = match f.id.as_str() {
            "memory_high" | "memory_pressure" => s.top_by_memory.first().map(|p| p.name.clone()),
            "cpu_high" => s.top_by_cpu.first().map(|p| p.name.clone()),
            _ => None,
        };
        out.push(Advice { id: f.id.clone(), severity, action, value: f.value, subject });
    }

    // A single process eating a lot of memory deserves a mention even when
    // the machine as a whole is fine.
    if s.memory_total > 0 && !out.iter().any(|a| a.id == "memory_high" || a.id == "memory_pressure") {
        if let Some(p) = s.top_by_memory.first() {
            let share = p.memory_bytes as f64 / s.memory_total as f64 * 100.0;
            if share >= 30.0 {
                out.push(Advice {
                    id: "process_memory_hog".into(),
                    severity: Severity::Info,
                    action: Some(AdviceAction::Processes),
                    value: share,
                    subject: Some(p.name.clone()),
                });
            }
        }
    }

    if !scanned {
        out.push(Advice {
            id: "never_scanned".into(),
            severity: Severity::Info,
            action: Some(AdviceAction::Scan),
            value: 0.0,
            subject: None,
        });
    }

    out.sort_by_key(|x| std::cmp::Reverse(x.severity));
    out.truncate(MAX_ADVICE);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::{evaluate, tests::snapshot, HealthInput};

    #[test]
    fn critical_disk_first() {
        let s = snapshot(50, 97, 10);
        let h = evaluate(&s, &HealthInput::default());
        let a = advise(&s, &h, true);
        assert_eq!(a[0].id, "system_disk_full");
        assert_eq!(a[0].severity, Severity::Critical);
        assert!(a.iter().any(|x| x.id == "uptime_long"));
    }

    #[test]
    fn suggests_first_scan() {
        let s = snapshot(30, 30, 0);
        let h = evaluate(&s, &HealthInput::default());
        let a = advise(&s, &h, false);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].action, Some(AdviceAction::Scan));
    }
}
