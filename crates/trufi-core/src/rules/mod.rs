//! Data-driven cleaning rules.
//!
//! A rule pack is a TOML file with a `[pack]` header and a list of
//! `[[rule]]` tables. Packs for every OS are embedded into the binary
//! (see [`builtin`]); extra packs can be dropped into the user rules folder
//! and are loaded with [`RuleSet::load_dir`]. The schema is documented in
//! `docs/RULES.md`.

mod builtin;
mod expand;

pub use builtin::builtin_packs;
pub use expand::{expand_lists, expand_template, resolve_roots};

use crate::model::{Category, Risk};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackHeader {
    pub id: String,
    pub name: String,
    /// "windows", "macos", "linux" or "any".
    pub os: String,
    #[serde(default = "one")]
    pub version: u32,
}

fn one() -> u32 {
    1
}
fn yes() -> bool {
    true
}

/// Rules with special handling that is not a plain file walk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Special {
    /// The OS trash / Recycle Bin, emptied through the platform API.
    Trash,
    /// macOS: local Time Machine snapshots on the system volume. macOS does
    /// not report their size, only how many there are.
    TmSnapshots,
    /// macOS: Xcode simulators whose runtime is no longer installed
    /// (`xcrun simctl delete unavailable`).
    Simulators,
    /// Docker build cache (`docker builder prune`).
    DockerBuildCache,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub category: Category,
    pub risk: Risk,
    /// Pre-select in the UI. Defaults to `risk == safe`.
    #[serde(default)]
    pub default: Option<bool>,
    #[serde(default)]
    pub requires_admin: bool,
    /// Path templates. `%VAR%` and a leading `~` are expanded, `*` and `?`
    /// in any segment are globbed. Each match is a root to clean inside
    /// (a directory) or a single file.
    #[serde(default)]
    pub paths: Vec<String>,
    /// Glob patterns over the path relative to a root. Empty means all files.
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Only files untouched for at least this long are cleaned.
    #[serde(default)]
    pub min_age_hours: u64,
    /// Limit how deep below a root the walk goes (1 = direct children).
    #[serde(default)]
    pub max_depth: Option<usize>,
    /// Remove directories left empty after cleaning (never the root itself).
    #[serde(default = "yes")]
    pub remove_empty_dirs: bool,
    /// Process names that lock these files while running.
    #[serde(default)]
    pub close_processes: Vec<String>,
    #[serde(default)]
    pub special: Option<Special>,
    /// Treat each child folder of a root as one entry and keep the N most
    /// recently modified (old app versions, e.g. Roblox `Versions`).
    #[serde(default)]
    pub keep_newest: Option<usize>,
}

impl Rule {
    pub fn selected_by_default(&self) -> bool {
        self.default.unwrap_or(self.risk == Risk::Safe)
    }
}

#[derive(Debug, Clone, Deserialize)]
struct PackFile {
    pack: PackHeader,
    #[serde(default, rename = "rule")]
    rules: Vec<Rule>,
}

#[derive(Debug, Clone, Default)]
pub struct RuleSet {
    pub packs: Vec<PackHeader>,
    pub rules: Vec<Rule>,
}

impl RuleSet {
    /// Builds the set for one OS from the embedded packs.
    pub fn builtin(os: &str) -> Result<Self> {
        let mut set = RuleSet::default();
        for (name, text) in builtin_packs() {
            set.add_pack_str(name, text, os)?;
        }
        Ok(set)
    }

    /// Parses a pack and adds its rules if it targets `os`.
    /// Rules with an id that already exists replace the earlier rule, so
    /// user packs can override built-in ones.
    pub fn add_pack_str(&mut self, pack_name: &str, text: &str, os: &str) -> Result<()> {
        let invalid = |message: String| Error::InvalidRulePack {
            pack: pack_name.to_string(),
            message,
        };
        let file: PackFile = toml::from_str(text).map_err(|e| invalid(e.to_string()))?;
        if file.pack.os != os && file.pack.os != "any" {
            return Ok(());
        }
        let mut seen = HashSet::new();
        for rule in &file.rules {
            if !seen.insert(rule.id.as_str()) {
                return Err(invalid(format!("duplicate rule id `{}`", rule.id)));
            }
            if rule.paths.is_empty() && rule.special.is_none() {
                return Err(invalid(format!("rule `{}` has no paths", rule.id)));
            }
            for pattern in rule.include.iter().chain(&rule.exclude) {
                globset::Glob::new(pattern)
                    .map_err(|e| invalid(format!("rule `{}`: {e}", rule.id)))?;
            }
        }
        for rule in file.rules {
            self.rules.retain(|r| r.id != rule.id);
            self.rules.push(rule);
        }
        self.packs.push(file.pack);
        Ok(())
    }

    /// Loads every `*.toml` in `dir`. Broken packs are skipped and returned
    /// as errors so one bad file never disables cleaning.
    pub fn load_dir(&mut self, dir: &Path, os: &str) -> Vec<Error> {
        let mut errors = Vec::new();
        let Ok(entries) = std::fs::read_dir(dir) else {
            return errors;
        };
        let mut files: Vec<_> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "toml"))
            .collect();
        files.sort();
        for path in files {
            let name = path.display().to_string();
            match std::fs::read_to_string(&path) {
                Ok(text) => {
                    if let Err(e) = self.add_pack_str(&name, &text, os) {
                        errors.push(e);
                    }
                }
                Err(e) => errors.push(e.into()),
            }
        }
        errors
    }

    pub fn get(&self, id: &str) -> Option<&Rule> {
        self.rules.iter().find(|r| r.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_packs_parse_for_every_os() {
        for os in ["windows", "macos", "linux"] {
            let set = RuleSet::builtin(os).unwrap_or_else(|e| panic!("{os}: {e}"));
            if os != "linux" {
                assert!(!set.rules.is_empty(), "{os} has no rules");
            }
        }
    }

    #[test]
    fn windows_rule_ids_are_unique_across_packs() {
        let mut ids = HashSet::new();
        for (name, text) in builtin_packs() {
            let file: PackFile = toml::from_str(text).unwrap();
            for r in file.rules {
                assert!(ids.insert(r.id.clone()), "{name}: duplicate id {}", r.id);
            }
        }
    }

    #[test]
    fn rejects_unknown_fields_and_duplicates() {
        let mut set = RuleSet::default();
        let bad = r#"
            [pack]
            id = "t"
            name = "t"
            os = "any"
            [[rule]]
            id = "a"
            name = "a"
            category = "system"
            risk = "safe"
            paths = ["/x"]
            pathz = ["/y"]
        "#;
        assert!(set.add_pack_str("bad", bad, "windows").is_err());

        let dup = r#"
            [pack]
            id = "t"
            name = "t"
            os = "any"
            [[rule]]
            id = "a"
            name = "a"
            category = "system"
            risk = "safe"
            paths = ["/x"]
            [[rule]]
            id = "a"
            name = "a"
            category = "system"
            risk = "safe"
            paths = ["/x"]
        "#;
        assert!(set.add_pack_str("dup", dup, "windows").is_err());
    }

    #[test]
    fn later_pack_overrides_rule() {
        let mut set = RuleSet::default();
        let pack = |risk: &str| {
            format!(
                "[pack]\nid='p'\nname='p'\nos='any'\n[[rule]]\nid='a'\nname='a'\ncategory='apps'\nrisk='{risk}'\npaths=['/x']\n"
            )
        };
        set.add_pack_str("1", &pack("safe"), "macos").unwrap();
        set.add_pack_str("2", &pack("expert"), "macos").unwrap();
        assert_eq!(set.rules.len(), 1);
        assert_eq!(set.rules[0].risk, Risk::Expert);
        assert!(!set.rules[0].selected_by_default());
    }
}
