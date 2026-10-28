use std::time::{Duration, SystemTime};

/// Lowercases and keeps only alphanumerics: "Opera Software" -> "operasoftware".
pub fn normalize_name(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

pub fn age_of(modified: SystemTime) -> Duration {
    SystemTime::now()
        .duration_since(modified)
        .unwrap_or(Duration::ZERO)
}

/// Days since 1970-01-01 for a civil date (proleptic Gregorian).
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Parses the date part of "YYYY-MM-DD..." into days since 1970-01-01.
pub fn parse_date_days(s: &str) -> Option<i64> {
    let mut it = s.get(..10)?.split('-');
    let y = it.next()?.parse().ok()?;
    let m = it.next()?.parse().ok().filter(|m| (1..=12).contains(m))?;
    let d = it.next()?.parse().ok().filter(|d| (1..=31).contains(d))?;
    Some(days_from_civil(y, m, d))
}

pub fn today_days() -> i64 {
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs();
    (secs / 86_400) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
        assert_eq!(parse_date_days("2026-10-13"), Some(days_from_civil(2026, 10, 13)));
        assert_eq!(parse_date_days("2026-13-01"), None);
        assert_eq!(parse_date_days("garbage"), None);
    }

    #[test]
    fn normalizes() {
        assert_eq!(normalize_name("Opera Software"), "operasoftware");
        assert_eq!(normalize_name("Яндекс.Браузер"), "яндексбраузер");
    }
}
