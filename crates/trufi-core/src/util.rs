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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes() {
        assert_eq!(normalize_name("Opera Software"), "operasoftware");
        assert_eq!(normalize_name("Яндекс.Браузер"), "яндексбраузер");
    }
}
