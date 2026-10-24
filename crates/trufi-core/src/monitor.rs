//! Live system load, built on `sysinfo` (which already abstracts Windows,
//! macOS and Linux). Keep one [`Monitor`] alive and call
//! [`Monitor::snapshot`] periodically: CPU usage is a delta between calls.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use sysinfo::{
    CpuRefreshKind, DiskKind, Disks, MemoryRefreshKind, Networks, ProcessRefreshKind,
    ProcessesToUpdate, RefreshKind, System,
};

const TOP_PROCESSES: usize = 6;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub name: String,
    pub mount: PathBuf,
    pub file_system: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    /// "ssd", "hdd" or "unknown".
    pub kind: String,
    pub removable: bool,
    pub is_system: bool,
}

/// Processes grouped by executable name (a browser is one line, not forty).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessGroup {
    pub name: String,
    pub instances: usize,
    /// Share of total CPU capacity, 0-100.
    pub cpu_percent: f32,
    pub memory_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    pub timestamp_ms: u64,
    pub cpu_percent: f32,
    pub cpu_per_core: Vec<f32>,
    pub cpu_brand: String,
    pub cpu_cores: usize,
    pub cpu_frequency_mhz: u64,
    pub memory_total: u64,
    pub memory_used: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub disks: Vec<DiskInfo>,
    pub net_rx_bps: u64,
    pub net_tx_bps: u64,
    pub uptime_secs: u64,
    pub process_count: usize,
    pub top_by_cpu: Vec<ProcessGroup>,
    pub top_by_memory: Vec<ProcessGroup>,
    pub host_name: String,
}

pub struct Monitor {
    sys: System,
    disks: Disks,
    networks: Networks,
    last_net: Instant,
    system_mount: PathBuf,
}

fn process_kind() -> ProcessRefreshKind {
    ProcessRefreshKind::nothing().with_cpu().with_memory()
}

impl Monitor {
    pub fn new(system_mount: PathBuf) -> Self {
        let mut sys = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        sys.refresh_processes_specifics(ProcessesToUpdate::All, true, process_kind());
        Monitor {
            sys,
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            last_net: Instant::now(),
            system_mount,
        }
    }

    pub fn snapshot(&mut self) -> SystemSnapshot {
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
        self.sys
            .refresh_processes_specifics(ProcessesToUpdate::All, true, process_kind());
        self.disks.refresh(true);
        self.networks.refresh(true);

        let elapsed = self.last_net.elapsed().as_secs_f64().max(0.001);
        self.last_net = Instant::now();
        let (rx, tx) = self
            .networks
            .list()
            .values()
            .fold((0u64, 0u64), |(r, t), n| (r + n.received(), t + n.transmitted()));

        let cores = self.sys.cpus().len().max(1);
        let mut groups: HashMap<String, ProcessGroup> = HashMap::new();
        for p in self.sys.processes().values() {
            if p.thread_kind().is_some() {
                continue; // Linux threads
            }
            let name = p.name().to_string_lossy().into_owned();
            let g = groups.entry(name.to_lowercase()).or_insert_with(|| ProcessGroup {
                name,
                instances: 0,
                cpu_percent: 0.0,
                memory_bytes: 0,
            });
            g.instances += 1;
            g.cpu_percent += p.cpu_usage() / cores as f32;
            g.memory_bytes += p.memory();
        }
        let process_count = groups.values().map(|g| g.instances).sum();
        let mut groups: Vec<ProcessGroup> = groups
            .into_values()
            .filter(|g| !is_idle_pseudo_process(&g.name))
            .collect();
        groups.sort_by(|a, b| b.cpu_percent.total_cmp(&a.cpu_percent));
        let top_by_cpu = groups.iter().take(TOP_PROCESSES).cloned().collect();
        groups.sort_by_key(|x| std::cmp::Reverse(x.memory_bytes));
        let top_by_memory = groups.iter().take(TOP_PROCESSES).cloned().collect();

        let mut disks: Vec<DiskInfo> = self
            .disks
            .list()
            .iter()
            .filter(|d| d.total_space() > 0)
            .map(|d| DiskInfo {
                name: d.name().to_string_lossy().into_owned(),
                mount: d.mount_point().to_path_buf(),
                file_system: d.file_system().to_string_lossy().into_owned(),
                total_bytes: d.total_space(),
                available_bytes: d.available_space(),
                kind: match d.kind() {
                    DiskKind::SSD => "ssd",
                    DiskKind::HDD => "hdd",
                    _ => "unknown",
                }
                .into(),
                removable: d.is_removable(),
                is_system: d.mount_point() == self.system_mount,
            })
            .collect();
        disks.sort_by(|a, b| b.is_system.cmp(&a.is_system).then(a.mount.cmp(&b.mount)));
        disks.dedup_by(|a, b| a.mount == b.mount);

        let cpus = self.sys.cpus();
        SystemSnapshot {
            timestamp_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            cpu_percent: self.sys.global_cpu_usage(),
            cpu_per_core: cpus.iter().map(|c| c.cpu_usage()).collect(),
            cpu_brand: cpus.first().map(|c| c.brand().trim().to_string()).unwrap_or_default(),
            cpu_cores: cpus.len(),
            cpu_frequency_mhz: cpus.first().map(|c| c.frequency()).unwrap_or(0),
            memory_total: self.sys.total_memory(),
            memory_used: self.sys.used_memory(),
            swap_total: self.sys.total_swap(),
            swap_used: self.sys.used_swap(),
            disks,
            net_rx_bps: (rx as f64 / elapsed) as u64,
            net_tx_bps: (tx as f64 / elapsed) as u64,
            uptime_secs: System::uptime(),
            process_count,
            top_by_cpu,
            top_by_memory,
            host_name: System::host_name().unwrap_or_default(),
        }
    }
}

fn is_idle_pseudo_process(name: &str) -> bool {
    matches!(name.to_lowercase().as_str(), "system idle process" | "idle" | "memory compression")
}

/// Lower-cased names of running processes, for "close the app first" hints.
pub fn running_process_names() -> HashSet<String> {
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    sys.processes()
        .values()
        .map(|p| p.name().to_string_lossy().to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_is_sane() {
        let mut m = Monitor::new("/".into());
        let s = m.snapshot();
        assert!(s.memory_total > 0);
        assert!(s.cpu_cores > 0);
        assert!(s.cpu_percent >= 0.0);
    }
}
