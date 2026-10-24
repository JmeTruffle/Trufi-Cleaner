//! Last line of defence before anything is deleted.
//!
//! Rules are data and can be wrong (or come from a user pack), so every root
//! and every file is checked here independently of the rule that produced it.

use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct SafetyGuard {
    /// Nothing inside these is ever deleted (System32, WinSxS, ...).
    never_touch: Vec<String>,
    /// A cleaning root may live inside these but must never *be* one of them
    /// or contain one (home, Windows, Program Files, AppData, ...).
    important: Vec<String>,
}

fn key(path: &Path) -> String {
    let s: PathBuf = path.components().collect();
    let mut s = s.to_string_lossy().replace('\\', "/");
    while s.len() > 1 && s.ends_with('/') {
        s.pop();
    }
    if cfg!(windows) {
        s.to_lowercase()
    } else {
        s
    }
}

fn within(child: &str, parent: &str) -> bool {
    child == parent
        || (child.starts_with(parent)
            && (parent.ends_with('/') || child.as_bytes().get(parent.len()) == Some(&b'/')))
}

impl SafetyGuard {
    pub fn new(never_touch: Vec<PathBuf>, important: Vec<PathBuf>) -> Self {
        SafetyGuard {
            never_touch: never_touch.iter().map(|p| key(p)).collect(),
            important: important.iter().map(|p| key(p)).collect(),
        }
    }

    /// Whether `root` is specific enough to clean inside.
    pub fn allows_root(&self, root: &Path) -> bool {
        let depth = root
            .components()
            .filter(|c| matches!(c, Component::Normal(_)))
            .count();
        if depth < 2 || !root.is_absolute() {
            return false;
        }
        let k = key(root);
        if self.never_touch.iter().any(|n| within(&k, n) || within(n, &k)) {
            return false;
        }
        !self.important.iter().any(|i| within(i, &k))
    }

    /// Whether a concrete file or directory found under `root` may be deleted.
    pub fn allows_entry(&self, path: &Path, root: &Path) -> bool {
        if path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
        {
            return false;
        }
        let k = key(path);
        let r = key(root);
        if !within(&k, &r) {
            return false;
        }
        !self.never_touch.iter().any(|n| within(&k, n))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unix-style test paths, made absolute on Windows with a drive letter.
    fn p(s: &str) -> PathBuf {
        if cfg!(windows) && s.starts_with('/') {
            PathBuf::from(format!("C:{s}"))
        } else {
            PathBuf::from(s)
        }
    }

    fn guard() -> SafetyGuard {
        SafetyGuard::new(
            vec![p("/win/System32")],
            vec![p("/home/u"), p("/win"), p("/home/u/AppData/Local")],
        )
    }

    #[test]
    fn roots() {
        let g = guard();
        assert!(g.allows_root(&p("/home/u/AppData/Local/Temp")));
        assert!(g.allows_root(&p("/win/Temp")));
        assert!(!g.allows_root(&p("/home/u")));
        assert!(!g.allows_root(&p("/home")));
        assert!(!g.allows_root(&p("/home/u/AppData")));
        assert!(!g.allows_root(&p("/win")));
        assert!(!g.allows_root(&p("/win/System32/drivers")));
        assert!(!g.allows_root(&p("/")));
        assert!(!g.allows_root(Path::new("relative/path")));
    }

    #[test]
    fn entries() {
        let g = guard();
        let root = &p("/home/u/AppData/Local/Temp");
        assert!(g.allows_entry(&p("/home/u/AppData/Local/Temp/a.tmp"), root));
        assert!(!g.allows_entry(&p("/home/u/AppData/Local/Temporary/a"), root));
        assert!(!g.allows_entry(&p("/home/u/AppData/Local/Temp/../x"), root));
    }
}
