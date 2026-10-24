//! Deletes what a scan found. Every entry is re-validated right before it
//! is removed: the safety guard runs again, links are never followed, and
//! files that changed since the scan (became young again) are skipped.

use crate::model::{CleanReport, RuleCleanResult};
use crate::platform::Platform;
use crate::rules::Special;
use crate::safety::SafetyGuard;
use crate::scanner::{Entry, RuleScan, ScanResult};
use crate::util::age_of;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const MAX_ERRORS_PER_RULE: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanRequest {
    pub rule_id: String,
    /// Restrict cleaning to these entries (or anything below them).
    /// `None` cleans everything the rule found.
    #[serde(default)]
    pub paths: Option<Vec<PathBuf>>,
}

pub fn clean(
    scan: &ScanResult,
    requests: &[CleanRequest],
    platform: &dyn Platform,
    guard: &SafetyGuard,
) -> CleanReport {
    let started = Instant::now();
    let per_rule: Vec<RuleCleanResult> = requests
        .par_iter()
        .filter_map(|req| scan.get(&req.rule_id).map(|rs| clean_rule(rs, req, platform, guard)))
        .collect();

    CleanReport {
        freed_bytes: per_rule.iter().map(|r| r.freed_bytes).sum(),
        removed_files: per_rule.iter().map(|r| r.removed_files).sum(),
        failed_files: per_rule.iter().map(|r| r.failed_files).sum(),
        skipped_files: 0,
        per_rule,
        duration_ms: started.elapsed().as_millis() as u64,
    }
}

fn wanted(entry: &Entry, filter: Option<&[PathBuf]>) -> bool {
    filter.is_none_or(|paths| paths.iter().any(|p| entry.path.starts_with(p)))
}

fn clean_rule(
    rs: &RuleScan,
    req: &CleanRequest,
    platform: &dyn Platform,
    guard: &SafetyGuard,
) -> RuleCleanResult {
    let mut result = RuleCleanResult {
        rule_id: rs.finding.rule_id.clone(),
        ..Default::default()
    };

    if rs.special == Some(Special::Trash) {
        match platform.empty_trash() {
            Ok(()) => {
                result.freed_bytes = rs.finding.bytes;
                result.removed_files = rs.finding.files;
            }
            Err(e) => {
                result.failed_files = rs.finding.files;
                result.errors.push(e.to_string());
            }
        }
        return result;
    }

    let min_age = rs
        .rule
        .as_ref()
        .map(|r| Duration::from_secs(r.min_age_hours * 3600))
        .unwrap_or_default();
    let remove_empty = rs.rule.as_ref().is_some_and(|r| r.remove_empty_dirs);
    let mut touched_dirs = BTreeSet::new();

    for entry in rs.entries.iter().filter(|e| wanted(e, req.paths.as_deref())) {
        match remove_entry(entry, guard, min_age) {
            Ok(true) => {
                result.freed_bytes += entry.bytes;
                result.removed_files += 1;
                if remove_empty {
                    if let Some(parent) = entry.path.parent() {
                        touched_dirs.insert(parent.to_path_buf());
                    }
                }
            }
            Ok(false) => {}
            Err(e) => {
                result.failed_files += 1;
                if result.errors.len() < MAX_ERRORS_PER_RULE {
                    result.errors.push(format!("{}: {e}", entry.path.display()));
                }
            }
        }
    }

    // Deepest first so emptied parents can go too; never the root itself.
    for dir in touched_dirs.iter().rev() {
        let root = rs.entries.iter().find(|e| dir.starts_with(&e.root)).map(|e| &e.root);
        let Some(root) = root else { continue };
        let mut d: &Path = dir;
        while d != root.as_path() && d.starts_with(root) && guard.allows_entry(d, root) {
            if std::fs::remove_dir(d).is_err() {
                break;
            }
            match d.parent() {
                Some(p) => d = p,
                None => break,
            }
        }
    }
    result
}

/// `Ok(false)` means the entry was skipped on purpose (gone, changed, unsafe).
fn remove_entry(entry: &Entry, guard: &SafetyGuard, min_age: Duration) -> io::Result<bool> {
    if !guard.allows_root(&entry.root) || !guard.allows_entry(&entry.path, &entry.root) {
        return Ok(false);
    }
    let meta = match std::fs::symlink_metadata(&entry.path) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e),
    };
    if meta.file_type().is_symlink() {
        return Ok(false);
    }
    if entry.tree {
        if !meta.is_dir() {
            return Ok(false);
        }
        // std's remove_dir_all does not follow links or junctions.
        std::fs::remove_dir_all(&entry.path)?;
        return Ok(true);
    }
    if !meta.is_file() {
        return Ok(false);
    }
    if !min_age.is_zero() && meta.modified().map(|m| age_of(m) < min_age).unwrap_or(true) {
        return Ok(false);
    }
    match std::fs::remove_file(&entry.path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied && meta.permissions().readonly() => {
            let mut perms = meta.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            perms.set_readonly(false);
            std::fs::set_permissions(&entry.path, perms)?;
            std::fs::remove_file(&entry.path)?;
            Ok(true)
        }
        Err(e) => Err(e),
    }
}
