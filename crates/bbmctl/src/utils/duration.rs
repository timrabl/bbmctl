use std::time::Duration;

use anyhow::{bail, Result};

/// Parse a human-readable duration string like "30m", "1h", "2h30m", "45s", "1d".
pub fn parse_duration(s: &str) -> Result<Duration> {
    let mut total_secs: u64 = 0;
    let mut current_num = String::new();

    for c in s.chars() {
        if c.is_ascii_digit() {
            current_num.push(c);
        } else {
            if current_num.is_empty() {
                bail!("invalid duration '{s}': expected a number before '{c}'");
            }
            let n: u64 = current_num.parse()?;
            current_num.clear();

            match c {
                's' => total_secs += n,
                'm' => total_secs += n * 60,
                'h' => total_secs += n * 3600,
                'd' => total_secs += n * 86400,
                _ => bail!("invalid duration '{s}': unknown unit '{c}' (use s, m, h, d)"),
            }
        }
    }

    if !current_num.is_empty() {
        bail!("invalid duration '{s}': number without unit (use s, m, h, d)");
    }

    if total_secs == 0 {
        bail!("invalid duration '{s}': duration must be greater than zero");
    }

    Ok(Duration::from_secs(total_secs))
}

/// Format a Duration as a human-readable string.
pub fn format_duration(d: Duration) -> String {
    let total = d.as_secs();
    if total == 0 {
        return "0s".to_string();
    }

    let days = total / 86400;
    let hours = (total % 86400) / 3600;
    let mins = (total % 3600) / 60;
    let secs = total % 60;

    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{days}d"));
    }
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    if mins > 0 {
        parts.push(format!("{mins}m"));
    }
    if secs > 0 {
        parts.push(format!("{secs}s"));
    }
    parts.join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_seconds() {
        assert_eq!(parse_duration("45s").unwrap(), Duration::from_secs(45));
    }

    #[test]
    fn parse_minutes() {
        assert_eq!(parse_duration("30m").unwrap(), Duration::from_secs(1800));
    }

    #[test]
    fn parse_hours() {
        assert_eq!(parse_duration("2h").unwrap(), Duration::from_secs(7200));
    }

    #[test]
    fn parse_days() {
        assert_eq!(parse_duration("1d").unwrap(), Duration::from_secs(86400));
    }

    #[test]
    fn parse_combined() {
        assert_eq!(parse_duration("1h30m").unwrap(), Duration::from_secs(5400));
        assert_eq!(
            parse_duration("2h30m45s").unwrap(),
            Duration::from_secs(9045)
        );
    }

    #[test]
    fn parse_invalid() {
        assert!(parse_duration("").is_err());
        assert!(parse_duration("30").is_err());
        assert!(parse_duration("abc").is_err());
        assert!(parse_duration("0m").is_err());
        assert!(parse_duration("30x").is_err());
    }

    #[test]
    fn format_roundtrip() {
        assert_eq!(format_duration(Duration::from_secs(45)), "45s");
        assert_eq!(format_duration(Duration::from_secs(1800)), "30m");
        assert_eq!(format_duration(Duration::from_secs(5400)), "1h30m");
        assert_eq!(format_duration(Duration::from_secs(90061)), "1d1h1m1s");
    }
}
