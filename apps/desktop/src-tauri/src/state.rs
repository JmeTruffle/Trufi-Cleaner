use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock};
use trufi_core::drivers::DriverInfo;
use trufi_core::monitor::{Monitor, SystemSnapshot};
use trufi_core::platform::Platform;
use trufi_core::rules::RuleSet;
use trufi_core::scanner::ScanResult;
use trufi_core::updates::UpdateStatus;

/// About one minute of samples at the UI's polling rate.
pub const CPU_HISTORY: usize = 40;

pub struct AppState {
    pub platform: Arc<dyn Platform>,
    pub rules: RwLock<RuleSet>,
    pub rule_errors: Vec<String>,
    pub user_rules_dir: Option<PathBuf>,
    pub monitor: Mutex<Monitor>,
    pub cpu_history: Mutex<VecDeque<f32>>,
    pub last_snapshot: Mutex<Option<SystemSnapshot>>,
    pub last_scan: Mutex<Option<Arc<ScanResult>>>,
    pub startup_enabled: Mutex<Option<usize>>,
    /// Last OS update check; the check is slow, so the UI asks for a
    /// fresh one only on start and on demand.
    pub updates: Mutex<Option<UpdateStatus>>,
    pub drivers: Mutex<Option<Vec<DriverInfo>>>,
    pub busy: AtomicBool,
}

impl AppState {
    pub fn new(user_rules_dir: Option<PathBuf>) -> Self {
        let platform: Arc<dyn Platform> = Arc::from(trufi_platform::current());
        let os = platform.os().as_str();
        let mut rules = RuleSet::builtin(os).unwrap_or_else(|e| {
            log::error!("built-in rules are broken: {e}");
            RuleSet::default()
        });
        let mut rule_errors = Vec::new();
        if let Some(dir) = &user_rules_dir {
            let _ = std::fs::create_dir_all(dir);
            rule_errors = rules.load_dir(dir, os).into_iter().map(|e| e.to_string()).collect();
        }
        let system_mount = platform
            .path_vars()
            .get("SYSTEMDRIVE")
            .cloned()
            .unwrap_or_else(|| PathBuf::from("/"));

        AppState {
            platform,
            rules: RwLock::new(rules),
            rule_errors,
            user_rules_dir,
            monitor: Mutex::new(Monitor::new(system_mount)),
            cpu_history: Mutex::new(VecDeque::with_capacity(CPU_HISTORY)),
            last_snapshot: Mutex::new(None),
            last_scan: Mutex::new(None),
            startup_enabled: Mutex::new(None),
            updates: Mutex::new(None),
            drivers: Mutex::new(None),
            busy: AtomicBool::new(false),
        }
    }
}
