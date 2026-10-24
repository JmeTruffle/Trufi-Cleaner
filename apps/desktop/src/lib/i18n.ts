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
};

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
