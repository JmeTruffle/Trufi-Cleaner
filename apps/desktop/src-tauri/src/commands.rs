use crate::state::{AppState, CPU_HISTORY};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use trufi_core::advisor::{advise, Advice};
use trufi_core::cleaner::{clean as run_clean, CleanRequest};
use trufi_core::drivers::{self, DriverInfo};
use trufi_core::health::{evaluate, Health, HealthInput};
use trufi_core::model::{CleanReport, InstalledApp, ScanSummary, StartupItem, UninstallOutcome};
use trufi_core::monitor::SystemSnapshot;
use trufi_core::platform::SystemPage;
use trufi_core::rules::Rule;
use trufi_core::scanner::{ScanOptions, Scanner};
use trufi_core::updates::UpdateStatus;

/// Errors cross the IPC boundary as a code the UI translates plus details.
#[derive(Debug, Serialize)]
pub struct CommandError {
    pub code: &'static str,
    pub message: String,
}

impl From<trufi_core::Error> for CommandError {
    fn from(e: trufi_core::Error) -> Self {
        let code = match e {
            trufi_core::Error::ElevationRequired => "elevation_required",
            trufi_core::Error::Unsupported(_) => "unsupported",
            _ => "failed",
        };
        CommandError { code, message: e.to_string() }
    }
}

fn fail(code: &'static str, message: impl Into<String>) -> CommandError {
    CommandError { code, message: message.into() }
}

type CmdResult<T> = Result<T, CommandError>;

/// Runs blocking work off the main thread with access to the state.
async fn blocking<T, F>(app: &AppHandle, f: F) -> CmdResult<T>
where
    T: Send + 'static,
    F: FnOnce(&AppState) -> CmdResult<T> + Send + 'static,
{
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || f(&app.state::<AppState>()))
        .await
        .map_err(|e| fail("failed", e.to_string()))?
}

/// Prevents a scan and a clean from running at the same time.
struct BusyGuard<'a>(&'a AppState);

impl<'a> BusyGuard<'a> {
    fn acquire(state: &'a AppState) -> CmdResult<Self> {
        state
            .busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map(|_| BusyGuard(state))
            .map_err(|_| fail("busy", "another operation is running"))
    }
}

impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        self.0.busy.store(false, Ordering::SeqCst);
    }
}

#[derive(Serialize)]
pub struct SystemInfo {
    pub os: &'static str,
    pub os_label: String,
    pub host_name: String,
    pub app_version: String,
    pub is_elevated: bool,
    /// macOS Full Disk Access; null on other systems.
    pub full_disk_access: Option<bool>,
    pub rule_count: usize,
    pub rule_packs: Vec<String>,
    pub rule_errors: Vec<String>,
    pub user_rules_dir: Option<PathBuf>,
}

#[tauri::command]
pub fn system_info(app: AppHandle, state: State<'_, AppState>) -> SystemInfo {
    let rules = state.rules.read().unwrap();
    SystemInfo {
        os: state.platform.os().as_str(),
        os_label: state.platform.os_label(),
        host_name: sysinfo::System::host_name().unwrap_or_default(),
        // The release build stamps the version into tauri.conf.json.
        app_version: app.package_info().version.to_string(),
        is_elevated: state.platform.is_elevated(),
        full_disk_access: state.platform.full_disk_access(),
        rule_count: rules.rules.len(),
        rule_packs: rules.packs.iter().map(|p| p.name.clone()).collect(),
        rule_errors: state.rule_errors.clone(),
        user_rules_dir: state.user_rules_dir.clone(),
    }
}

#[tauri::command]
pub async fn snapshot(app: AppHandle) -> CmdResult<SystemSnapshot> {
    blocking(&app, |state| {
        let mut snap = state.monitor.lock().unwrap().snapshot();
        snap.memory_pressure = state.platform.memory_pressure();
        let mut hist = state.cpu_history.lock().unwrap();
        if hist.len() == CPU_HISTORY {
            hist.pop_front();
        }
        hist.push_back(snap.cpu_percent);
        *state.last_snapshot.lock().unwrap() = Some(snap.clone());
        Ok(snap)
    })
    .await
}

#[derive(Serialize)]
pub struct HealthReport {
    pub health: Health,
    pub advice: Vec<Advice>,
}

#[tauri::command]
pub async fn health(app: AppHandle) -> CmdResult<HealthReport> {
    blocking(&app, |state| {
        let snap = match state.last_snapshot.lock().unwrap().clone() {
            Some(s) => s,
            None => {
                let mut s = state.monitor.lock().unwrap().snapshot();
                s.memory_pressure = state.platform.memory_pressure();
                s
            }
        };
        let hist = state.cpu_history.lock().unwrap();
        let cpu_avg = if hist.is_empty() {
            snap.cpu_percent
        } else {
            hist.iter().sum::<f32>() / hist.len() as f32
        };
        let scan = state.last_scan.lock().unwrap().clone();
        let mut startup = *state.startup_enabled.lock().unwrap();
        if startup.is_none() {
            if let Ok(n) = state.platform.startup_enabled_count() {
                startup = Some(n);
                *state.startup_enabled.lock().unwrap() = startup;
            }
        }
        let updates = state.updates.lock().unwrap().clone();
        let pending = updates.as_ref().map(|u| u.pending.as_slice()).unwrap_or_default();
        let input = HealthInput {
            cpu_avg_percent: cpu_avg,
            junk_bytes: scan.as_ref().map(|s| s.summary().total_bytes),
            startup_enabled: startup,
            updates: updates.as_ref().map(UpdateStatus::health),
            drivers: state.drivers.lock().unwrap().as_deref().map(|d| drivers::health(d, pending)),
        };
        let health = evaluate(&snap, &input);
        let advice = advise(&snap, &health, scan.is_some());
        Ok(HealthReport { health, advice })
    })
    .await
}

/// OS update state (Windows Update, macOS Software Update). Cached after
/// the first check; `refresh` asks the update service again.
#[tauri::command]
pub async fn update_status(app: AppHandle, refresh: bool) -> CmdResult<UpdateStatus> {
    blocking(&app, move |state| {
        if !refresh {
            if let Some(s) = state.updates.lock().unwrap().clone() {
                return Ok(s);
            }
        }
        let status = state.platform.update_status()?;
        // Driver offers may have changed: re-link them to devices.
        if let Some(list) = state.drivers.lock().unwrap().as_mut() {
            drivers::annotate(list, &status.pending);
        }
        *state.updates.lock().unwrap() = Some(status.clone());
        Ok(status)
    })
    .await
}

#[tauri::command]
pub async fn drivers(app: AppHandle, refresh: bool) -> CmdResult<Vec<DriverInfo>> {
    blocking(&app, move |state| {
        if !refresh {
            if let Some(list) = state.drivers.lock().unwrap().clone() {
                return Ok(list);
            }
        }
        let mut list = state.platform.drivers()?;
        let updates = state.updates.lock().unwrap().clone();
        drivers::annotate(&mut list, updates.as_ref().map(|u| u.pending.as_slice()).unwrap_or_default());
        *state.drivers.lock().unwrap() = Some(list.clone());
        Ok(list)
    })
    .await
}

#[tauri::command]
pub fn open_system_page(state: State<'_, AppState>, page: SystemPage) -> CmdResult<()> {
    Ok(state.platform.open_system_page(page)?)
}

#[tauri::command]
pub fn rules(state: State<'_, AppState>) -> Vec<Rule> {
    state.rules.read().unwrap().rules.clone()
}

#[derive(serde::Deserialize, Default)]
pub struct ScanArgs {
    /// Only these rules; all when absent.
    pub rule_ids: Option<Vec<String>>,
    #[serde(default)]
    pub include_leftovers: bool,
    #[serde(default)]
    pub include_large_files: bool,
    /// "safe" for quick clean.
    pub max_risk: Option<trufi_core::model::Risk>,
}

#[tauri::command]
pub async fn scan(app: AppHandle, args: Option<ScanArgs>) -> CmdResult<ScanSummary> {
    let args = args.unwrap_or_default();
    let emitter = app.clone();
    blocking(&app, move |state| {
        let _busy = BusyGuard::acquire(state)?;
        let rules: Vec<Rule> = {
            let set = state.rules.read().unwrap();
            match &args.rule_ids {
                Some(ids) => set.rules.iter().filter(|r| ids.contains(&r.id)).cloned().collect(),
                None => set.rules.clone(),
            }
        };
        let scanner = Scanner::new(state.platform.as_ref());
        let options = ScanOptions {
            include_leftovers: args.include_leftovers,
            include_large_files: args.include_large_files,
            max_risk: args.max_risk,
        };
        let result = scanner.scan(&rules, options, &|event| {
            let _ = emitter.emit("scan-event", event);
        });
        let summary = result.summary();
        *state.last_scan.lock().unwrap() = Some(Arc::new(result));
        Ok(summary)
    })
    .await
}

#[tauri::command]
pub async fn clean(app: AppHandle, requests: Vec<CleanRequest>) -> CmdResult<CleanReport> {
    blocking(&app, move |state| {
        let _busy = BusyGuard::acquire(state)?;
        let scan = state
            .last_scan
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| fail("no_scan", "scan first"))?;
        let scanner = Scanner::new(state.platform.as_ref());
        let report = run_clean(&scan, &requests, state.platform.as_ref(), scanner.guard());
        // The scan no longer reflects the disk; the UI re-scans when needed.
        *state.last_scan.lock().unwrap() = None;
        Ok(report)
    })
    .await
}

#[tauri::command]
pub async fn startup_items(app: AppHandle) -> CmdResult<Vec<StartupItem>> {
    blocking(&app, |state| {
        let items = state.platform.startup_items()?;
        *state.startup_enabled.lock().unwrap() = Some(items.iter().filter(|i| i.enabled).count());
        Ok(items)
    })
    .await
}

#[tauri::command]
pub async fn set_startup_enabled(app: AppHandle, id: String, enabled: bool) -> CmdResult<()> {
    blocking(&app, move |state| {
        state.platform.set_startup_enabled(&id, enabled)?;
        *state.startup_enabled.lock().unwrap() = None;
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn installed_apps(app: AppHandle) -> CmdResult<Vec<InstalledApp>> {
    blocking(&app, |state| Ok(state.platform.installed_apps()?)).await
}

#[tauri::command]
pub async fn uninstall_app(app: AppHandle, id: String) -> CmdResult<UninstallOutcome> {
    blocking(&app, move |state| Ok(state.platform.uninstall_app(&id)?)).await
}

#[tauri::command]
pub fn relaunch_elevated(app: AppHandle, state: State<'_, AppState>) -> CmdResult<()> {
    state.platform.relaunch_elevated()?;
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn reveal_path(path: PathBuf) -> CmdResult<()> {
    tauri_plugin_opener::reveal_item_in_dir(&path).map_err(|e| fail("failed", e.to_string()))
}
