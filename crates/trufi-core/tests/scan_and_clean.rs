//! End-to-end: a fake platform, a user rule pack, scan, then clean.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use trufi_core::cleaner::{clean, CleanRequest};
use trufi_core::model::{InstalledApp, StartupItem};
use trufi_core::platform::{LeftoverRoots, OsFamily, Platform};
use trufi_core::rules::RuleSet;
use trufi_core::scanner::{ScanOptions, Scanner};

struct FakePlatform {
    home: PathBuf,
}

impl Platform for FakePlatform {
    fn os(&self) -> OsFamily {
        OsFamily::Linux
    }
    fn os_label(&self) -> String {
        "test".into()
    }
    fn path_vars(&self) -> HashMap<String, PathBuf> {
        HashMap::from([("HOME".into(), self.home.clone())])
    }
    fn protected_paths(&self) -> Vec<PathBuf> {
        vec![self.home.join("protected")]
    }
    fn is_elevated(&self) -> bool {
        false
    }
    fn installed_apps(&self) -> trufi_core::Result<Vec<InstalledApp>> {
        Ok(vec![])
    }
    fn leftover_roots(&self) -> LeftoverRoots {
        LeftoverRoots::default()
    }
    fn startup_items(&self) -> trufi_core::Result<Vec<StartupItem>> {
        Ok(vec![])
    }
    fn set_startup_enabled(&self, _: &str, _: bool) -> trufi_core::Result<()> {
        Ok(())
    }
    fn trash_info(&self) -> trufi_core::Result<Option<(u64, u64)>> {
        Ok(None)
    }
    fn empty_trash(&self) -> trufi_core::Result<()> {
        Ok(())
    }
}

fn write(path: &Path, bytes: usize, age_hours: u64) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, vec![1u8; bytes]).unwrap();
    let t = SystemTime::now() - Duration::from_secs(age_hours * 3600);
    std::fs::File::options().write(true).open(path).unwrap().set_modified(t).unwrap();
}

const PACK: &str = r#"
[pack]
id = "test"
name = "test"
os = "linux"

[[rule]]
id = "cache"
name = "App cache"
category = "apps"
risk = "safe"
paths = ["~/app/*/Cache"]
min_age_hours = 24

[[rule]]
id = "logs"
name = "Logs"
category = "logs"
risk = "safe"
paths = ["~/app"]
include = ["*.log"]

[[rule]]
id = "evil"
name = "Points at home"
category = "system"
risk = "safe"
paths = ["~"]

[[rule]]
id = "protected"
name = "Protected"
category = "system"
risk = "safe"
paths = ["~/protected/inner"]
"#;

#[test]
fn scan_then_clean() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().to_path_buf();
    let a = home.join("app/Default/Cache");
    let b = home.join("app/Profile 1/Cache");
    write(&a.join("old.bin"), 1000, 48);
    write(&a.join("deep/older.bin"), 500, 100);
    write(&a.join("fresh.bin"), 700, 1);
    write(&b.join("old.bin"), 300, 48);
    write(&home.join("app/run.log"), 50, 0);
    write(&home.join("app/Default/Cache/x.log"), 10, 48);
    write(&home.join("protected/inner/keep.bin"), 10, 48);
    write(&home.join("document.txt"), 10, 48);

    #[cfg(unix)]
    {
        // A link inside a cache must not lead the cleaner outside of it.
        std::os::unix::fs::symlink(home.join("document.txt"), a.join("link.txt")).unwrap();
    }

    let mut rules = RuleSet::default();
    rules.add_pack_str("test", PACK, "linux").unwrap();
    let platform = FakePlatform { home: home.clone() };
    let scanner = Scanner::new(&platform);
    let result = scanner.scan(&rules.rules, ScanOptions::default(), &|_| {});
    let summary = result.summary();

    let ids: Vec<&str> = summary.findings.iter().map(|f| f.rule_id.as_str()).collect();
    assert!(!ids.contains(&"evil"), "home itself must never be a root");
    assert!(!ids.contains(&"protected"));

    let cache = summary.findings.iter().find(|f| f.rule_id == "cache").unwrap();
    // x.log is deeper than ~/app, so the cache rule owns it.
    assert_eq!(cache.bytes, 1000 + 500 + 300 + 10);
    assert_eq!(cache.files, 4);
    let logs = summary.findings.iter().find(|f| f.rule_id == "logs").unwrap();
    assert_eq!(logs.bytes, 50);

    let report = clean(
        &result,
        &[CleanRequest { rule_id: "cache".into(), paths: None }],
        &platform,
        scanner.guard(),
    );
    assert_eq!(report.freed_bytes, 1810);
    assert_eq!(report.failed_files, 0);

    assert!(a.join("fresh.bin").exists(), "young files survive");
    assert!(!a.join("old.bin").exists());
    assert!(!a.join("deep").exists(), "emptied dirs are removed");
    assert!(a.exists(), "the root itself stays");
    assert!(!b.join("old.bin").exists());
    assert!(home.join("app/run.log").exists(), "logs rule was not requested");
    assert!(home.join("document.txt").exists());
    assert!(home.join("protected/inner/keep.bin").exists());
}

#[test]
fn clean_only_selected_paths() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().to_path_buf();
    write(&home.join("app/Default/Cache/a"), 10, 48);
    write(&home.join("app/Profile 1/Cache/b"), 10, 48);

    let mut rules = RuleSet::default();
    rules.add_pack_str("test", PACK, "linux").unwrap();
    let platform = FakePlatform { home: home.clone() };
    let scanner = Scanner::new(&platform);
    let result = scanner.scan(&rules.rules, ScanOptions::default(), &|_| {});
    let only = home.join("app/Default/Cache");
    let report = clean(
        &result,
        &[CleanRequest { rule_id: "cache".into(), paths: Some(vec![only]) }],
        &platform,
        scanner.guard(),
    );
    assert_eq!(report.removed_files, 1);
    assert!(home.join("app/Profile 1/Cache/b").exists());
}

#[test]
fn keep_newest_versions() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().to_path_buf();
    for (i, v) in ["v1", "v2", "v3"].iter().enumerate() {
        let d = home.join("Versions").join(v);
        write(&d.join("app.bin"), 100, 0);
        let t = SystemTime::now() - Duration::from_secs((10 - i as u64) * 3600);
        std::fs::File::open(&d).unwrap().set_modified(t).unwrap();
    }
    let pack = r#"
        [pack]
        id = "v"
        name = "v"
        os = "linux"
        [[rule]]
        id = "old"
        name = "Old versions"
        category = "games"
        risk = "safe"
        paths = ["~/Versions"]
        keep_newest = 1
    "#;
    let mut rules = RuleSet::default();
    rules.add_pack_str("v", pack, "linux").unwrap();
    let platform = FakePlatform { home: home.clone() };
    let scanner = Scanner::new(&platform);
    let result = scanner.scan(&rules.rules, ScanOptions::default(), &|_| {});
    assert_eq!(result.summary().total_bytes, 200);
    clean(&result, &[CleanRequest { rule_id: "old".into(), paths: None }], &platform, scanner.guard());
    assert!(!home.join("Versions/v1").exists());
    assert!(!home.join("Versions/v2").exists());
    assert!(home.join("Versions/v3/app.bin").exists(), "newest version stays");
}
