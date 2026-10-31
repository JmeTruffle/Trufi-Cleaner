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

/// "YYYY-MM-DD" for days since 1970-01-01 (inverse of [`days_from_civil`],
/// Howard Hinnant's algorithm).
pub fn format_days(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Days since 1970-01-01 for a point in time (UTC).
pub fn days_of(t: SystemTime) -> i64 {
    match t.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => (d.as_secs() / 86_400) as i64,
        Err(e) => -((e.duration().as_secs() / 86_400) as i64) - 1,
    }
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
    fn formats_dates_back() {
        for (y, m, d) in [(1970, 1, 1), (2024, 2, 29), (2026, 10, 13), (2099, 12, 31)] {
            assert_eq!(format_days(days_from_civil(y, m, d)), format!("{y:04}-{m:02}-{d:02}"));
        }
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(86_400 * 3 + 5);
        assert_eq!(format_days(days_of(t)), "1970-01-04");
    }

    #[test]
    fn normalizes() {
        assert_eq!(normalize_name("Opera Software"), "operasoftware");
        assert_eq!(normalize_name("Яндекс.Браузер"), "яндексбраузер");
    }
}
