//! Thin wrappers over Win32 shell and security APIs.

use std::ffi::c_void;
use trufi_core::{Error, Result};
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::Win32::UI::Shell::{
    SHEmptyRecycleBinW, SHQueryRecycleBinW, ShellExecuteW, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI,
    SHERB_NOSOUND, SHQUERYRBINFO,
};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

fn platform_err(e: windows::core::Error) -> Error {
    Error::Platform(e.message())
}

pub fn is_elevated() -> bool {
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut len = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut c_void),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut len,
        )
        .is_ok();
        let _ = CloseHandle(token);
        ok && elevation.TokenIsElevated != 0
    }
}

pub fn shell_execute(verb: &str, file: &str, args: &str) -> Result<()> {
    let verb = HSTRING::from(verb);
    let file = HSTRING::from(file);
    let args = HSTRING::from(args);
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR(args.as_ptr()),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    // Values above 32 mean success; 5 is "access denied" (UAC declined).
    match result.0 as isize {
        n if n > 32 => Ok(()),
        5 => Err(Error::ElevationRequired),
        n => Err(Error::Platform(format!("ShellExecute failed ({n})"))),
    }
}

pub fn relaunch_elevated() -> Result<()> {
    let exe = std::env::current_exe()?;
    shell_execute("runas", &exe.to_string_lossy(), "")
}

pub fn recycle_bin_info() -> Result<(u64, u64)> {
    let mut info = SHQUERYRBINFO {
        cbSize: std::mem::size_of::<SHQUERYRBINFO>() as u32,
        ..Default::default()
    };
    unsafe { SHQueryRecycleBinW(PCWSTR::null(), &mut info) }.map_err(platform_err)?;
    Ok((info.i64Size.max(0) as u64, info.i64NumItems.max(0) as u64))
}

pub fn empty_recycle_bin() -> Result<()> {
    if recycle_bin_info().map(|(_, n)| n == 0).unwrap_or(false) {
        return Ok(());
    }
    unsafe {
        SHEmptyRecycleBinW(
            None,
            PCWSTR::null(),
            SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND,
        )
    }
    .map_err(platform_err)
}

/// Roots of all fixed (non-removable, non-network) drives: `C:\`, `D:\`...
pub fn fixed_drives() -> Vec<std::path::PathBuf> {
    const DRIVE_FIXED: u32 = 3;
    let mask = unsafe { GetLogicalDrives() };
    (0..26u8)
        .filter(|i| mask & (1 << i) != 0)
        .map(|i| format!("{}:\\", (b'A' + i) as char))
        .filter(|root| unsafe { GetDriveTypeW(&HSTRING::from(root.as_str())) } == DRIVE_FIXED)
        .map(std::path::PathBuf::from)
        .collect()
}
