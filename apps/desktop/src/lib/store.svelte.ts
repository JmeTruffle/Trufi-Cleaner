// App-wide reactive state. Lives outside components so live charts keep
// their history and a scan keeps running while the user switches pages.

import { api, onScanEvent } from "./api";
import { checkForUpdate, type AvailableUpdate } from "./updater";
import { errorText } from "./i18n";
import { os } from "./os";
import type {
  CleanReport, CleanRequest, DriverInfo, HealthReport, RuleFinding, ScanArgs, ScanSummary, SystemInfo,
  SystemSnapshot, UpdateStatus,
} from "./types";

export type Page = "overview" | "cleaner" | "optimize" | "drivers" | "settings";
/** Progress of a slow system check (OS updates, driver list). */
export type CheckState = "idle" | "loading" | "done" | "error";

const HISTORY = 60;
const UPDATE_CHECK_MS = 6 * 60 * 60 * 1000;
const POLL_MS = 1500;

function load<T>(key: string, fallback: T): T {
  try {
    const v = localStorage.getItem(key);
    return v ? (JSON.parse(v) as T) : fallback;
  } catch {
    return fallback;
  }
}

function save(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    /* storage unavailable: settings just won't persist */
  }
}

class AppStore {
  page = $state<Page>("overview");
  optimizeTab = $state<"startup" | "apps" | "processes">("startup");
  info = $state<SystemInfo | null>(null);

  snapshot = $state<SystemSnapshot | null>(null);
  cpuHistory = $state<number[]>([]);
  memHistory = $state<number[]>([]);
  netHistory = $state<number[]>([]);
  health = $state<HealthReport | null>(null);

  updates = $state<UpdateStatus | null>(null);
  updatesState = $state<CheckState>("idle");
  updatesError = $state("");
  drivers = $state<DriverInfo[] | null>(null);
  driversState = $state<CheckState>("idle");
  driversError = $state("");

  scanning = $state(false);
  scanProgress = $state({ done: 0, total: 0, current: "" });
  scan = $state<ScanSummary | null>(null);
  /** rule id -> selected; per-entry selection for rules reviewed item by item. */
  selected = $state<Record<string, boolean>>({});
  selectedEntries = $state<Record<string, string[]>>({});
  cleaning = $state(false);
  lastClean = $state<CleanReport | null>(null);
  scanOptions = $state(load("scanOptions", { leftovers: true, largeFiles: true }));

  update = $state<AvailableUpdate | null>(null);
  updateState = $state<"idle" | "checking" | "none" | "downloading" | "error">("idle");
  updateProgress = $state<number | null>(null);
  updateDismissed = $state(false);

  private timer: ReturnType<typeof setInterval> | null = null;
  private healthTick = 0;

  async start() {
    this.info = await api.systemInfo().catch(() => null);
    await this.poll();
    this.refreshHealth();
    this.timer ??= setInterval(() => this.poll(), POLL_MS);
    // Both checks are slow (WMI, Windows Update, softwareupdate); run them
    // in the background and let the score pick them up when they finish.
    // Drivers are a Windows notion; macOS manages them itself.
    const system = this.info?.os ?? os;
    if (system === "windows") this.loadDrivers().then(() => this.loadUpdates());
    else if (system === "macos") {
      this.loadUpdates();
      // Full Disk Access is granted in System Settings; pick it up on return.
      window.addEventListener("focus", () => this.refreshInfo());
    }
    setTimeout(() => this.checkUpdate(), 3000);
    setInterval(() => this.checkUpdate(), UPDATE_CHECK_MS);
    onScanEvent((e) => {
      if (e.kind === "started") this.scanProgress = { done: 0, total: e.total_rules, current: "" };
      else this.scanProgress = { done: e.done, total: e.total, current: e.name };
    });
  }

  private async poll() {
    try {
      const s = await api.snapshot();
      this.snapshot = s;
      const push = (arr: number[], v: number) => [...arr, v].slice(-HISTORY);
      this.cpuHistory = push(this.cpuHistory, s.cpu_percent);
      this.memHistory = push(this.memHistory, (s.memory_used / Math.max(1, s.memory_total)) * 100);
      this.netHistory = push(this.netHistory, s.net_rx_bps + s.net_tx_bps);
      if (++this.healthTick % 10 === 0) this.refreshHealth();
    } catch {
      /* transient: next tick retries */
    }
  }

  async checkUpdate() {
    if (this.updateState === "checking" || this.updateState === "downloading") return;
    this.updateState = "checking";
    try {
      this.update = await checkForUpdate();
      this.updateState = this.update ? "idle" : "none";
    } catch {
      // Offline or no release yet: stay quiet, try again later.
      this.updateState = "error";
    }
  }

  async installUpdate() {
    if (!this.update) return;
    this.updateState = "downloading";
    this.updateProgress = 0;
    try {
      await this.update.install((f) => (this.updateProgress = f));
    } catch {
      this.updateState = "error";
    }
  }

  async loadUpdates(refresh = false) {
    if (this.updatesState === "loading") return;
    this.updatesState = "loading";
    try {
      this.updates = await api.updateStatus(refresh);
      this.updatesState = "done";
      // Driver offers from Windows Update are linked to devices server-side.
      if (this.drivers) this.drivers = await api.drivers(false);
    } catch (e) {
      this.updatesError = errorText(e);
      this.updatesState = "error";
    }
    this.refreshHealth();
  }

  async loadDrivers(refresh = false) {
    if (this.driversState === "loading") return;
    this.driversState = "loading";
    try {
      this.drivers = await api.drivers(refresh);
      this.driversState = "done";
    } catch (e) {
      this.driversError = errorText(e);
      this.driversState = "error";
    }
    this.refreshHealth();
  }

  /** Re-reads system info (macOS: after granting Full Disk Access). */
  async refreshInfo() {
    this.info = await api.systemInfo().catch(() => this.info);
  }

  async refreshHealth() {
    this.health = await api.health().catch(() => this.health);
  }

  setScanOptions(o: { leftovers: boolean; largeFiles: boolean }) {
    this.scanOptions = o;
    save("scanOptions", o);
  }

  async runScan(quick = false) {
    if (this.scanning || this.cleaning) return;
    this.scanning = true;
    this.lastClean = null;
    this.scanProgress = { done: 0, total: 0, current: "" };
    try {
      const args: ScanArgs = quick
        ? { max_risk: "safe" }
        : { include_leftovers: this.scanOptions.leftovers, include_large_files: this.scanOptions.largeFiles };
      const result = await api.scan(args);
      this.scan = result;
      this.selected = Object.fromEntries(result.findings.map((f) => [f.rule_id, f.selected_by_default]));
      this.selectedEntries = {};
      this.refreshHealth();
    } finally {
      this.scanning = false;
    }
  }

  /** Rules whose entries are reviewed one by one instead of all at once. */
  isItemized(f: RuleFinding) {
    return f.itemized;
  }

  selectedBytes(): number {
    if (!this.scan) return 0;
    let total = 0;
    for (const f of this.scan.findings) {
      if (this.isItemized(f)) {
        const picked = new Set(this.selectedEntries[f.rule_id] ?? []);
        total += f.top_entries.filter((e) => picked.has(e.path)).reduce((s, e) => s + e.bytes, 0);
      } else if (this.selected[f.rule_id]) total += f.bytes;
    }
    return total;
  }

  async runClean() {
    if (!this.scan || this.cleaning) return;
    const requests = this.scan.findings.flatMap((f): CleanRequest[] => {
      if (this.isItemized(f)) {
        const paths = this.selectedEntries[f.rule_id] ?? [];
        return paths.length ? [{ rule_id: f.rule_id, paths }] : [];
      }
      return this.selected[f.rule_id] ? [{ rule_id: f.rule_id, paths: null }] : [];
    });
    if (!requests.length) return;
    this.cleaning = true;
    try {
      this.lastClean = await api.clean(requests);
      this.scan = null;
      this.refreshHealth();
    } finally {
      this.cleaning = false;
    }
  }
}

export const app = new AppStore();
