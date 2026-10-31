// Browser-only stand-in for the Rust backend. Numbers resemble a real
// Windows 11 machine (or a Mac with `?os=macos`) so the UI can be judged
// realistically.

import type {
  CleanReport, CleanRequest, DriverInfo, HealthReport, InstalledApp, PendingUpdate, RuleFinding, ScanArgs, ScanEvent,
  ScanSummary, StartupItem, SystemInfo, SystemSnapshot, UpdateStatus,
} from "./types";
import { isMac } from "./os";

const GB = 1024 ** 3;
const MB = 1024 ** 2;
export const mockEvents = new Set<(e: ScanEvent) => void>();
const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

let cpu = 18;
const demoHealth: Record<string, { id: string; penalty: number; value: number }[]> = {
  good: [],
  fair: [
    { id: "system_disk_full", penalty: 12, value: 91.4 },
    { id: "startup_many", penalty: 8, value: 14 },
    { id: "uptime_long", penalty: 6, value: 12.5 },
  ],
  poor: [
    { id: "system_disk_full", penalty: 20, value: 96.2 },
    { id: "memory_high", penalty: 15, value: 92 },
    { id: "junk", penalty: 15, value: 24.6 * 1024 ** 3 },
  ],
  bad: [
    { id: "system_disk_full", penalty: 25, value: 98.7 },
    { id: "memory_high", penalty: 20, value: 96 },
    { id: "cpu_high", penalty: 15, value: 94 },
    { id: "junk", penalty: 15, value: 41.3 * 1024 ** 3 },
  ],
};
let scanned: ScanSummary | null = null;
const macLogin: StartupItem[] = [["Telegram", "/Applications/Telegram.app", true], ["Spotify", "/Applications/Spotify.app", false]].map(
  ([name, path, enabled]) => ({
    id: `login|${path}`, name: name as string, command: path as string, source: "login_item", scope: "current_user",
    enabled: enabled as boolean, requires_admin: false, target_exists: true,
  }),
);
const macStartup: StartupItem[] = [
  ["Google Updater", "com.google.GoogleUpdater.wake", "~/Library/Application Support/Google/GoogleUpdater/Current/GoogleUpdater.app/Contents/MacOS/GoogleUpdater --wake", true, "gui", true],
  ["Zoom", "us.zoom.updater", "/Library/Application Support/zoom.us/ZoomUpdater.app/Contents/MacOS/ZoomUpdater", true, "gui", true],
  ["Docker", "com.docker.vmnetd", "/Library/PrivilegedHelperTools/com.docker.vmnetd", true, "system", true],
  ["Adobe Acrobat Update", "com.adobe.ARMDC.SMJobBlessHelper", "/Library/PrivilegedHelperTools/com.adobe.ARMDC.SMJobBlessHelper", true, "system", true],
  ["Logi Options+", "com.logi.optionsplus.updater", "/Applications/logioptionsplus.app/Contents/Frameworks/logioptionsplus_updater.app/Contents/MacOS/logioptionsplus_updater", false, "gui", false],
].map(([name, label, command, enabled, domain, exists]) => ({
  id: `${domain}|${label}`,
  name: name as string,
  command: command as string,
  source: domain === "system" ? "launch_daemon" : "launch_agent",
  scope: command.toString().startsWith("~") ? "current_user" : "all_users",
  enabled: enabled as boolean,
  requires_admin: domain === "system",
  target_exists: exists as boolean,
}));

const windowsStartup: StartupItem[] = [
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
const startup = isMac ? [...macLogin, ...macStartup] : windowsStartup;

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
    ...(isMac ? macSnapshot() : {}),
  };
}

function macSnapshot(): Partial<SystemSnapshot> {
  return {
    cpu_brand: "Apple M2 Pro",
    cpu_cores: 12,
    memory_total: 16 * GB,
    memory_used: (11.4 + Math.random() * 0.4) * GB,
    disks: [
      { name: "Macintosh HD", mount: "/", file_system: "apfs", total_bytes: 494 * GB, available_bytes: 61 * GB, kind: "ssd", removable: false, is_system: true },
      { name: "Time Machine", mount: "/Volumes/Time Machine", file_system: "apfs", total_bytes: 1863 * GB, available_bytes: 912 * GB, kind: "hdd", removable: false, is_system: false },
    ],
    top_by_cpu: [
      { name: "Google Chrome Helper (Renderer)", instances: 24, cpu_percent: cpu * 0.3, memory_bytes: 2.9 * GB },
      { name: "WindowServer", instances: 1, cpu_percent: cpu * 0.12, memory_bytes: 0.4 * GB },
      { name: "Telegram", instances: 1, cpu_percent: cpu * 0.05, memory_bytes: 0.5 * GB },
      { name: "kernel_task", instances: 1, cpu_percent: cpu * 0.04, memory_bytes: 0.1 * GB },
      { name: "Code Helper (Renderer)", instances: 9, cpu_percent: cpu * 0.03, memory_bytes: 1.2 * GB },
    ],
    top_by_memory: [
      { name: "Google Chrome Helper (Renderer)", instances: 24, cpu_percent: 6, memory_bytes: 2.9 * GB },
      { name: "Code Helper (Renderer)", instances: 9, cpu_percent: 1, memory_bytes: 1.2 * GB },
      { name: "Telegram", instances: 1, cpu_percent: 1, memory_bytes: 0.5 * GB },
      { name: "WindowServer", instances: 1, cpu_percent: 3, memory_bytes: 0.4 * GB },
      { name: "Finder", instances: 1, cpu_percent: 0.2, memory_bytes: 0.2 * GB },
    ],
    host_name: "MacBook-Pro-Trufi",
    memory_pressure: { level: "normal", percent: 34 },
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

const macApps: InstalledApp[] = [
  ["Xcode", "com.apple.dt.Xcode", "16.1", 11.8 * GB], ["Google Chrome", "com.google.Chrome", "130.0", 1.3 * GB],
  ["Telegram", "ru.keepcoder.Telegram", "11.3", 0.3 * GB], ["Docker", "com.docker.docker", "4.35.1", 2.1 * GB],
  ["Visual Studio Code", "com.microsoft.VSCode", "1.95.2", 0.6 * GB], ["Slack", "com.tinyspeck.slackmacgap", "4.41.105", 0.5 * GB],
  ["GarageBand", "com.apple.garageband10", "10.4.11", 1.9 * GB], ["Safari", "com.apple.Safari", "18.1", 0.1 * GB],
].map(([name, bundle, version, size]) => ({
  id: `/Applications/${name}.app`, name: name as string, publisher: null, version: version as string,
  install_location: `/Applications/${name}.app`, size_bytes: size as number, broken: false,
  can_uninstall: bundle !== "com.apple.Safari", bundle_id: bundle as string,
}));

const windowsApps: InstalledApp[] = [
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
const apps = isMac ? macApps : windowsApps;

const daysAgo = (n: number) => new Date(Date.now() - n * 86400000).toISOString().slice(0, 10);
const update = (title: string, extra: Partial<PendingUpdate> = {}): PendingUpdate => ({
  id: title, title, driver: false, optional: false, definition: false, severity: null, kb: null, size_bytes: 0,
  driver_class: null, driver_provider: null, driver_hardware_id: null, driver_date: null, ...extra,
});
const GPU_HWID = "PCI\\VEN_10DE&DEV_2484&SUBSYS_146B10DE";
let updatesChecked = false;
let driversListed = false;
const macUpdates: UpdateStatus = {
  version: { name: "macOS Sequoia", display_version: "15.1", build: 0, revision: 0, edition: "", build_label: "24B83" },
  servicing_ends: null,
  out_of_support: false,
  pending: [
    update("macOS Sequoia 15.1.1", { id: "macOS Sequoia 15.1.1-24B91", severity: "Important", size_bytes: 1.6 * GB }),
    update("XProtectPlistConfigData", { definition: true, optional: true, size_bytes: 1 * MB }),
  ],
  reboot_required: false,
  last_install: daysAgo(31),
  days_since_install: 31,
  error: null,
  checked_at_ms: Date.now(),
};
const windowsUpdates: UpdateStatus = {
  version: { name: "Windows 11", display_version: "25H2", build: 26200, revision: 6899, edition: "Professional" },
  servicing_ends: "2027-10-12",
  out_of_support: false,
  pending: [
    update("2026-10 Накопительное обновление для Windows 11 Version 25H2 для систем на базе процессоров x64", { severity: "Critical", kb: "KB5068861", size_bytes: 820 * MB }),
    update("Обновление аналитики системы безопасности для Microsoft Defender Antivirus", { definition: true, size_bytes: 60 * MB }),
    update("NVIDIA - Display - 32.0.15.8129", { driver: true, optional: true, driver_class: "Display", driver_provider: "NVIDIA", driver_hardware_id: GPU_HWID, driver_date: daysAgo(40) }),
  ],
  reboot_required: false,
  last_install: daysAgo(24),
  days_since_install: 24,
  error: null,
  checked_at_ms: Date.now(),
};
const updates = isMac ? macUpdates : windowsUpdates;
const drv = (device: string, cls: string, provider: string, version: string, ageDays: number, extra: Partial<DriverInfo> = {}): DriverInfo => {
  const inbox = provider === "Microsoft";
  return {
    id: `ROOT\\${device}`, device, class: cls, provider, version, date: daysAgo(ageDays), inf: "oem12.inf",
    signed: true, problem_code: null, hardware_id: null, inbox, age_days: ageDays, old: !inbox && ageDays >= 3 * 365,
    update: null, ...extra,
  };
};
const drivers: DriverInfo[] = [
  drv("Intel(R) Wireless Bluetooth(R)", "BLUETOOTH", "Intel Corporation", "22.150.0.6", 420, { problem_code: 43 }),
  drv("NVIDIA GeForce RTX 3070", "DISPLAY", "NVIDIA", "32.0.15.6094", 210, {
    hardware_id: GPU_HWID, update: { title: "NVIDIA - Display - 32.0.15.8129", date: daysAgo(40), optional: true },
  }),
  drv("Генератор сигналов Lightshot", "MEDIA", "Skillbrains", "1.0.0.3", 2100, { signed: false }),
  drv("Realtek PCIe GbE Family Controller", "NET", "Realtek", "10.38.1001.2019", 1600),
  drv("Realtek High Definition Audio", "MEDIA", "Realtek Semiconductor Corp.", "6.0.9549.1", 300),
  drv("AMD SMBus", "SYSTEM", "Advanced Micro Devices, Inc", "5.12.0.38", 900),
  drv("AMD GPIO Controller", "SYSTEM", "Advanced Micro Devices, Inc.", "2.2.0.130", 700),
  drv("Logitech G502 HERO", "HIDCLASS", "Logitech", "1.10.82.0", 500),
  drv("Samsung SSD 980 PRO 1TB", "DISKDRIVE", "Microsoft", "10.0.26100.1", 7400),
  drv("Стандартный контроллер SATA AHCI", "HDC", "Microsoft", "10.0.26100.1150", 7400),
  drv("Корневой USB-концентратор (USB 3.0)", "USB", "Microsoft", "10.0.26100.1", 7400),
  drv("HID-клавиатура", "KEYBOARD", "Microsoft", "10.0.26100.1", 7400),
  drv("Монитор PnP (стандартный)", "MONITOR", "Microsoft", "10.0.26100.1", 7400),
  drv("Процессор AMD Ryzen 7 5800X", "PROCESSOR", "Microsoft", "10.0.26100.1", 7400),
];

export const mock = {
  async system_info(): Promise<SystemInfo> {
    if (isMac) {
      return {
        os: "macos", os_label: "macOS Sequoia 15.1", host_name: "MacBook-Pro-Trufi", app_version: "0.1.0",
        is_elevated: false, full_disk_access: new URLSearchParams(location.search).get("fda") === "1",
        rule_count: 46, rule_packs: ["macOS", "Браузеры", "Программы", "Разработка"],
        rule_errors: [], user_rules_dir: "/Users/user/Library/Application Support/app.trufi.cleaner/rules",
      };
    }
    return {
      os: "windows", os_label: "Windows 11 Pro 26300", host_name: "DESKTOP-TRUFI", app_version: "0.1.0",
      is_elevated: false, full_disk_access: null, rule_count: 84, rule_packs: ["Windows", "Браузеры", "Программы", "Игры", "Разработка"],
      rule_errors: [], user_rules_dir: `${R}\\app.trufi.cleaner\\rules`,
    };
  },
  async snapshot() { return snapshot(); },
  async update_status(): Promise<UpdateStatus> {
    if (!updatesChecked) await wait(2500);
    updatesChecked = true;
    return structuredClone(updates);
  },
  async drivers(): Promise<DriverInfo[]> {
    if (isMac) throw { code: "unsupported", message: "" };
    if (!driversListed) await wait(900);
    driversListed = true;
    return drivers.map((d) => ({ ...d }));
  },
  async open_system_page() {},
  async health(): Promise<HealthReport> {
    const junk = scanned?.findings.filter((x) => x.risk !== "expert").reduce((s, x) => s + x.bytes, 0);
    const factors = [
      { id: "system_disk_full", penalty: 5, value: 81.8 },
      { id: "uptime_long", penalty: 4, value: 9.2 },
      { id: "startup_many", penalty: 5, value: startup.filter((s) => s.enabled).length },
      ...(junk ? [{ id: "junk", penalty: 15, value: junk }] : []),
      ...(updatesChecked ? [{ id: "updates_pending", penalty: 12, value: 1 }] : []),
      ...(driversListed ? [{ id: "driver_problems", penalty: 12, value: 1 }, { id: "driver_unsigned", penalty: 6, value: 1 }] : []),
      ...(updatesChecked && driversListed ? [{ id: "driver_updates", penalty: 4, value: 1 }] : []),
    ].filter((x) => x.id !== "startup_many" || x.value >= 8).sort((a, b) => b.penalty - a.penalty);
    // `?health=good|fair|poor|bad` previews the dashboard in other states.
    const demo = demoHealth[new URLSearchParams(location.search).get("health") ?? ""];
    if (demo) factors.splice(0, factors.length, ...demo);
    const score = 100 - factors.reduce((s, x) => s + x.penalty, 0);
    return {
      health: { score, level: score >= 80 ? "good" : score >= 55 ? "fair" : "poor", factors },
      advice: [
        ...(updatesChecked ? [{ id: "updates_pending", severity: "critical" as const, action: "updates" as const, value: 1, subject: null }] : []),
        ...(driversListed ? [{ id: "driver_problems", severity: "warning" as const, action: "drivers" as const, value: 1, subject: null }] : []),
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
    if (s?.requires_admin && !isMac) throw { code: "elevation_required", message: "" };
    if (s) s.enabled = a.enabled;
  },
  async installed_apps() { return apps; },
  async uninstall_app() {
    return isMac ? ("moved_to_trash" as const) : ("uninstaller_started" as const);
  },
  async relaunch_elevated() {},
  async reveal_path() {},
};
