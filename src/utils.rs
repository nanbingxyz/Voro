/// Truncate a string to a maximum length, adding ellipsis if needed
pub fn truncate_string(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }

    if max <= 3 {
        return ".".repeat(max);
    }

    let truncated: String = s.chars().take(max - 3).collect();
    format!("{}...", truncated)
}

/// Format duration in milliseconds to a human-readable string
pub fn format_duration(ms: i64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60000 {
        format!("{:.2}s", ms as f64 / 1000.0)
    } else {
        let minutes = ms / 60000;
        let seconds = (ms % 60000) / 1000;
        format!("{}m {}s", minutes, seconds)
    }
}

/// Format a command for display with favorite indicator
#[allow(dead_code)]
pub fn format_command_name(cmd: &crate::model::Command, width: usize) -> String {
    let favorite_indicator = if cmd.favorite { "★ " } else { "  " };
    let name = truncate_string(&cmd.name, width.saturating_sub(2));
    format!("{}{}", favorite_indicator, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_string_short() {
        assert_eq!(truncate_string("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_string_exact() {
        assert_eq!(truncate_string("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_string_long() {
        assert_eq!(truncate_string("hello world", 8), "hello...");
    }

    #[test]
    fn test_truncate_string_very_short() {
        assert_eq!(truncate_string("hello", 2), "..");
        assert_eq!(truncate_string("hello", 3), "...");
    }

    #[test]
    fn test_truncate_string_empty() {
        assert_eq!(truncate_string("", 5), "");
    }

    #[test]
    fn test_truncate_string_unicode() {
        // Unicode characters should be handled correctly (counted by chars, not bytes)
        // "hello 🌍" = 7 chars, + "..." = 10 total
        assert_eq!(truncate_string("hello 🌍 world", 10), "hello 🌍...");
    }

    #[test]
    fn test_format_duration_ms() {
        assert_eq!(format_duration(500), "500ms");
        assert_eq!(format_duration(0), "0ms");
        assert_eq!(format_duration(999), "999ms");
    }

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(1000), "1.00s");
        assert_eq!(format_duration(1500), "1.50s");
        assert_eq!(format_duration(59990), "59.99s");
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(60000), "1m 0s");
        assert_eq!(format_duration(90000), "1m 30s");
        assert_eq!(format_duration(125000), "2m 5s");
    }
}
