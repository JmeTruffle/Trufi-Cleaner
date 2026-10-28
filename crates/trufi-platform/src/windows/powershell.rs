//! Runs a PowerShell script without a console window and parses its JSON
//! output. Used where the Windows API is COM or WMI, which PowerShell
//! already speaks well (Windows Update, Plug and Play drivers).

use serde::de::DeserializeOwned;
use std::io::Read;
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use trufi_core::{Error, Result};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// UTF-8 output whatever the console code page, and no progress bars.
const PRELUDE: &str = "$ProgressPreference = 'SilentlyContinue'\n\
                       [Console]::OutputEncoding = [System.Text.Encoding]::UTF8\n";

fn exe() -> String {
    std::env::var("SystemRoot")
        .map(|root| format!(r"{root}\System32\WindowsPowerShell\v1.0\powershell.exe"))
        .unwrap_or_else(|_| "powershell.exe".into())
}

/// `-EncodedCommand` takes base64 of UTF-16LE, which sidesteps all quoting.
fn encode(script: &str) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |n, (i, b)| n | ((*b as u32) << (16 - 8 * i)));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ABC[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

pub fn run_json<T: DeserializeOwned>(script: &str, timeout: Duration) -> Result<T> {
    let mut child = Command::new(exe())
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand"])
        .arg(encode(&format!("{PRELUDE}{script}")))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()?;

    // Read on a thread so a large output cannot fill the pipe and stall
    // the script while we wait for it to exit.
    let mut stdout = child.stdout.take().expect("stdout is piped");
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });

    let started = Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            break;
        }
        if started.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::Platform("PowerShell did not answer in time".into()));
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    let buf = reader.join().unwrap_or_default();
    let text = String::from_utf8_lossy(&buf);
    let text = text.trim_start_matches('\u{feff}').trim();
    serde_json::from_str(text).map_err(|e| {
        log::warn!("unexpected PowerShell output: {e}: {}", text.chars().take(300).collect::<String>());
        Error::Platform(format!("unexpected PowerShell output: {e}"))
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn encodes_like_powershell_expects() {
        // [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes("dir"))
        assert_eq!(super::encode("dir"), "ZABpAHIA");
        assert_eq!(super::encode("ab"), "YQBiAA==");
    }
}
