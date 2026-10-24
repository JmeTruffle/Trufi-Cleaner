// Self-update through the Tauri updater plugin. Releases are published by
// .github/workflows/release.yml; the endpoint lives in tauri.conf.json.

import { inTauri } from "./api";

export interface AvailableUpdate {
  version: string;
  notes: string;
  /** Downloads, installs and restarts. `progress` gets 0..1 when the size is known. */
  install(progress: (fraction: number | null) => void): Promise<void>;
}

export async function checkForUpdate(): Promise<AvailableUpdate | null> {
  if (!inTauri) return mockUpdate();
  const { check } = await import("@tauri-apps/plugin-updater");
  const update = await check();
  if (!update) return null;
  return {
    version: update.version,
    notes: update.body ?? "",
    async install(progress) {
      let total = 0;
      let done = 0;
      await update.downloadAndInstall((e) => {
        if (e.event === "Started") total = e.data.contentLength ?? 0;
        else if (e.event === "Progress") {
          done += e.data.chunkLength;
          progress(total ? Math.min(1, done / total) : null);
        } else if (e.event === "Finished") progress(1);
      });
      // On Windows the installer closes the app itself; elsewhere restart.
      const { relaunch } = await import("@tauri-apps/plugin-process");
      await relaunch();
    },
  };
}

function mockUpdate(): AvailableUpdate {
  return {
    version: "0.1.7",
    notes: "Новые правила очистки и ускорение сканирования.",
    async install(progress) {
      for (let i = 1; i <= 20; i++) {
        await new Promise((r) => setTimeout(r, 80));
        progress(i / 20);
      }
    },
  };
}
