//! Concrete [`Platform`] implementations. The app asks for [`current`] and
//! never names an OS itself.
//!
//! * `windows`: the primary target, fully implemented.
//! * `unix`: macOS and Linux. Enough for development and for the macOS
//!   port to start from; startup items and app inventory are basic.

use trufi_core::platform::Platform;

#[cfg(not(windows))]
mod unix;
#[cfg(windows)]
mod windows;

pub fn current() -> Box<dyn Platform> {
    #[cfg(windows)]
    return Box::new(windows::WindowsPlatform::new());
    #[cfg(not(windows))]
    return Box::new(unix::UnixPlatform::new());
}

/// Splits a command line into the program path and its arguments:
/// `"C:\a b\x.exe" -q` and `C:\a b\x.exe -q` both give `C:\a b\x.exe`.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn split_command(cmd: &str) -> (String, String) {
    let cmd = cmd.trim();
    if let Some(rest) = cmd.strip_prefix('"') {
        if let Some(end) = rest.find('"') {
            return (rest[..end].to_string(), rest[end + 1..].trim().to_string());
        }
    }
    let lower = cmd.to_ascii_lowercase();
    if let Some(i) = lower.find(".exe") {
        let end = i + 4;
        if end == cmd.len() || cmd.as_bytes()[end] == b' ' {
            return (cmd[..end].to_string(), cmd[end..].trim().to_string());
        }
    }
    match cmd.split_once(' ') {
        Some((a, b)) => (a.to_string(), b.trim().to_string()),
        None => (cmd.to_string(), String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::split_command;

    #[test]
    fn splits_commands() {
        assert_eq!(
            split_command(r#""C:\Program Files\A\a.exe" --min"#),
            (r"C:\Program Files\A\a.exe".into(), "--min".into())
        );
        assert_eq!(
            split_command(r"C:\Program Files\A\a.exe /S"),
            (r"C:\Program Files\A\a.exe".into(), "/S".into())
        );
        assert_eq!(
            split_command("MsiExec.exe /X{GUID}"),
            ("MsiExec.exe".into(), "/X{GUID}".into())
        );
        assert_eq!(split_command("ctfmon"), ("ctfmon".into(), String::new()));
    }
}
