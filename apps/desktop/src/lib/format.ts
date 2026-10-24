const UNITS = ["Б", "КБ", "МБ", "ГБ", "ТБ"];

export function bytes(n: number, digits = 1): string {
  if (!Number.isFinite(n) || n <= 0) return "0 Б";
  const i = Math.min(UNITS.length - 1, Math.floor(Math.log(n) / Math.log(1024)));
  const v = n / Math.pow(1024, i);
  const d = i === 0 || v >= 100 ? 0 : digits;
  return `${v.toLocaleString("ru-RU", { maximumFractionDigits: d, minimumFractionDigits: 0 })} ${UNITS[i]}`;
}

export function rate(bps: number): string {
  return `${bytes(bps)}/с`;
}

export function percent(v: number, digits = 0): string {
  return `${v.toLocaleString("ru-RU", { maximumFractionDigits: digits })}%`;
}

export function plural(n: number, one: string, few: string, many: string): string {
  const m10 = n % 10, m100 = n % 100;
  if (m10 === 1 && m100 !== 11) return one;
  if (m10 >= 2 && m10 <= 4 && (m100 < 12 || m100 > 14)) return few;
  return many;
}

export function duration(secs: number): string {
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (d > 0) return `${d} ${plural(d, "день", "дня", "дней")} ${h} ч`;
  if (h > 0) return `${h} ч ${m} мин`;
  return `${m} мин`;
}

export function count(n: number): string {
  return n.toLocaleString("ru-RU");
}
