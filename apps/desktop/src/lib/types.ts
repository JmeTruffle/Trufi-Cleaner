// Mirrors the serde types in trufi-core. Keep in sync with crates/trufi-core/src.

export type Risk = "safe" | "moderate" | "expert";
export type Category =
  | "system" | "browsers" | "apps" | "games" | "developer" | "logs" | "privacy" | "leftovers";

export interface EntryPreview { path: string; bytes: number; note?: string }

export interface RuleFinding {
  rule_id: string;
  name: string;
  description: string;
  category: Category;
  risk: Risk;
  requires_admin: boolean;
  selected_by_default: boolean;
  bytes: number;
  files: number;
  running_processes: string[];
  top_entries: EntryPreview[];
  itemized: boolean;
  roots: string[];
}

export interface ScanSummary {
  findings: RuleFinding[];
  total_bytes: number;
  total_files: number;
  duration_ms: number;
  rules_checked: number;
}

export type ScanEvent =
  | { kind: "started"; total_rules: number }
  | { kind: "rule_done"; rule_id: string; name: string; done: number; total: number; bytes: number };

export interface RuleCleanResult {
  rule_id: string;
  freed_bytes: number;
  removed_files: number;
  failed_files: number;
  errors: string[];
}

export interface CleanReport {
  freed_bytes: number;
  removed_files: number;
  skipped_files: number;
  failed_files: number;
  per_rule: RuleCleanResult[];
  duration_ms: number;
}

export interface CleanRequest { rule_id: string; paths?: string[] | null }

export interface DiskInfo {
  name: string;
  mount: string;
  file_system: string;
  total_bytes: number;
  available_bytes: number;
  kind: "ssd" | "hdd" | "unknown";
  removable: boolean;
  is_system: boolean;
}

export interface ProcessGroup { name: string; instances: number; cpu_percent: number; memory_bytes: number }

export interface SystemSnapshot {
  timestamp_ms: number;
  cpu_percent: number;
  cpu_per_core: number[];
  cpu_brand: string;
  cpu_cores: number;
  cpu_frequency_mhz: number;
  memory_total: number;
  memory_used: number;
  swap_total: number;
  swap_used: number;
  disks: DiskInfo[];
  net_rx_bps: number;
  net_tx_bps: number;
  uptime_secs: number;
  process_count: number;
  top_by_cpu: ProcessGroup[];
  top_by_memory: ProcessGroup[];
  host_name: string;
}

export interface HealthFactor { id: string; penalty: number; value: number }
export interface Health { score: number; level: "good" | "fair" | "poor"; factors: HealthFactor[] }
export type AdviceAction = "scan" | "clean" | "startup" | "processes" | "restart" | "updates" | "drivers";
export interface Advice {
  id: string;
  severity: "info" | "warning" | "critical";
  action: AdviceAction | null;
  value: number;
  subject: string | null;
}
export interface HealthReport { health: Health; advice: Advice[] }

export interface StartupItem {
  id: string;
  name: string;
  command: string;
  source: string;
  scope: "current_user" | "all_users";
  enabled: boolean;
  requires_admin: boolean;
  target_exists: boolean;
}

export interface InstalledApp {
  id: string;
  name: string;
  publisher: string | null;
  version: string | null;
  install_location: string | null;
  size_bytes: number | null;
  broken: boolean;
  can_uninstall: boolean;
}

export interface SystemInfo {
  os: "windows" | "macos" | "linux";
  os_label: string;
  host_name: string;
  app_version: string;
  is_elevated: boolean;
  rule_count: number;
  rule_packs: string[];
  rule_errors: string[];
  user_rules_dir: string | null;
}

export interface WindowsVersion {
  name: string;
  display_version: string | null;
  build: number;
  revision: number;
  edition: string;
}

export interface PendingUpdate {
  id: string;
  title: string;
  driver: boolean;
  optional: boolean;
  definition: boolean;
  severity: string | null;
  kb: string | null;
  size_bytes: number;
  driver_class: string | null;
  driver_provider: string | null;
  driver_hardware_id: string | null;
  driver_date: string | null;
}

export interface UpdateStatus {
  version: WindowsVersion | null;
  servicing_ends: string | null;
  out_of_support: boolean;
  pending: PendingUpdate[];
  reboot_required: boolean;
  last_install: string | null;
  days_since_install: number | null;
  error: string | null;
  checked_at_ms: number;
}

export interface DriverUpdate { title: string; date: string | null; optional: boolean }

export interface DriverInfo {
  id: string;
  device: string;
  class: string | null;
  provider: string | null;
  version: string | null;
  date: string | null;
  inf: string | null;
  signed: boolean | null;
  problem_code: number | null;
  hardware_id: string | null;
  inbox: boolean;
  age_days: number | null;
  old: boolean;
  update: DriverUpdate | null;
}

export type SystemPage = "windows_update" | "optional_updates" | "device_manager";

export interface CommandError { code: string; message: string }

export interface ScanArgs {
  rule_ids?: string[];
  include_leftovers?: boolean;
  include_large_files?: boolean;
  max_risk?: Risk;
}
