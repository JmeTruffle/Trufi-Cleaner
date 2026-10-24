//! Startup programs, enabled/disabled the same way Task Manager does it:
//! through the `StartupApproved` keys. Nothing is deleted, so every change
//! can be undone here or in Task Manager.

use super::expand_env;
use crate::split_command;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use trufi_core::model::{StartupItem, StartupScope};
use trufi_core::{Error, Result};
use winreg::enums::*;
use winreg::types::FromRegValue;
use winreg::{RegKey, RegValue};

const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN32: &str = r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run";
const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved";
const USER_FOLDER: &str = r"Microsoft\Windows\Start Menu\Programs\Startup";
const COMMON_FOLDER: &str = r"Microsoft\Windows\Start Menu\Programs\StartUp";

/// One place startup entries come from.
struct Source {
    id: &'static str,
    hklm: bool,
    /// Registry key with the entries, or `None` for a startup folder.
    run_key: Option<&'static str>,
    /// Subkey of StartupApproved holding the enabled flags.
    approved: &'static str,
}

const SOURCES: &[Source] = &[
    Source { id: "hkcu_run", hklm: false, run_key: Some(RUN), approved: "Run" },
    Source { id: "hklm_run", hklm: true, run_key: Some(RUN), approved: "Run" },
    Source { id: "hklm_run32", hklm: true, run_key: Some(RUN32), approved: "Run32" },
    Source { id: "user_folder", hklm: false, run_key: None, approved: "StartupFolder" },
    Source { id: "common_folder", hklm: true, run_key: None, approved: "StartupFolder" },
];

fn hive(hklm: bool) -> RegKey {
    RegKey::predef(if hklm { HKEY_LOCAL_MACHINE } else { HKEY_CURRENT_USER })
}

/// Missing value or an even first byte means enabled.
fn is_enabled(source: &Source, name: &str) -> bool {
    hive(source.hklm)
        .open_subkey(format!(r"{APPROVED}\{}", source.approved))
        .and_then(|k| k.get_raw_value(name))
        .map(|v| v.bytes.first().is_none_or(|b| b % 2 == 0))
        .unwrap_or(true)
}

fn folder(source: &Source, vars: &HashMap<String, PathBuf>) -> Option<PathBuf> {
    let (base, sub) = if source.hklm { ("PROGRAMDATA", COMMON_FOLDER) } else { ("APPDATA", USER_FOLDER) };
    vars.get(base).map(|b| b.join(sub))
}

fn target_exists(command: &str) -> bool {
    let (exe, _) = split_command(&expand_env(command));
    let p = Path::new(&exe);
    // Bare names ("ctfmon.exe") are resolved through PATH; assume they exist.
    !p.is_absolute() || p.exists()
}

fn display_name(name: &str) -> String {
    name.trim_end_matches(".lnk").trim_end_matches(".url").to_string()
}

pub fn list(vars: &HashMap<String, PathBuf>) -> Vec<StartupItem> {
    let mut items = Vec::new();
    for source in SOURCES {
        let scope = if source.hklm { StartupScope::AllUsers } else { StartupScope::CurrentUser };
        match source.run_key {
            Some(key) => {
                let Ok(k) = hive(source.hklm).open_subkey(key) else { continue };
                for (name, value) in k.enum_values().flatten() {
                    if name.is_empty() {
                        continue;
                    }
                    let Ok(command) = String::from_reg_value(&value) else { continue };
                    items.push(StartupItem {
                        id: format!("{}|{name}", source.id),
                        name: name.clone(),
                        target_exists: target_exists(&command),
                        command,
                        source: "registry".into(),
                        scope,
                        enabled: is_enabled(source, &name),
                        requires_admin: source.hklm,
                    });
                }
            }
            None => {
                let Some(dir) = folder(source, vars) else { continue };
                let Ok(entries) = std::fs::read_dir(&dir) else { continue };
                for e in entries.flatten() {
                    let name = e.file_name().to_string_lossy().into_owned();
                    if name.eq_ignore_ascii_case("desktop.ini") {
                        continue;
                    }
                    items.push(StartupItem {
                        id: format!("{}|{name}", source.id),
                        name: display_name(&name),
                        command: e.path().display().to_string(),
                        source: "folder".into(),
                        scope,
                        enabled: is_enabled(source, &name),
                        requires_admin: source.hklm,
                        target_exists: true,
                    });
                }
            }
        }
    }
    items.sort_by_key(|x| x.name.to_lowercase());
    items
}

fn filetime_now() -> [u8; 8] {
    // 100ns intervals since 1601-01-01.
    let since_unix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let ft = since_unix.as_nanos() as u64 / 100 + 116_444_736_000_000_000;
    ft.to_le_bytes()
}

pub fn set_enabled(id: &str, enabled: bool) -> Result<()> {
    let (source_id, name) = id
        .split_once('|')
        .ok_or_else(|| Error::Platform(format!("bad startup id {id}")))?;
    let source = SOURCES
        .iter()
        .find(|s| s.id == source_id)
        .ok_or_else(|| Error::Platform(format!("bad startup source {source_id}")))?;

    // Same 12-byte layout Task Manager writes: state byte, 3 zero bytes,
    // then a FILETIME of the change (zero when enabling).
    let mut bytes = vec![0u8; 12];
    if enabled {
        bytes[0] = 0x02;
    } else {
        bytes[0] = 0x03;
        bytes[4..].copy_from_slice(&filetime_now());
    }
    let result = hive(source.hklm)
        .create_subkey(format!(r"{APPROVED}\{}", source.approved))
        .and_then(|(k, _)| {
            k.set_raw_value(name, &RegValue { bytes: bytes.into(), vtype: REG_BINARY })
        });
    match result {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => Err(Error::ElevationRequired),
        Err(e) => Err(e.into()),
    }
}
