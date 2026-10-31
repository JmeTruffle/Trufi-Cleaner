//! Running the system's command-line tools (`osascript`, `launchctl`,
//! `softwareupdate`, `open`) with a time limit.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use trufi_core::{Error, Result};

pub struct Output {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Runs a program and collects its output; kills it after `timeout`.
pub fn run(program: &str, args: &[&str], timeout: Duration) -> Result<Output> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Error::Platform(format!("{program}: {e}")))?;
    // Read both pipes while waiting so a chatty tool cannot block on a full pipe.
    let mut out = child.stdout.take();
    let mut err = child.stderr.take();
    let out_t = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(o) = out.as_mut() {
            let _ = o.read_to_string(&mut s);
        }
        s
    });
    let err_t = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(e) = err.as_mut() {
            let _ = e.read_to_string(&mut s);
        }
        s
    });
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::Platform(format!("{program} не ответил за {} с", timeout.as_secs())));
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    Ok(Output {
        success: status.success(),
        stdout: out_t.join().unwrap_or_default(),
        stderr: err_t.join().unwrap_or_default(),
    })
}

/// Runs a program and fails with its error output when it fails.
pub fn run_ok(program: &str, args: &[&str], timeout: Duration) -> Result<String> {
    let o = run(program, args, timeout)?;
    if o.success {
        Ok(o.stdout)
    } else {
        let msg = o.stderr.trim();
        Err(Error::Platform(if msg.is_empty() { format!("{program} завершился с ошибкой") } else { msg.to_string() }))
    }
}

/// Runs an AppleScript. `argv` reaches the script as `item n of argv`, so
/// paths and names are never pasted into the script text.
pub fn osascript(lines: &[&str], argv: &[&str], timeout: Duration) -> Result<String> {
    let mut args: Vec<&str> = vec!["-e", "on run argv"];
    for l in lines {
        args.push("-e");
        args.push(l);
    }
    args.extend(["-e", "end run"]);
    args.extend(argv);
    let o = run("/usr/bin/osascript", &args, timeout)?;
    if o.success {
        return Ok(o.stdout.trim().to_string());
    }
    let msg = o.stderr.trim();
    // -128: the user pressed Cancel (password prompt, Finder dialog).
    if msg.contains("-128") {
        return Err(Error::Platform("Отменено.".into()));
    }
    // -1743: the user has not allowed controlling Finder / System Events.
    if msg.contains("-1743") {
        return Err(Error::Platform(
            "macOS не разрешила Trufi Cleaner управлять Finder. Разрешите в Системных настройках: Конфиденциальность и безопасность → Автоматизация.".into(),
        ));
    }
    Err(Error::Platform(if msg.is_empty() { "Не удалось выполнить действие.".into() } else { msg.to_string() }))
}

/// Opens a URL or file with its default handler (`open`).
pub fn open(target: &str) -> Result<()> {
    run_ok("/usr/bin/open", &[target], Duration::from_secs(15)).map(|_| ())
}
