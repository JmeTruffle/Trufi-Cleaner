// UI wording. The core sends ids and numbers; this file turns them into text.
// Russian first; add another locale by adding a dictionary with the same keys.

import { bytes, duration, percent, plural } from "./format";
import type { Advice, Category, HealthFactor, Risk } from "./types";

export const categoryName: Record<Category, string> = {
  system: "Система",
  browsers: "Браузеры",
  apps: "Программы",
  games: "Игры",
  developer: "Разработка",
  logs: "Журналы и отчёты",
  privacy: "Конфиденциальность",
  leftovers: "Требуют проверки",
};

export const categoryOrder: Category[] = [
  "system", "browsers", "apps", "games", "developer", "logs", "privacy", "leftovers",
];

export const riskName: Record<Risk, string> = {
  safe: "Безопасно",
  moderate: "Осторожно",
  expert: "Проверьте",
};

export const riskHint: Record<Risk, string> = {
  safe: "Кеш и временные файлы, программы создадут их заново.",
  moderate: "Безвредно, но что-то придётся загрузить заново или войти снова.",
  expert: "Посмотрите, что внутри, прежде чем удалять.",
};

export const healthLevel = {
  good: "Система в хорошем состоянии",
  fair: "Есть что улучшить",
  poor: "Системе нужна помощь",
};

export function factorText(f: HealthFactor): string {
  switch (f.id) {
    case "cpu_high": return `Процессор загружен в среднем на ${percent(f.value)}`;
    case "memory_high": return `Занято ${percent(f.value)} оперативной памяти`;
    case "system_disk_full": return `Системный диск заполнен на ${percent(f.value)}`;
    case "disk_full": return `Один из дисков заполнен на ${percent(f.value)}`;
    case "junk": return `Найдено ${bytes(f.value)} ненужных файлов`;
    case "startup_many": return `${f.value} ${plural(f.value, "программа", "программы", "программ")} в автозагрузке`;
    case "uptime_long": return `Компьютер не перезагружался ${duration(f.value * 86400)}`;
    case "os_unsupported": return "Версия Windows больше не получает обновлений";
    case "updates_pending": return `${f.value} ${plural(f.value, "обновление Windows ждёт", "обновления Windows ждут", "обновлений Windows ждут")} установки`;
    case "updates_restart": return "Обновления ждут перезагрузки";
    case "updates_stale": return `Обновления не ставились ${f.value} ${plural(f.value, "день", "дня", "дней")}`;
    case "driver_problems": return `${f.value} ${plural(f.value, "устройство работает", "устройства работают", "устройств работают")} с ошибкой`;
    case "driver_unsigned": return `${f.value} ${plural(f.value, "драйвер", "драйвера", "драйверов")} без цифровой подписи`;
    case "driver_updates": return `${f.value} ${plural(f.value, "обновление", "обновления", "обновлений")} драйверов`;
    default: return f.id;
  }
}

export function adviceText(a: Advice): { title: string; detail: string } {
  switch (a.id) {
    case "system_disk_full":
      return { title: "Мало места на системном диске", detail: `Занято ${percent(a.value)}. Windows и программы начинают тормозить, когда свободно меньше 10%.` };
    case "disk_full":
      return { title: "Диск почти заполнен", detail: `Один из дисков занят на ${percent(a.value)}.` };
    case "memory_high":
      return { title: "Не хватает памяти", detail: a.subject ? `Больше всего памяти занимает ${a.subject}.` : "Закройте неиспользуемые программы." };
    case "cpu_high":
      return { title: "Процессор сильно загружен", detail: a.subject ? `Больше всего нагружает ${a.subject}.` : "Проверьте запущенные программы." };
    case "junk":
      return { title: "Можно освободить место", detail: `Найдено ${bytes(a.value)} кеша и временных файлов.` };
    case "startup_many":
      return { title: "Много программ в автозагрузке", detail: `${a.value} ${plural(a.value, "программа запускается", "программы запускаются", "программ запускаются")} вместе с Windows и замедляют старт.` };
    case "uptime_long":
      return { title: "Пора перезагрузиться", detail: `Компьютер работает без перезагрузки ${duration(a.value * 86400)}. Перезагрузка освободит память и завершит обновления.` };
    case "process_memory_hog":
      return { title: `${a.subject} занимает много памяти`, detail: `${percent(a.value)} всей оперативной памяти.` };
    case "os_unsupported":
      return { title: "Windows пора обновить до новой версии", detail: `Сборка ${a.value} больше не получает исправлений безопасности. Установите новую версию через Центр обновления.` };
    case "updates_pending":
      return { title: "Установите обновления Windows", detail: `${a.value} ${plural(a.value, "обновление ждёт", "обновления ждут", "обновлений ждут")} установки. В них исправления безопасности и стабильности.` };
    case "updates_restart":
      return { title: "Перезагрузите, чтобы завершить обновление", detail: "Windows установила обновления, но применит их только после перезагрузки." };
    case "updates_stale":
      return { title: "Обновления давно не устанавливались", detail: `Последнее успешное обновление было ${a.value} ${plural(a.value, "день", "дня", "дней")} назад. Проверьте Центр обновления.` };
    case "driver_problems":
      return { title: "Устройства работают с ошибками", detail: `${a.value} ${plural(a.value, "устройство", "устройства", "устройств")} в Диспетчере устройств с ошибкой. Из-за этого бывают сбои и конфликты.` };
    case "driver_unsigned":
      return { title: "Драйверы без подписи", detail: `${a.value} ${plural(a.value, "драйвер не подписан", "драйвера не подписаны", "драйверов не подписаны")}. Такие драйверы чаще вызывают сбои.` };
    case "driver_updates":
      return { title: "Есть обновления драйверов", detail: `Windows Update предлагает ${a.value} ${plural(a.value, "проверенное обновление", "проверенных обновления", "проверенных обновлений")} драйверов.` };
    case "never_scanned":
      return { title: "Проверьте систему", detail: "Сканирование покажет, сколько места занимают кеши и временные файлы." };
    default:
      return { title: a.id, detail: "" };
  }
}

export const adviceActionLabel = {
  scan: "Сканировать",
  clean: "К очистке",
  startup: "Автозагрузка",
  processes: "Процессы",
  restart: "Понятно",
  updates: "Центр обновления",
  drivers: "Драйверы",
};

/** Device Manager problem codes (CM_PROB_*), as Windows words them. */
export function problemText(code: number): string {
  switch (code) {
    case 1: return "Устройство настроено неправильно";
    case 3: return "Драйвер повреждён или не хватает памяти";
    case 10: return "Устройство не запускается";
    case 12: return "Конфликт ресурсов с другим устройством";
    case 14: return "Нужна перезагрузка";
    case 18: return "Драйвер нужно переустановить";
    case 19: return "Ошибка настроек в реестре";
    case 21: return "Windows удаляет устройство";
    case 22: return "Отключено";
    case 24: return "Устройство отсутствует или неисправно";
    case 28: return "Драйвер не установлен";
    case 29: return "Отключено прошивкой";
    case 31: return "Windows не может загрузить драйвер";
    case 32: return "Служба драйвера отключена";
    case 37: return "Драйвер не смог запуститься";
    case 38: return "Старая копия драйвера ещё в памяти";
    case 39: return "Драйвер повреждён или отсутствует";
    case 40: return "Ошибка службы драйвера в реестре";
    case 41: return "Драйвер загружен, но устройство не найдено";
    case 43: return "Устройство сообщило о сбое";
    case 44: return "Остановлено программой или службой";
    case 45: return "Не подключено";
    case 47: return "Подготовлено к извлечению";
    case 48: return "Драйвер заблокирован из-за известных проблем";
    case 52: return "Не удалось проверить подпись драйвера";
    default: return `Ошибка, код ${code}`;
  }
}

/** What to do about a problem code. */
export function problemHint(code: number): string {
  switch (code) {
    case 12: return "Два устройства претендуют на одни ресурсы. Обновите драйвер чипсета и BIOS, отключите лишнее устройство.";
    case 14: case 21: case 38: case 47: return "Перезагрузите компьютер.";
    case 28: case 18: case 39: case 31: case 52: return "Поставьте драйвер через Центр обновления (раздел «Необязательные обновления») или с сайта производителя устройства.";
    case 48: return "Windows заблокировала этот драйвер. Нужна новая версия от производителя.";
    case 43: case 10: case 37: return "Переподключите устройство, затем переустановите драйвер. Если не помогло, проверьте устройство на другом компьютере.";
    case 29: return "Устройство выключено в BIOS/UEFI.";
    default: return "Откройте устройство в Диспетчере устройств, там Windows подскажет, что не так.";
  }
}

const driverClasses: Record<string, string> = {
  DISPLAY: "Видеоадаптер", NET: "Сеть", MEDIA: "Звук", AUDIOENDPOINT: "Аудиовыход", BLUETOOTH: "Bluetooth",
  USB: "USB", HIDCLASS: "Устройство ввода", KEYBOARD: "Клавиатура", MOUSE: "Мышь", MONITOR: "Монитор",
  DISKDRIVE: "Диск", SCSIADAPTER: "Контроллер хранения", HDC: "Контроллер дисков", SYSTEM: "Системное",
  PROCESSOR: "Процессор", PRINTER: "Принтер", CAMERA: "Камера", IMAGE: "Камера или сканер",
  BIOMETRIC: "Биометрия", FIRMWARE: "Прошивка", SOFTWAREDEVICE: "Программное", SECURITYDEVICES: "Безопасность",
  VOLUME: "Том", VOLUMESNAPSHOT: "Том", COMPUTER: "Компьютер", PORTS: "Порты", BATTERY: "Питание",
  SMARTCARDREADER: "Считыватель карт", NETSERVICE: "Сеть", NETTRANS: "Сеть", WPD: "Переносное устройство",
  EXTENSION: "Расширение драйвера", SOFTWARECOMPONENT: "Компонент драйвера", PRINTQUEUE: "Принтер",
};

export function driverClassName(c: string | null): string {
  if (!c) return "Устройство";
  return driverClasses[c.toUpperCase()] ?? c;
}

export function errorText(e: unknown): string {
  const err = e as { code?: string; message?: string };
  switch (err?.code) {
    case "elevation_required": return "Нужны права администратора.";
    case "busy": return "Дождитесь окончания текущей операции.";
    case "no_scan": return "Сначала выполните сканирование.";
    case "unsupported": return "Пока не поддерживается в этой системе.";
    default: return err?.message ?? String(e);
  }
}
