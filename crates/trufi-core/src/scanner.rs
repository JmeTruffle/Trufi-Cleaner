//! Walks rule roots and measures what can be cleaned. Scanning never
//! modifies anything; the resulting [`ScanResult`] is what the cleaner acts
//! on, so only files that were shown to the user can be deleted.

use crate::leftovers;
use crate::model::{EntryPreview, Risk, RuleFinding, ScanEvent, ScanSummary};
use crate::platform::Platform;
use crate::rules::{expand_lists, resolve_roots, Rule, Special};
use crate::safety::SafetyGuard;
use crate::util::age_of;
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use walkdir::WalkDir;

const TOP_ENTRIES: usize = 6;
/// Expert rules with at most this many entries are reviewed item by item.
pub const ITEMIZED_MAX: usize = 200;

/// A deletable entry recorded during the scan.
#[derive(Debug, Clone)]
pub struct Entry {
    pub path: PathBuf,
    pub root: PathBuf,
    pub bytes: u64,
    /// Delete as a whole directory tree (used for leftovers).
    pub tree: bool,
}

#[derive(Debug, Clone)]
pub struct RuleScan {
    pub finding: RuleFinding,
    pub entries: Vec<Entry>,
    pub rule: Option<Rule>,
    pub special: Option<Special>,
}

#[derive(Debug, Clone, Default)]
pub struct ScanResult {
    pub rules: Vec<RuleScan>,
    pub duration: Duration,
}

impl ScanResult {
    pub fn summary(&self) -> ScanSummary {
        let findings: Vec<RuleFinding> = self.rules.iter().map(|r| r.finding.clone()).collect();
        ScanSummary {
            total_bytes: findings.iter().map(|f| f.bytes).sum(),
            total_files: findings.iter().map(|f| f.files).sum(),
            rules_checked: findings.len(),
            duration_ms: self.duration.as_millis() as u64,
            findings,
        }
    }

    pub fn get(&self, rule_id: &str) -> Option<&RuleScan> {
        self.rules.iter().find(|r| r.finding.rule_id == rule_id)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ScanOptions {
    pub include_leftovers: bool,
    /// Also list big files on every drive (`DRIVES`).
    pub include_large_files: bool,
    /// Skip rules whose risk is above this level.
    pub max_risk: Option<Risk>,
}

pub struct Scanner<'a> {
    platform: &'a dyn Platform,
    vars: HashMap<String, PathBuf>,
    lists: HashMap<String, Vec<PathBuf>>,
    guard: SafetyGuard,
    running: HashSet<String>,
}

pub(crate) fn guard_for(platform: &dyn Platform, vars: &HashMap<String, PathBuf>) -> SafetyGuard {
    SafetyGuard::new(platform.protected_paths(), vars.values().cloned().collect())
}

impl<'a> Scanner<'a> {
    pub fn new(platform: &'a dyn Platform) -> Self {
        let vars = platform.path_vars();
        let guard = guard_for(platform, &vars);
        Scanner {
            platform,
            vars,
            lists: platform.path_lists(),
            guard,
            running: crate::monitor::running_process_names(),
        }
    }

    pub fn guard(&self) -> &SafetyGuard {
        &self.guard
    }

    pub fn scan(
        &self,
        rules: &[Rule],
        options: ScanOptions,
        progress: &(dyn Fn(ScanEvent) + Sync),
    ) -> ScanResult {
        let started = Instant::now();
        let rules: Vec<&Rule> = rules
            .iter()
            .filter(|r| options.max_risk.is_none_or(|max| r.risk <= max))
            .collect();
        let total =
            rules.len() + usize::from(options.include_leftovers) + usize::from(options.include_large_files);
        progress(ScanEvent::Started { total_rules: total });
        let done = AtomicUsize::new(0);
        let report = |scan: &RuleScan| {
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            progress(ScanEvent::RuleDone {
                rule_id: scan.finding.rule_id.clone(),
                name: scan.finding.name.clone(),
                done: n,
                total,
                bytes: scan.finding.bytes,
            });
        };

        let mut scans: Vec<RuleScan> = rules
            .par_iter()
            .filter_map(|rule| {
                let scan = self.scan_rule(rule);
                if let Some(s) = &scan {
                    report(s);
                } else {
                    done.fetch_add(1, Ordering::Relaxed);
                }
                scan
            })
            .collect();

        if options.include_leftovers {
            if let Ok(apps) = self.platform.installed_apps() {
                let found = leftovers::find(
                    &self.platform.leftover_roots(),
                    &apps,
                    &self.running,
                    &self.guard,
                );
                if let Some(s) = found {
                    report(&s);
                    scans.push(s);
                }
            }
        }

        if options.include_large_files {
            let drives = self.lists.get("DRIVES").cloned().unwrap_or_default();
            let found = crate::large_files::find(
                &drives,
                &self.platform.large_file_skip(),
                crate::large_files::DEFAULT_MIN_BYTES,
                &self.guard,
            );
            match found {
                Some(s) => {
                    report(&s);
                    scans.push(s);
                }
                None => {
                    done.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        // Large files go last so that caches claim their files first.
        let large = scans.iter().position(|s| s.finding.rule_id == crate::large_files::RULE_ID);
        if let Some(i) = large {
            let s = scans.remove(i);
            dedupe_across_rules(&mut scans);
            let mut s = s;
            let taken: HashSet<PathBuf> = scans.iter().flat_map(|r| r.entries.iter().map(|e| e.path.clone())).collect();
            s.entries.retain(|e| !taken.contains(&e.path));
            s.finding.top_entries.retain(|e| !taken.contains(&e.path));
            s.finding.bytes = s.entries.iter().map(|e| e.bytes).sum();
            s.finding.files = s.entries.len() as u64;
            scans.push(s);
        } else {
            dedupe_across_rules(&mut scans);
        }
        scans.retain(|s| s.finding.bytes > 0 || s.special.is_some() && s.finding.files > 0);
        scans.sort_by_key(|x| std::cmp::Reverse(x.finding.bytes));
        ScanResult {
            rules: scans,
            duration: started.elapsed(),
        }
    }

    fn finding_for(&self, rule: &Rule, roots: Vec<PathBuf>) -> RuleFinding {
        let running_processes = rule
            .close_processes
            .iter()
            .filter(|p| self.running.contains(&p.to_lowercase()))
            .cloned()
            .collect();
        RuleFinding {
            rule_id: rule.id.clone(),
            name: rule.name.clone(),
            description: rule.description.clone(),
            category: rule.category,
            risk: rule.risk,
            requires_admin: rule.requires_admin,
            selected_by_default: rule.selected_by_default(),
            bytes: 0,
            files: 0,
            running_processes,
            top_entries: Vec::new(),
            itemized: false,
            roots,
        }
    }

    /// Returns `None` when the rule does not apply to this machine
    /// (nothing at any of its paths).
    pub fn scan_rule(&self, rule: &Rule) -> Option<RuleScan> {
        if let Some(special) = rule.special {
            return self.scan_special(rule, special);
        }

        let mut roots: Vec<PathBuf> = rule
            .paths
            .iter()
            .flat_map(|t| expand_lists(t, &self.lists))
            .flat_map(|t| resolve_roots(&t, &self.vars))
            .filter(|r| self.guard.allows_root(r))
            .collect();
        roots.sort();
        roots.dedup();
        // A root nested in another root would be counted twice.
        let snapshot = roots.clone();
        roots.retain(|r| !snapshot.iter().any(|o| o != r && r.starts_with(o)));
        if roots.is_empty() {
            return None;
        }

        let include = build_globset(&rule.include);
        let exclude = build_globset(&rule.exclude);
        let min_age = Duration::from_secs(rule.min_age_hours * 3600);

        let mut entries = Vec::new();
        let mut per_root = Vec::with_capacity(roots.len());
        for root in &roots {
            let before = entries.len();
            match rule.keep_newest {
                Some(keep) => self.old_versions(root, keep, &mut entries),
                None => self.walk_root(rule, root, include.as_ref(), exclude.as_ref(), min_age, &mut entries),
            }
            let bytes: u64 = entries[before..].iter().map(|e| e.bytes).sum();
            per_root.push(EntryPreview { path: root.clone(), bytes, note: None });
        }

        let mut finding = self.finding_for(rule, roots);
        finding.bytes = entries.iter().map(|e| e.bytes).sum();
        finding.files = entries.len() as u64;
        let itemize = rule.risk == Risk::Expert && !entries.is_empty() && entries.len() <= ITEMIZED_MAX;
        finding.itemized = itemize;
        finding.top_entries = if itemize {
            entries.sort_by_key(|x| std::cmp::Reverse(x.bytes));
            entries
                .iter()
                .map(|e| EntryPreview { path: e.path.clone(), bytes: e.bytes, note: None })
                .collect()
        } else if per_root.len() > 1 {
            per_root.sort_by_key(|x| std::cmp::Reverse(x.bytes));
            per_root.retain(|p| p.bytes > 0);
            per_root.truncate(TOP_ENTRIES);
            per_root
        } else {
            let mut top: Vec<&Entry> = entries.iter().collect();
            top.sort_by_key(|x| std::cmp::Reverse(x.bytes));
            top.iter()
                .take(TOP_ENTRIES)
                .map(|e| EntryPreview { path: e.path.clone(), bytes: e.bytes, note: None })
                .collect()
        };
        Some(RuleScan {
            finding,
            entries,
            rule: Some(rule.clone()),
            special: None,
        })
    }

    fn walk_root(
        &self,
        rule: &Rule,
        root: &Path,
        include: Option<&GlobSet>,
        exclude: Option<&GlobSet>,
        min_age: Duration,
        out: &mut Vec<Entry>,
    ) {
        let mut walker = WalkDir::new(root).follow_links(false);
        if let Some(depth) = rule.max_depth {
            walker = walker.max_depth(depth);
        }
        for entry in walker.into_iter().flatten() {
            let ft = entry.file_type();
            if ft.is_symlink() || !ft.is_file() {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            if is_placeholder(&meta) {
                continue;
            }
            let path = entry.path();
            let rel = path
                .strip_prefix(root)
                .ok()
                .filter(|r| !r.as_os_str().is_empty())
                .map(|r| r.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|| entry.file_name().to_string_lossy().into_owned());
            if include.is_some_and(|g| !g.is_match(&rel)) || exclude.is_some_and(|g| g.is_match(&rel)) {
                continue;
            }
            if !min_age.is_zero() {
                match meta.modified() {
                    Ok(m) if age_of(m) >= min_age => {}
                    _ => continue,
                }
            }
            if !self.guard.allows_entry(path, root) {
                continue;
            }
            out.push(Entry {
                path: path.to_path_buf(),
                root: root.to_path_buf(),
                bytes: meta.len(),
                tree: false,
            });
        }
    }

    fn old_versions(&self, root: &Path, keep: usize, out: &mut Vec<Entry>) {
        let Ok(children) = std::fs::read_dir(root) else { return };
        let mut dirs: Vec<(std::time::SystemTime, PathBuf)> = children
            .flatten()
            .filter(|c| c.file_type().is_ok_and(|t| t.is_dir() && !t.is_symlink()))
            .filter_map(|c| Some((c.metadata().ok()?.modified().ok()?, c.path())))
            .collect();
        dirs.sort_by_key(|d| std::cmp::Reverse(d.0));
        for (_, dir) in dirs.into_iter().skip(keep) {
            if !self.guard.allows_entry(&dir, root) {
                continue;
            }
            let Some(bytes) = tree_size(&dir) else { continue };
            out.push(Entry { path: dir, root: root.to_path_buf(), bytes, tree: true });
        }
    }

    fn scan_special(&self, rule: &Rule, special: Special) -> Option<RuleScan> {
        match special {
            Special::Trash => {
                let (bytes, items) = self.platform.trash_info().ok().flatten()?;
                let mut finding = self.finding_for(rule, Vec::new());
                finding.bytes = bytes;
                finding.files = items;
                Some(RuleScan {
                    finding,
                    entries: Vec::new(),
                    rule: Some(rule.clone()),
                    special: Some(special),
                })
            }
        }
    }
}

/// Total file size of a tree, or `None` if it contains a link.
pub(crate) fn tree_size(dir: &Path) -> Option<u64> {
    let mut bytes = 0;
    for entry in WalkDir::new(dir).follow_links(false) {
        let entry = entry.ok()?;
        if entry.file_type().is_symlink() {
            return None;
        }
        if entry.file_type().is_file() {
            bytes += entry.metadata().ok()?.len();
        }
    }
    Some(bytes)
}

/// Rules can overlap (an app's temp folder inside `%TEMP%`). Each file is
/// kept by the most specific rule, the one with the deepest root, so sizes
/// are not counted twice.
fn dedupe_across_rules(scans: &mut [RuleScan]) {
    let depth = |s: &RuleScan| s.entries.iter().map(|e| e.root.components().count()).max().unwrap_or(0);
    let mut order: Vec<usize> = (0..scans.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(depth(&scans[i])));
    let mut seen: HashSet<PathBuf> = HashSet::new();
    for i in order {
        let scan = &mut scans[i];
        let before = scan.entries.len();
        scan.entries.retain(|e| seen.insert(e.path.clone()));
        if scan.entries.len() != before {
            if scan.finding.itemized {
                let kept: HashSet<&PathBuf> = scan.entries.iter().map(|e| &e.path).collect();
                scan.finding.top_entries.retain(|e| kept.contains(&e.path));
            }
            scan.finding.bytes = scan.entries.iter().map(|e| e.bytes).sum();
            scan.finding.files = scan.entries.len() as u64;
        }
    }
}

pub(crate) fn build_globset(patterns: &[String]) -> Option<GlobSet> {
    if patterns.is_empty() {
        return None;
    }
    let mut b = GlobSetBuilder::new();
    for p in patterns {
        if let Ok(g) = GlobBuilder::new(p).case_insensitive(true).literal_separator(false).build() {
            b.add(g);
        }
    }
    b.build().ok()
}

/// Cloud-only files (OneDrive "online-only") must not be touched: reading
/// them triggers a download and they occupy no local space anyway.
#[cfg(windows)]
fn is_placeholder(meta: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const OFFLINE: u32 = 0x1000;
    const RECALL_ON_OPEN: u32 = 0x40000;
    const RECALL_ON_DATA_ACCESS: u32 = 0x400000;
    meta.file_attributes() & (OFFLINE | RECALL_ON_OPEN | RECALL_ON_DATA_ACCESS) != 0
}

#[cfg(not(windows))]
fn is_placeholder(_meta: &std::fs::Metadata) -> bool {
    false
}
