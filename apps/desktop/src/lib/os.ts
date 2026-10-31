// Which OS the UI runs on, known synchronously at startup so the window
// chrome and the wording never jump once the backend answers. In a plain
// browser (mock backend) `?os=macos` previews the Mac version.

import type { SystemInfo } from "./types";

type Os = SystemInfo["os"];

function detect(): Os {
  const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  if (!inTauri) {
    const param = typeof location !== "undefined" ? new URLSearchParams(location.search).get("os") : null;
    return param === "macos" || param === "linux" ? param : "windows";
  }
  const ua = typeof navigator !== "undefined" ? navigator.userAgent : "";
  return /Mac/.test(ua) ? "macos" : /Windows/.test(ua) ? "windows" : "linux";
}

export const os: Os = detect();
export const isMac = os === "macos";
export const isWindows = os === "windows";

/** "Windows" / "macOS", for sentences. */
export const osName = isMac ? "macOS" : isWindows ? "Windows" : "Linux";

/** Where the OS installs its updates, as the OS names it. */
export const updatesPlace = isMac ? "«Обновление ПО»" : "Центр обновления";
