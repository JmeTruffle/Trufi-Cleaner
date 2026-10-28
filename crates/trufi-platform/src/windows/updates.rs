//! Windows version from the registry, pending updates and history from the
//! Windows Update Agent (COM, through PowerShell).

use super::powershell;
use serde::Deserialize;
use std::time::{Duration, SystemTime};
use trufi_core::updates::{PendingUpdate, UpdateStatus, WindowsVersion};
use trufi_core::Result;
use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;

/// The search contacts Microsoft's servers and can take a while on a slow
/// connection.
const TIMEOUT: Duration = Duration::from_secs(180);

/// Definition Updates classification (Defender signatures).
const DEFINITION_UPDATES: &str = "e0789628-ce08-4437-be74-2495b842f43b";

const SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$out = [ordered]@{ pending = @(); reboot_required = $false; last_install = $null; error = $null }
try { $out.reboot_required = [bool](New-Object -ComObject Microsoft.Update.SystemInfo).RebootRequired } catch {}
$session = New-Object -ComObject Microsoft.Update.Session
$session.ClientApplicationID = 'Trufi Cleaner'
try {
  $searcher = $session.CreateUpdateSearcher()
  $n = $searcher.GetTotalHistoryCount()
  if ($n -gt 0) {
    $last = $searcher.QueryHistory(0, [Math]::Min($n, 200)) |
      Where-Object { $_.Operation -eq 1 -and $_.ResultCode -eq 2 } |
      Sort-Object Date -Descending | Select-Object -First 1
    if ($last) { $out.last_install = $last.Date.ToString('yyyy-MM-dd') }
  }
} catch {}
try {
  $result = $session.CreateUpdateSearcher().Search("IsInstalled=0 and IsHidden=0")
  $out.pending = @(foreach ($u in $result.Updates) {
    $isDriver = $u.Type -eq 2
    [ordered]@{
      id = [string]$u.Identity.UpdateID
      title = [string]$u.Title
      driver = $isDriver
      optional = [bool]$u.BrowseOnly
      definition = [bool](@($u.Categories | Where-Object { $_.CategoryID -eq '__DEFINITIONS__' }).Count)
      severity = $(if ($u.MsrcSeverity) { [string]$u.MsrcSeverity } else { $null })
      kb = $(@($u.KBArticleIDs | ForEach-Object { "KB$_" }) -join ', ')
      size_bytes = [int64]$u.MaxDownloadSize
      driver_class = $(if ($isDriver) { [string]$u.DriverClass } else { $null })
      driver_provider = $(if ($isDriver) { [string]$u.DriverProvider } else { $null })
      driver_hardware_id = $(if ($isDriver) { [string]$u.DriverHardwareID } else { $null })
      driver_date = $(if ($isDriver -and $u.DriverVerDate) { ([datetime]$u.DriverVerDate).ToString('yyyy-MM-dd') } else { $null })
    }
  })
} catch { $out.error = $_.Exception.Message }
[pscustomobject]$out | ConvertTo-Json -Compress -Depth 4
"#;

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawPending {
    id: String,
    title: String,
    driver: bool,
    optional: bool,
    definition: bool,
    severity: Option<String>,
    kb: Option<String>,
    size_bytes: u64,
    driver_class: Option<String>,
    driver_provider: Option<String>,
    driver_hardware_id: Option<String>,
    driver_date: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Raw {
    pending: Vec<RawPending>,
    reboot_required: bool,
    last_install: Option<String>,
    error: Option<String>,
}

fn non_empty(s: Option<String>) -> Option<String> {
    s.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

pub fn version() -> Option<WindowsVersion> {
    let key = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")
        .ok()?;
    let build: u32 = key.get_value::<String, _>("CurrentBuild").ok()?.trim().parse().ok()?;
    let name = if build >= 22000 { "Windows 11" } else { "Windows 10" };
    Some(WindowsVersion {
        name: name.into(),
        display_version: non_empty(key.get_value("DisplayVersion").ok())
            .or_else(|| non_empty(key.get_value("ReleaseId").ok())),
        build,
        revision: key.get_value::<u32, _>("UBR").unwrap_or(0),
        edition: key.get_value("EditionID").unwrap_or_default(),
    })
}

pub fn status() -> Result<UpdateStatus> {
    let script = SCRIPT.replace("__DEFINITIONS__", DEFINITION_UPDATES);
    let raw: Raw = powershell::run_json(&script, TIMEOUT)?;
    let mut status = UpdateStatus {
        version: version(),
        pending: raw
            .pending
            .into_iter()
            .map(|u| PendingUpdate {
                id: u.id,
                title: u.title,
                driver: u.driver,
                optional: u.optional,
                definition: u.definition,
                severity: non_empty(u.severity),
                kb: non_empty(u.kb),
                size_bytes: u.size_bytes,
                driver_class: non_empty(u.driver_class),
                driver_provider: non_empty(u.driver_provider),
                driver_hardware_id: non_empty(u.driver_hardware_id),
                driver_date: non_empty(u.driver_date),
            })
            .collect(),
        reboot_required: raw.reboot_required,
        last_install: non_empty(raw.last_install),
        error: non_empty(raw.error),
        checked_at_ms: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        ..Default::default()
    };
    status.finish();
    Ok(status)
}
