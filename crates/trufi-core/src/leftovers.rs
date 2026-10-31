//! Detects data folders left behind by applications that are no longer
//! installed.
//!
//! This is heuristic by nature, so findings are always `Risk::Expert`,
//! never pre-selected, and conservative: a folder is reported only when no
//! installed app, publisher, install location or running process resembles
//! it, nothing inside was modified recently, and it is big enough to matter.

use crate::model::{Category, EntryPreview, InstalledApp, Risk, RuleFinding};
use crate::platform::LeftoverRoots;
use crate::safety::SafetyGuard;
use crate::scanner::{Entry, RuleScan};
use crate::util::{age_of, normalize_name};
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;
use walkdir::WalkDir;

pub const RULE_ID: &str = "leftovers.apps";
const MIN_BYTES: u64 = 1024 * 1024;
const QUIET_FOR: Duration = Duration::from_secs(30 * 24 * 3600);
const MAX_LISTED: usize = 200;

/// Names an installed app is known by, normalised.
fn known_tokens(apps: &[InstalledApp]) -> HashSet<String> {
    let mut tokens = HashSet::new();
    let mut add = |s: &str| {
        let n = normalize_name(s);
        if n.len() >= 3 {
            tokens.insert(n);
        }
    };
    for app in apps {
        add(&app.name);
        if let Some(first) = app.name.split_whitespace().next() {
            add(first);
        }
        if let Some(p) = &app.publisher {
            add(p);
            if let Some(first) = p.split_whitespace().next() {
                add(first);
            }
        }
        if let Some(id) = &app.bundle_id {
            add(id);
            // com.vendor.app: the vendor names shared folders.
            if let Some(vendor) = id.split('.').nth(1) {
                add(vendor);
            }
        }
        if let Some(loc) = &app.install_location {
            for c in loc.components().rev().take(2) {
                add(&c.as_os_str().to_string_lossy());
            }
        }
    }
    tokens
}

fn resembles(folder: &str, token: &str) -> bool {
    token.contains(folder) || (folder.len() >= 4 && token.len() >= 4 && folder.contains(token))
}

pub fn is_orphan(folder_name: &str, known: &HashSet<String>, running: &HashSet<String>) -> bool {
    let n = normalize_name(folder_name);
    if n.len() < 3 {
        return false;
    }
    if known.iter().any(|t| resembles(&n, t)) {
        return false;
    }
    !running.iter().any(|p| {
        let p = normalize_name(p.trim_end_matches(".exe"));
        p.len() >= 4 && resembles(&n, &p)
    })
}

/// Folder names games use for save files. Leftover folders containing one
/// are flagged so the user does not lose progress by accident.
const SAVE_DIR_NAMES: &[&str] = &["saved", "saves", "savegames", "savegame", "savedgames", "save", "profiles"];

/// Size of a tree and whether it looks like it holds game saves, or `None`
/// if it contains a link (we never delete through links) or was touched
/// recently.
fn measure_quiet_tree(dir: &Path) -> Option<(u64, bool)> {
    let mut bytes = 0;
    let mut saves = false;
    for entry in WalkDir::new(dir).follow_links(false).max_depth(12) {
        let entry = entry.ok()?;
        if entry.file_type().is_symlink() {
            return None;
        }
        let meta = entry.metadata().ok()?;
        if let Ok(m) = meta.modified() {
            if age_of(m) < QUIET_FOR {
                return None;
            }
        }
        if meta.is_file() {
            bytes += meta.len();
            let ext = entry.path().extension().map(|e| e.to_string_lossy().to_lowercase());
            if matches!(ext.as_deref(), Some("sav" | "save")) {
                saves = true;
            }
        } else if entry.depth() > 0 && entry.depth() <= 3 {
            let n = normalize_name(&entry.file_name().to_string_lossy());
            if SAVE_DIR_NAMES.contains(&n.as_str()) {
                saves = true;
            }
        }
    }
    Some((bytes, saves))
}

pub fn find(
    roots: &LeftoverRoots,
    apps: &[InstalledApp],
    running: &HashSet<String>,
    guard: &SafetyGuard,
) -> Option<RuleScan> {
    // Without an app inventory every folder would look orphaned.
    if apps.is_empty() {
        return None;
    }
    let known = known_tokens(apps);
    let ignore: HashSet<String> = roots.ignore_names.iter().map(|s| normalize_name(s)).collect();
    let ignore_prefix: Vec<String> = roots.ignore_prefixes.iter().map(|p| p.to_lowercase()).collect();

    let mut entries = Vec::new();
    let mut saves_at = HashSet::new();
    let mut requires_admin = false;
    for root in &roots.data_roots {
        let Ok(children) = std::fs::read_dir(root) else { continue };
        for child in children.flatten() {
            let Ok(ft) = child.file_type() else { continue };
            if !ft.is_dir() || ft.is_symlink() {
                continue;
            }
            let name = child.file_name().to_string_lossy().into_owned();
            let lower = name.to_lowercase();
            if ignore.contains(&normalize_name(&name))
                || ignore_prefix.iter().any(|p| lower.starts_with(p.as_str()))
                || !is_orphan(&name, &known, running)
            {
                continue;
            }
            let path = child.path();
            if !guard.allows_root(&path) {
                continue;
            }
            let Some((bytes, saves)) = measure_quiet_tree(&path) else { continue };
            if bytes < MIN_BYTES {
                continue;
            }
            if roots.admin_roots.iter().any(|a| path.starts_with(a)) {
                requires_admin = true;
            }
            if saves {
                saves_at.insert(path.clone());
            }
            entries.push(Entry { root: path.clone(), path, bytes, tree: true });
        }
    }
    if entries.is_empty() {
        return None;
    }
    entries.sort_by_key(|x| std::cmp::Reverse(x.bytes));
    entries.truncate(MAX_LISTED);
    let finding = RuleFinding {
        rule_id: RULE_ID.into(),
        name: "Остатки удалённых программ".into(),
        description: "Папки данных программ, которых больше нет в списке установленных. \
                      Проверьте каждую папку перед удалением: в папках игр бывают сохранения."
            .into(),
        category: Category::Leftovers,
        risk: Risk::Expert,
        requires_admin,
        selected_by_default: false,
        bytes: entries.iter().map(|e| e.bytes).sum(),
        files: entries.len() as u64,
        running_processes: Vec::new(),
        itemized: true,
        top_entries: entries
            .iter()
            .map(|e| EntryPreview {
                path: e.path.clone(),
                bytes: e.bytes,
                note: saves_at.contains(&e.path).then(|| "possible_saves".to_string()),
            })
            .collect(),
        roots: roots.data_roots.clone(),
    };
    Some(RuleScan { finding, entries, rule: None, special: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apps() -> Vec<InstalledApp> {
        vec![
            InstalledApp { name: "Google Chrome".into(), publisher: Some("Google LLC".into()), ..Default::default() },
            InstalledApp { name: "Telegram Desktop".into(), ..Default::default() },
            InstalledApp {
                name: "Slack".into(),
                bundle_id: Some("com.tinyspeck.slackmacgap".into()),
                ..Default::default()
            },
            InstalledApp {
                name: "Some Tool 2.1".into(),
                install_location: Some("/opt/Vendorly/ToolX".into()),
                ..Default::default()
            },
        ]
    }

    #[test]
    fn matching() {
        let known = known_tokens(&apps());
        let running = HashSet::from(["discord.exe".to_string()]);
        assert!(!is_orphan("Google", &known, &running));
        assert!(!is_orphan("Telegram Desktop", &known, &running));
        assert!(!is_orphan("Vendorly", &known, &running));
        assert!(!is_orphan("discord", &known, &running));
        assert!(is_orphan("OldGameStudio", &known, &running));
        assert!(!is_orphan("ab", &known, &running));
        // macOS containers are named after the bundle id.
        assert!(!is_orphan("com.tinyspeck.slackmacgap", &known, &running));
        assert!(is_orphan("com.oldvendor.gone", &known, &running));
    }

    #[test]
    fn skips_recent_and_small() {
        let dir = tempfile::tempdir().unwrap();
        let fresh = dir.path().join("FreshVendor");
        std::fs::create_dir_all(&fresh).unwrap();
        std::fs::write(fresh.join("big.bin"), vec![0u8; 2 * 1024 * 1024]).unwrap();
        let roots = LeftoverRoots { data_roots: vec![dir.path().into()], ..Default::default() };
        let guard = SafetyGuard::new(vec![], vec![]);
        // Fresh files: not a leftover.
        assert!(find(&roots, &apps(), &HashSet::new(), &guard).is_none());
    }
}
