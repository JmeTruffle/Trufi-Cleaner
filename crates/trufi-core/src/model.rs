//! Data types shared between the core, the platform layer and the UI.
//! Everything here is serialisable so it can cross the Tauri IPC boundary.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// How confident we are that removing something is harmless.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Risk {
    /// Pure caches/temp data that the owner re-creates. Selected by default.
    Safe,
    /// Harmless but has a visible cost (re-login, re-download, slower first start).
    Moderate,
    /// Needs a human look (possible leftovers, large system stores).
    Expert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    System,
    Browsers,
    Apps,
    Games,
    Developer,
    Logs,
    Privacy,
    Leftovers,
}

/// One rule's outcome after a scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleFinding {
    pub rule_id: String,
    pub name: String,
    pub description: String,
    pub category: Category,
    pub risk: Risk,
    pub requires_admin: bool,
    /// Whether the UI should pre-select this finding.
    pub selected_by_default: bool,
    pub bytes: u64,
    pub files: u64,
    /// Processes from the rule's `close_processes` that are running now;
    /// their files are probably locked.
    pub running_processes: Vec<String>,
    /// Largest few entries, for the details view. When `itemized`, this is
    /// every entry and the user picks them one by one.
    pub top_entries: Vec<EntryPreview>,
    pub itemized: bool,
    /// Roots that were scanned, for display.
    pub roots: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryPreview {
    pub path: PathBuf,
    pub bytes: u64,
    /// Why this entry deserves a closer look, e.g. "possible_saves".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanSummary {
    pub findings: Vec<RuleFinding>,
    pub total_bytes: u64,
    pub total_files: u64,
    pub duration_ms: u64,
    pub rules_checked: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScanEvent {
    Started { total_rules: usize },
    RuleDone { rule_id: String, name: String, done: usize, total: usize, bytes: u64 },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CleanReport {
    pub freed_bytes: u64,
    pub removed_files: u64,
    pub skipped_files: u64,
    pub failed_files: u64,
    pub per_rule: Vec<RuleCleanResult>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleCleanResult {
    pub rule_id: String,
    pub freed_bytes: u64,
    pub removed_files: u64,
    pub failed_files: u64,
    /// A handful of error messages for diagnostics.
    pub errors: Vec<String>,
}

/// An installed application as reported by the OS.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstalledApp {
    /// Platform-specific identifier (registry key path on Windows, bundle
    /// path on macOS).
    pub id: String,
    pub name: String,
    pub publisher: Option<String>,
    pub version: Option<String>,
    pub install_location: Option<PathBuf>,
    pub size_bytes: Option<u64>,
    /// The OS still lists the app but its files are gone.
    pub broken: bool,
    pub can_uninstall: bool,
    /// macOS bundle identifier (`com.tinyspeck.slackmacgap`). Data folders
    /// in `~/Library` are often named after it.
    #[serde(default)]
    pub bundle_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupScope {
    CurrentUser,
    AllUsers,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupItem {
    /// Stable identifier the platform understands when toggling.
    pub id: String,
    pub name: String,
    pub command: String,
    /// "registry" or "folder" on Windows, "launch_agent" on macOS, ...
    pub source: String,
    pub scope: StartupScope,
    pub enabled: bool,
    pub requires_admin: bool,
    /// False when the program the entry starts no longer exists.
    pub target_exists: bool,
}

/// What removing an app did, so the UI can say the right thing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UninstallOutcome {
    /// The app's own uninstaller is running and asks the user.
    UninstallerStarted,
    /// The app was moved to the Trash (macOS).
    MovedToTrash,
}
