//! Installed Plug and Play drivers (Win32_PnPSignedDriver) and present
//! devices that Device Manager marks with a problem (Win32_PnPEntity).

use super::powershell;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;
use trufi_core::drivers::DriverInfo;
use trufi_core::Result;

const TIMEOUT: Duration = Duration::from_secs(90);

const SCRIPT: &str = r#"
$ErrorActionPreference = 'SilentlyContinue'
$drivers = @(Get-CimInstance Win32_PnPSignedDriver | Where-Object { $_.DeviceID -and $_.DeviceName } | ForEach-Object {
  [ordered]@{
    id = [string]$_.DeviceID
    device = [string]$_.DeviceName
    class = [string]$_.DeviceClass
    provider = [string]$_.DriverProviderName
    version = [string]$_.DriverVersion
    date = $(if ($_.DriverDate) { $_.DriverDate.ToString('yyyy-MM-dd') } else { $null })
    inf = [string]$_.InfName
    signed = $_.IsSigned
    hardware_id = [string]$_.HardWareID
  }
})
$problems = @(Get-CimInstance Win32_PnPEntity | Where-Object { $_.ConfigManagerErrorCode -and $_.Present -ne $false } | ForEach-Object {
  [ordered]@{
    id = [string]$_.DeviceID
    device = [string]$_.Name
    class = [string]$_.PNPClass
    code = [int]$_.ConfigManagerErrorCode
    hardware_id = [string]@($_.HardwareID)[0]
  }
})
[pscustomobject]@{ drivers = $drivers; problems = $problems } | ConvertTo-Json -Compress -Depth 4
"#;

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawDriver {
    id: String,
    device: String,
    class: Option<String>,
    provider: Option<String>,
    version: Option<String>,
    date: Option<String>,
    inf: Option<String>,
    signed: Option<bool>,
    hardware_id: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawProblem {
    id: String,
    device: String,
    class: Option<String>,
    code: u32,
    hardware_id: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Raw {
    drivers: Vec<RawDriver>,
    problems: Vec<RawProblem>,
}

fn non_empty(s: Option<String>) -> Option<String> {
    s.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

pub fn list() -> Result<Vec<DriverInfo>> {
    let raw: Raw = powershell::run_json(SCRIPT, TIMEOUT)?;
    let mut out: Vec<DriverInfo> = raw
        .drivers
        .into_iter()
        .map(|d| DriverInfo {
            id: d.id,
            device: d.device.trim().to_string(),
            class: non_empty(d.class),
            provider: non_empty(d.provider),
            version: non_empty(d.version),
            date: non_empty(d.date),
            inf: non_empty(d.inf),
            signed: d.signed,
            hardware_id: non_empty(d.hardware_id),
            ..Default::default()
        })
        .collect();

    // Attach problem codes; a device without a working driver may be
    // missing from the driver list entirely, so add it.
    let index: HashMap<String, usize> =
        out.iter().enumerate().map(|(i, d)| (d.id.to_ascii_uppercase(), i)).collect();
    for p in raw.problems {
        match index.get(&p.id.to_ascii_uppercase()) {
            Some(&i) => out[i].problem_code = Some(p.code),
            None => out.push(DriverInfo {
                device: non_empty(Some(p.device)).unwrap_or_else(|| p.id.clone()),
                id: p.id,
                class: non_empty(p.class),
                problem_code: Some(p.code),
                hardware_id: non_empty(p.hardware_id),
                ..Default::default()
            }),
        }
    }
    Ok(out)
}
