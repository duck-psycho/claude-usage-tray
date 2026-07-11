//! Typed model for the usage API response, with the tooltip formatting that
//! belongs to it. Fields are `Option` because the API is treated as
//! best-effort: a missing or malformed window degrades to "unavailable"
//! instead of failing the whole response.

use chrono::{DateTime, Utc};
use serde::Deserialize;

fn format_countdown(target: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let total_seconds = (target - now).num_seconds();
    if total_seconds <= 0 {
        return "resetting...".to_string();
    }
    let days = total_seconds / 86_400;
    let remainder = total_seconds % 86_400;
    let hours = remainder / 3600;
    let minutes = (remainder % 3600) / 60;
    if days >= 1 {
        format!("{days}d {hours}h")
    } else {
        format!("{hours}h {minutes}m")
    }
}

#[derive(Deserialize)]
pub struct UsageWindow {
    pub utilization: Option<f64>,
    pub resets_at: Option<String>,
}

impl UsageWindow {
    fn format_line(&self, label: &str, now: DateTime<Utc>) -> String {
        let (Some(utilization), Some(resets_at)) = (self.utilization, self.resets_at.as_deref())
        else {
            return format!("{label}: unavailable");
        };

        let percent = utilization.round() as i64;
        match DateTime::parse_from_rfc3339(resets_at) {
            Ok(target) => {
                let countdown = format_countdown(target.with_timezone(&Utc), now);
                format!("{label}: {percent}% (resets in {countdown})")
            }
            Err(_) => format!("{label}: {percent}%"),
        }
    }
}

#[derive(Default, Deserialize)]
pub struct UsageSnapshot {
    #[serde(default)]
    pub five_hour: Option<UsageWindow>,
    #[serde(default)]
    pub seven_day: Option<UsageWindow>,
}

impl UsageSnapshot {
    pub fn percent(&self) -> Option<f64> {
        self.five_hour.as_ref().and_then(|w| w.utilization)
    }

    pub fn tooltip_lines(&self, now: DateTime<Utc>) -> Vec<String> {
        vec![
            self.five_hour
                .as_ref()
                .map(|w| w.format_line("5h session", now))
                .unwrap_or_else(|| "5h session: unavailable".to_string()),
            self.seven_day
                .as_ref()
                .map(|w| w.format_line("7d total", now))
                .unwrap_or_else(|| "7d total: unavailable".to_string()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;
    use serde_json::{from_value, json};

    use super::*;

    #[test]
    fn format_countdown_reports_resetting_once_the_target_has_passed() {
        let now = Utc::now();
        assert_eq!(format_countdown(now, now), "resetting...");
        assert_eq!(
            format_countdown(now - Duration::minutes(5), now),
            "resetting..."
        );
    }

    #[test]
    fn format_countdown_uses_hours_and_minutes_under_a_day() {
        let now = Utc::now();
        let target = now + Duration::hours(2) + Duration::minutes(30);
        assert_eq!(format_countdown(target, now), "2h 30m");
    }

    #[test]
    fn format_countdown_uses_days_and_hours_at_or_over_a_day() {
        let now = Utc::now();
        let target = now + Duration::days(1) + Duration::hours(4);
        assert_eq!(format_countdown(target, now), "1d 4h");
    }

    #[test]
    fn window_format_line_reports_unavailable_when_fields_are_missing() {
        let now = Utc::now();
        let empty: UsageWindow = from_value(json!({})).unwrap();
        assert_eq!(empty.format_line("5h session", now), "5h session: unavailable");

        let missing_resets: UsageWindow = from_value(json!({ "utilization": 42.0 })).unwrap();
        assert_eq!(
            missing_resets.format_line("5h session", now),
            "5h session: unavailable"
        );
    }

    #[test]
    fn window_format_line_includes_the_countdown_when_resets_at_parses() {
        let now = Utc::now();
        let resets_at = (now + Duration::hours(1)).to_rfc3339();
        let window: UsageWindow = from_value(json!({ "utilization": 12.4, "resets_at": resets_at })).unwrap();
        assert_eq!(
            window.format_line("5h session", now),
            "5h session: 12% (resets in 1h 0m)"
        );
    }

    #[test]
    fn window_format_line_falls_back_to_bare_percent_on_unparsable_resets_at() {
        let now = Utc::now();
        let window: UsageWindow =
            from_value(json!({ "utilization": 5.0, "resets_at": "not-a-date" })).unwrap();
        assert_eq!(window.format_line("7d total", now), "7d total: 5%");
    }

    #[test]
    fn tooltip_lines_renders_both_windows() {
        let now = Utc::now();
        let resets_at = (now + Duration::hours(3)).to_rfc3339();
        let usage: UsageSnapshot = from_value(json!({
            "five_hour": { "utilization": 10.0, "resets_at": resets_at },
            "seven_day": { "utilization": 20.0, "resets_at": resets_at },
        }))
        .unwrap();
        let lines = usage.tooltip_lines(now);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("5h session: 10%"));
        assert!(lines[1].starts_with("7d total: 20%"));
    }

    #[test]
    fn tooltip_lines_reports_unavailable_for_a_missing_window() {
        let now = Utc::now();
        let usage: UsageSnapshot = from_value(json!({})).unwrap();
        let lines = usage.tooltip_lines(now);
        assert_eq!(lines, vec!["5h session: unavailable", "7d total: unavailable"]);
    }

    #[test]
    fn percent_reads_the_five_hour_window() {
        let usage: UsageSnapshot = from_value(json!({
            "five_hour": { "utilization": 37.0, "resets_at": Utc::now().to_rfc3339() },
        }))
        .unwrap();
        assert_eq!(usage.percent(), Some(37.0));
        assert_eq!(UsageSnapshot::default().percent(), None);
    }
}
