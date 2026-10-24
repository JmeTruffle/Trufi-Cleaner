//! Finds big files on every drive the platform lists, so the user can
//! decide about them. Never pre-selected: these are the user's own files.

use crate::model::{Category, EntryPreview, Risk, RuleFinding};
use crate::safety::SafetyGuard;
use crate::scanner::{Entry, RuleScan};
use std::path::PathBuf;
use walkdir::WalkDir;

pub const RULE_ID: &str = "large_files";
pub const DEFAULT_MIN_BYTES: u64 = 512 * 1024 * 1024;
const MAX_LISTED: usize = 100;

/// `roots`: drive roots. `skip`: subtrees that hold OS or program files and
/// are not worth showing (Windows, Program Files, game installs...).
pub fn find(roots: &[PathBuf], skip: &[PathBuf], min_bytes: u64, guard: &SafetyGuard) -> Option<RuleScan> {
    let skip_lower: Vec<String> = skip.iter().map(|p| p.to_string_lossy().to_lowercase()).collect();
    let skipped = |p: &std::path::Path| {
        let s = p.to_string_lossy().to_lowercase();
        skip_lower.iter().any(|k| s.starts_with(k.as_str()))
    };
    let mut entries = Vec::new();
    for root in roots {
        let walker = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !(e.file_type().is_dir() && skipped(e.path())));
        for e in walker.flatten() {
            if !e.file_type().is_file() {
                continue;
            }
            let Ok(meta) = e.metadata() else { continue };
            if meta.len() < min_bytes {
                continue;
            }
            let path = e.path().to_path_buf();
            let parent = path.parent().map(PathBuf::from).unwrap_or_default();
            // The file's own folder is the root for the guard: a drive root
            // itself never passes `allows_root`.
            if !guard.allows_entry(&path, &parent) || parent.components().count() < 2 {
                continue;
            }
            entries.push(Entry { root: parent, path, bytes: meta.len(), tree: false });
        }
    }
    if entries.is_empty() {
        return None;
    }
    entries.sort_by_key(|x| std::cmp::Reverse(x.bytes));
    entries.truncate(MAX_LISTED);
    let finding = RuleFinding {
        rule_id: RULE_ID.into(),
        name: "Крупные файлы на всех дисках".into(),
        description: "Самые большие файлы вне системных папок и папок программ. Удаляются только те, что вы отметите."
            .into(),
        category: Category::Leftovers,
        risk: Risk::Expert,
        requires_admin: false,
        selected_by_default: false,
        bytes: entries.iter().map(|e| e.bytes).sum(),
        files: entries.len() as u64,
        running_processes: Vec::new(),
        itemized: true,
        top_entries: entries
            .iter()
            .map(|e| EntryPreview { path: e.path.clone(), bytes: e.bytes, note: None })
            .collect(),
        roots: roots.to_vec(),
    };
    Some(RuleScan { finding, entries, rule: None, special: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_big_files_and_skips_excluded() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::create_dir_all(root.join("media")).unwrap();
        std::fs::create_dir_all(root.join("Windows")).unwrap();
        std::fs::write(root.join("media/big.mkv"), vec![0u8; 4096]).unwrap();
        std::fs::write(root.join("media/small.txt"), vec![0u8; 10]).unwrap();
        std::fs::write(root.join("Windows/big.sys"), vec![0u8; 4096]).unwrap();
        let guard = SafetyGuard::new(vec![], vec![]);
        let found = find(std::slice::from_ref(&root), &[root.join("Windows")], 1024, &guard).unwrap();
        assert_eq!(found.entries.len(), 1);
        assert!(found.entries[0].path.ends_with("media/big.mkv"));
    }
}
