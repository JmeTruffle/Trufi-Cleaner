// Single entry point to the backend. Inside Tauri it calls Rust commands;
// in a plain browser (`npm run dev`) it falls back to a mock so the UI can
// be designed and tested without Windows.

import type {
  CleanReport, CleanRequest, HealthReport, InstalledApp, ScanArgs, ScanEvent, ScanSummary,
  StartupItem, SystemInfo, SystemSnapshot,
} from "./types";

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

type Commands = {
  system_info: [undefined, SystemInfo];
  snapshot: [undefined, SystemSnapshot];
  health: [undefined, HealthReport];
  scan: [{ args: ScanArgs }, ScanSummary];
  clean: [{ requests: CleanRequest[] }, CleanReport];
  startup_items: [undefined, StartupItem[]];
  set_startup_enabled: [{ id: string; enabled: boolean }, void];
  installed_apps: [undefined, InstalledApp[]];
  uninstall_app: [{ id: string }, void];
  relaunch_elevated: [undefined, void];
  reveal_path: [{ path: string }, void];
};

async function call<K extends keyof Commands>(cmd: K, args?: Commands[K][0]): Promise<Commands[K][1]> {
  if (inTauri) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke(cmd, args as Record<string, unknown> | undefined);
  }
  const { mock } = await import("./mock");
  return (mock[cmd] as (a: unknown) => Promise<Commands[K][1]>)(args);
}

export const api = {
  systemInfo: () => call("system_info"),
  snapshot: () => call("snapshot"),
  health: () => call("health"),
  scan: (args: ScanArgs) => call("scan", { args }),
  clean: (requests: CleanRequest[]) => call("clean", { requests }),
  startupItems: () => call("startup_items"),
  setStartupEnabled: (id: string, enabled: boolean) => call("set_startup_enabled", { id, enabled }),
  installedApps: () => call("installed_apps"),
  uninstallApp: (id: string) => call("uninstall_app", { id }),
  relaunchElevated: () => call("relaunch_elevated"),
  revealPath: (path: string) => call("reveal_path", { path }),
};

export async function onScanEvent(cb: (e: ScanEvent) => void): Promise<() => void> {
  if (inTauri) {
    const { listen } = await import("@tauri-apps/api/event");
    return listen<ScanEvent>("scan-event", (e) => cb(e.payload));
  }
  const { mockEvents } = await import("./mock");
  mockEvents.add(cb);
  return () => mockEvents.delete(cb);
}

export async function windowControls() {
  if (!inTauri) return null;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  const w = getCurrentWindow();
  return {
    minimize: () => w.minimize(),
    toggleMaximize: () => w.toggleMaximize(),
    close: () => w.close(),
  };
}
