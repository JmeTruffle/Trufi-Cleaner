// Browser-only stand-in for the Rust backend. Numbers resemble a real
// Windows 11 machine so the UI can be judged realistically.

import type {
  CleanReport, CleanRequest, HealthReport, InstalledApp, RuleFinding, ScanArgs, ScanEvent,
  ScanSummary, StartupItem, SystemInfo, SystemSnapshot,
} from "./types";

const GB = 1024 ** 3;
const MB = 1024 ** 2;
export const mockEvents = new Set<(e: ScanEvent) => void>();
const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

let cpu = 18;
let scanned: ScanSummary | null = null;
const startup: StartupItem[] = [
  ["Steam", "\"C:\\Program Files (x86)\\Steam\\steam.exe\" -silent", true, true],
  ["Discord", "C:\\Users\\user\\AppData\\Local\\Discord\\Update.exe --processStart Discord.exe", true, true],
  ["EpicGamesLauncher", "\"C:\\Program Files (x86)\\Epic Games\\Launcher\\EpicGamesLauncher.exe\" -silent", true, true],
  ["OneDrive", "\"C:\\Program Files\\Microsoft OneDrive\\OneDrive.exe\" /background", true, true],
  ["Yandex Browser", "\"C:\\Users\\user\\AppData\\Local\\Yandex\\YandexBrowser\\Application\\browser.exe\" --autorun", true, true],
  ["MicrosoftEdgeAutoLaunch", "\"C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe\" --no-startup-window", true, true],
  ["SunJavaUpdateSched", "\"C:\\Program Files (x86)\\Common Files\\Java\\Java Update\\jusched.exe\"", true, true],
  ["Lightshot", "C:\\Users\\user\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\Startup\\Lightshot.lnk", true, true],
  ["Spotify", "C:\\Users\\user\\AppData\\Roaming\\Spotify\\Spotify.exe /minimized", true, false],
  ["v2RayTun", "D:\\Launchers\\v2RayTun\\v2RayTun.exe", false, false],
  ["RtkAudUService", "\"C:\\Windows\\System32\\RtkAudUService64.exe\" -background", true, true],
].map(([name, command, enabled, exists], i) => ({
  id: `hkcu_run|${name}`,
  name: name as string,
  command: command as string,
  source: i === 7 ? "folder" : "registry",
  scope: i >= 9 ? "all_users" : "current_user",
  enabled: enabled as boolean,
  requires_admin: i >= 10,
  target_exists: exists as boolean,
}));

function snapshot(): SystemSnapshot {
  cpu = Math.max(3, Math.min(97, cpu + (Math.random() - 0.5) * 14));
  const cores = Array.from({ length: 16 }, () => Math.max(0, Math.min(100, cpu + (Math.random() - 0.5) * 30)));
  return {
    timestamp_ms: Date.now(),
    cpu_percent: cpu,
    cpu_per_core: cores,
    cpu_brand: "AMD Ryzen 7 5800X 8-Core Processor",
    cpu_cores: 16,
    cpu_frequency_mhz: 3800,
    memory_total: 32 * GB,
    memory_used: (14.2 + Math.random() * 0.6) * GB,
    swap_total: 14.2 * GB,
    swap_used: 1.1 * GB,
    disks: [
      { name: "System", mount: "C:\\", file_system: "NTFS", total_bytes: 110.8 * GB, available_bytes: 20.2 * GB, kind: "ssd", removable: false, is_system: true },
      { name: "Data", mount: "D:\\", file_system: "NTFS", total_bytes: 1863 * GB, available_bytes: 846 * GB, kind: "ssd", removable: false, is_system: false },
      { name: "Backup", mount: "Z:\\", file_system: "NTFS", total_bytes: 238 * GB, available_bytes: 50.7 * GB, kind: "hdd", removable: false, is_system: false },
    ],
    net_rx_bps: Math.random() * 3 * MB,
    net_tx_bps: Math.random() * 300 * 1024,
    uptime_secs: 9 * 86400 + 5 * 3600,
    process_count: 286,
    top_by_cpu: [
      { name: "chrome.exe", instances: 31, cpu_percent: cpu * 0.35, memory_bytes: 3.4 * GB },
      { name: "Discord.exe", instances: 6, cpu_percent: cpu * 0.12, memory_bytes: 0.9 * GB },
      { name: "steamwebhelper.exe", instances: 8, cpu_percent: cpu * 0.08, memory_bytes: 0.7 * GB },
      { name: "explorer.exe", instances: 1, cpu_percent: cpu * 0.04, memory_bytes: 0.2 * GB },
      { name: "Code.exe", instances: 12, cpu_percent: cpu * 0.03, memory_bytes: 1.6 * GB },
    ],
    top_by_memory: [
      { name: "chrome.exe", instances: 31, cpu_percent: 6, memory_bytes: 3.4 * GB },
      { name: "Code.exe", instances: 12, cpu_percent: 1, memory_bytes: 1.6 * GB },
      { name: "Discord.exe", instances: 6, cpu_percent: 2, memory_bytes: 0.9 * GB },
      { name: "steamwebhelper.exe", instances: 8, cpu_percent: 1, memory_bytes: 0.7 * GB },
      { name: "MsMpEng.exe", instances: 1, cpu_percent: 0.4, memory_bytes: 0.4 * GB },
    ],
    host_name: "DESKTOP-TRUFI",
  };
}

const L = "C:\\Users\\user\\AppData\\Local";
const R = "C:\\Users\\user\\AppData\\Roaming";
const f = (
  rule_id: string, name: string, category: RuleFinding["category"], risk: RuleFinding["risk"],
  size: number, extra: Partial<RuleFinding> = {},
): RuleFinding => ({
  rule_id, name, description: "", category, risk, requires_admin: false,
  selected_by_default: risk === "safe", bytes: size, files: Math.round(size / (180 * 1024)) + 3,
  running_processes: [], top_entries: [], roots: [], itemized: false, ...extra,
});

const findings: RuleFinding[] = [
  f("win.gpu.amd_installers", "Распакованный установщик AMD Software", "system", "safe", 4158 * MB,
    { description: "C:\\AMD\\AMD-Software-Installer остаётся после установки драйвера.", top_entries: [{ path: "C:\\AMD\\AMD-Software-Installer", bytes: 4158 * MB }] }),
  f("app.roblox.old_versions", "Roblox: старые версии", "games", "safe", 1980 * MB,
    { description: "Самая новая версия остаётся.", files: 6, top_entries: [1, 2, 3].map((i) => ({ path: `${L}\\Roblox\\Versions\\version-a1b2c3d${i}`, bytes: 330 * MB })) }),
  f("browser.chrome.cache", "Google Chrome: кеш", "browsers", "safe", 1150 * MB,
    { running_processes: ["chrome.exe"], description: "Кеш страниц, скриптов и шейдеров во всех профилях. Вход на сайты сохранится.", top_entries: [{ path: `${L}\\Google\\Chrome\\User Data\\Default\\Service Worker\\CacheStorage`, bytes: 843 * MB }, { path: `${L}\\Google\\Chrome\\User Data\\Default\\Code Cache`, bytes: 132 * MB }] }),
  f("dev.yarn.cache", "Yarn: кеш пакетов", "developer", "safe", 999 * MB),
  f("app.vscode.cache", "Visual Studio Code: кеш", "apps", "safe", 1081 * MB, { running_processes: ["Code.exe"] }),
  f("win.gpu.nvidia", "Кеш шейдеров NVIDIA", "system", "safe", 789 * MB),
  f("win.update.download", "Загрузки Центра обновления", "system", "moderate", 730 * MB, { requires_admin: true, selected_by_default: false }),
  f("game.steam.cache", "Steam: кеш и журналы", "games", "safe", 510 * MB),
  f("browser.edge.cache", "Microsoft Edge: кеш", "browsers", "safe", 234 * MB),
  f("app.vkplay.cache", "VK Play: кеш", "games", "safe", 339 * MB),
  f("app.unity.cache", "Unity: кеш", "developer", "safe", 262 * MB),
  f("app.onedrive.logs", "OneDrive: журналы", "logs", "safe", 198 * MB),
  f("win.logs", "Журналы Windows", "logs", "safe", 173 * MB, { requires_admin: true }),
  f("browser.yandex.cache", "Яндекс Браузер: кеш", "browsers", "safe", 113 * MB),
  f("win.shader_cache", "Кеш шейдеров DirectX", "system", "safe", 84 * MB),
  f("app.discord.cache", "Discord: кеш", "apps", "safe", 75 * MB, { running_processes: ["Discord.exe"] }),
  f("win.crashdumps", "Дампы памяти программ", "logs", "safe", 17 * MB),
  f("win.thumbcache", "Кеш эскизов", "system", "moderate", 10.6 * MB, { selected_by_default: false }),
  f("win.temp.user", "Временные файлы пользователя", "system", "safe", 3.7 * MB),
  f("win.downloads_installers", "Установщики в Загрузках", "apps", "expert", 147 * MB,
    { itemized: true, files: 2, top_entries: [{ path: "C:\\Users\\user\\Downloads\\BsgLauncher.15.1.0.4635.exe", bytes: 139 * MB }, { path: "C:\\Users\\user\\Downloads\\ChromeSetup.exe", bytes: 8 * MB }] }),
  f("leftovers.apps", "Остатки удалённых программ", "leftovers", "expert", 3_900 * MB, {
    description: "Папки данных программ, которых больше нет в списке установленных. Проверьте каждую папку перед удалением: в папках игр бывают сохранения.",
    files: 9,
    itemized: true,
    top_entries: [
      { path: "C:\\ProgramData\\Hogwarts Legacy", bytes: 1269 * MB },
      { path: `${R}\\MetaQuotes`, bytes: 678 * MB },
      { path: `${L}\\Discovery`, bytes: 401 * MB, note: "possible_saves" },
      { path: "C:\\ProgramData\\MetaQuotes", bytes: 274 * MB },
      { path: `${L}\\Ubisoft`, bytes: 240 * MB },
      { path: `${L}\\TslGame`, bytes: 237 * MB, note: "possible_saves" },
      { path: `${R}\\exjava`, bytes: 201 * MB },
      { path: "C:\\Users\\user\\AppData\\LocalLow\\FeelFreeGames", bytes: 175 * MB, note: "possible_saves" },
      { path: `${L}\\DriveBeyondHorizons`, bytes: 52 * MB, note: "possible_saves" },
    ],
  }),
  f("large_files", "Крупные файлы на всех дисках", "leftovers", "expert", 21.3 * GB, {
    description: "Самые большие файлы вне системных папок и папок программ. Удаляются только те, что вы отметите.",
    files: 4,
    itemized: true,
    top_entries: [
      { path: "D:\\Video\\stream-2026-08-11.mkv", bytes: 9.8 * GB },
      { path: "D:\\ISO\\Win11_24H2_Russian_x64.iso", bytes: 6.1 * GB },
      { path: "Z:\\Backup\\old-ssd.img", bytes: 4.6 * GB },
      { path: "C:\\Users\\user\\Downloads\\dataset.zip", bytes: 0.8 * GB },
    ],
  }),
];

const apps: InstalledApp[] = [
  ["Steam", "Valve Corporation", 0.3 * GB, false], ["Discord", "Discord Inc.", 0.4 * GB, false],
  ["Epic Games Launcher", "Epic Games, Inc.", 0.2 * GB, false], ["Google Chrome", "Google LLC", 0.6 * GB, false],
  ["Java 8 Update 431", "Oracle Corporation", 212 * MB, false], ["Lightshot-5.5.0.7", "Skillbrains", 6 * MB, false],
  ["The Witcher 3: Wild Hunt", "CD PROJEKT RED", null, true], ["Deadlock", "Valve", null, true],
  ["Visual Studio Code", "Microsoft Corporation", 0.4 * GB, false], ["Яндекс Браузер", "ООО «ЯНДЕКС»", 0.6 * GB, false],
  ["AnyDesk", "AnyDesk Software GmbH", 8 * MB, false], ["Microsoft Visual C++ 2013 Redistributable (x64)", "Microsoft Corporation", 20 * MB, false],
].map(([name, publisher, size, broken], i) => ({
  id: `HKLM64\\app${i}`, name: name as string, publisher: publisher as string, version: "1.0",
  install_location: null, size_bytes: size as number | null, broken: broken as boolean, can_uninstall: !broken,
}));

export const mock = {
  async system_info(): Promise<SystemInfo> {
    return {
      os: "windows", os_label: "Windows 11 Pro 26300", host_name: "DESKTOP-TRUFI", app_version: "0.1.0",
      is_elevated: false, rule_count: 84, rule_packs: ["Windows", "Браузеры", "Программы", "Игры", "Разработка"],
      rule_errors: [], user_rules_dir: `${R}\\app.trufi.cleaner\\rules`,
    };
  },
  async snapshot() { return snapshot(); },
  async health(): Promise<HealthReport> {
    const junk = scanned?.findings.filter((x) => x.risk !== "expert").reduce((s, x) => s + x.bytes, 0);
    const factors = [
      { id: "system_disk_full", penalty: 5, value: 81.8 },
      { id: "uptime_long", penalty: 4, value: 9.2 },
      { id: "startup_many", penalty: 5, value: startup.filter((s) => s.enabled).length },
      ...(junk ? [{ id: "junk", penalty: 15, value: junk }] : []),
    ].filter((x) => x.id !== "startup_many" || x.value >= 8);
    const score = 100 - factors.reduce((s, x) => s + x.penalty, 0);
    return {
      health: { score, level: score >= 80 ? "good" : score >= 55 ? "fair" : "poor", factors },
      advice: [
        ...(junk ? [{ id: "junk", severity: "warning" as const, action: "clean" as const, value: junk, subject: null }] : []),
        { id: "system_disk_full", severity: "warning", action: "scan", value: 81.8, subject: null },
        ...(factors.some((x) => x.id === "startup_many") ? [{ id: "startup_many", severity: "info" as const, action: "startup" as const, value: startup.filter((s) => s.enabled).length, subject: null }] : []),
        { id: "uptime_long", severity: "info", action: "restart", value: 9.2, subject: null },
        ...(!scanned ? [{ id: "never_scanned", severity: "info" as const, action: "scan" as const, value: 0, subject: null }] : []),
      ],
    };
  },
  async scan(a?: { args: ScanArgs }): Promise<ScanSummary> {
    const args = a?.args ?? {};
    const list = findings.filter((x) =>
      (x.rule_id !== "leftovers.apps" || args.include_leftovers) &&
      (x.rule_id !== "large_files" || args.include_large_files) &&
      (!args.max_risk || x.risk === "safe"));
    mockEvents.forEach((cb) => cb({ kind: "started", total_rules: 84 }));
    for (let i = 1; i <= 84; i++) {
      await wait(18);
      const x = list[i % list.length];
      mockEvents.forEach((cb) => cb({ kind: "rule_done", rule_id: x.rule_id, name: x.name, done: i, total: 84, bytes: x.bytes }));
    }
    scanned = {
      findings: list.map((x) => ({ ...x })),
      total_bytes: list.reduce((s, x) => s + x.bytes, 0),
      total_files: list.reduce((s, x) => s + x.files, 0),
      duration_ms: 1520,
      rules_checked: 84,
    };
    return scanned;
  },
  async clean(a: { requests: CleanRequest[] }): Promise<CleanReport> {
    await wait(900);
    const per_rule = a.requests.map((r) => {
      const x = findings.find((y) => y.rule_id === r.rule_id)!;
      const freed = r.paths ? x.top_entries.filter((e) => r.paths!.includes(e.path)).reduce((s, e) => s + e.bytes, 0) : x.bytes;
      const locked = x.running_processes.length ? Math.round(x.files * 0.1) : 0;
      return { rule_id: r.rule_id, freed_bytes: freed * (locked ? 0.9 : 1), removed_files: x.files - locked, failed_files: locked, errors: [] };
    });
    scanned = null;
    return {
      freed_bytes: per_rule.reduce((s, r) => s + r.freed_bytes, 0),
      removed_files: per_rule.reduce((s, r) => s + r.removed_files, 0),
      failed_files: per_rule.reduce((s, r) => s + r.failed_files, 0),
      skipped_files: 0, per_rule, duration_ms: 900,
    };
  },
  async startup_items() { return startup.map((s) => ({ ...s })); },
  async set_startup_enabled(a: { id: string; enabled: boolean }) {
    const s = startup.find((x) => x.id === a.id);
    if (s?.requires_admin) throw { code: "elevation_required", message: "" };
    if (s) s.enabled = a.enabled;
  },
  async installed_apps() { return apps; },
  async uninstall_app() {},
  async relaunch_elevated() {},
  async reveal_path() {},
};
