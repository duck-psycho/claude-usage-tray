use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::api_client::{ApiClient, ApiError};
use crate::constants;
use crate::icon_renderer::State;
use crate::usage::UsageSnapshot;

pub struct PollOutcome {
    pub percent: Option<f64>,
    pub state: State,
    pub lines: Vec<String>,
}

pub struct Poller {
    client: ApiClient,
    backoff: Duration,
    last_ok: Option<(UsageSnapshot, DateTime<Utc>)>,
}

impl Poller {
    pub fn new() -> Self {
        Self {
            client: ApiClient::new(),
            backoff: constants::BACKOFF_INITIAL,
            last_ok: None,
        }
    }

    fn next_backoff(&mut self) -> Duration {
        let wait = self.backoff;
        self.backoff = (self.backoff * 2).min(constants::BACKOFF_MAX);
        wait
    }

    /// Tooltip for a failed poll: falls back to the last known-good snapshot,
    /// annotated as stale, or to just the error message if there isn't one.
    fn error_tooltip(&self, message: &str, now: DateTime<Utc>) -> Vec<String> {
        let Some((usage, at)) = &self.last_ok else {
            return vec![message.to_string()];
        };

        let age_minutes = (now - *at).num_seconds().div_euclid(60);
        let mut lines = usage.tooltip_lines(now);
        lines.push(format!("(stale, last updated {age_minutes}m ago)"));
        lines.push(message.to_string());
        lines
    }

    /// Builds the outcome/backoff pair shared by the retryable error cases
    /// (auth failures and network/HTTP failures): both fall back to the last
    /// known-good usage snapshot and back off before the next attempt.
    fn retry_outcome(
        &mut self,
        prefix: &str,
        err: ApiError,
        state: State,
        now: DateTime<Utc>,
    ) -> (PollOutcome, Duration) {
        let lines = self.error_tooltip(&format!("{prefix}: {err}, retrying..."), now);
        let wait = self.next_backoff();
        (
            PollOutcome {
                percent: None,
                state,
                lines,
            },
            wait,
        )
    }

    pub fn poll_once(&mut self) -> (PollOutcome, Duration) {
        let now = Utc::now();

        match self.client.fetch_usage() {
            Err(ApiError::CredentialsNotFound(_)) => {
                self.backoff = constants::BACKOFF_INITIAL;
                let outcome = PollOutcome {
                    percent: None,
                    state: State::NotLoggedIn,
                    lines: vec!["Not logged in \u{2014} run 'claude' to authenticate".to_string()],
                };
                (outcome, constants::POLL_INTERVAL)
            }
            Err(err @ ApiError::Auth(_)) => {
                self.retry_outcome("Auth error", err, State::AuthError, now)
            }
            Err(err @ ApiError::UsageFetch(_)) => {
                self.retry_outcome("Network error", err, State::NetworkError, now)
            }
            Ok(usage) => {
                self.backoff = constants::BACKOFF_INITIAL;

                let percent = usage.percent();
                let lines = usage.tooltip_lines(now);

                self.last_ok = Some((usage, now));

                (
                    PollOutcome {
                        percent,
                        state: State::Ok,
                        lines,
                    },
                    constants::POLL_INTERVAL,
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration as ChronoDuration;
    use serde_json::{from_value, json};

    use super::*;

    #[test]
    fn error_tooltip_is_just_the_message_without_a_prior_snapshot() {
        let poller = Poller::new();
        let now = Utc::now();
        assert_eq!(poller.error_tooltip("boom", now), vec!["boom".to_string()]);
    }

    #[test]
    fn error_tooltip_prepends_the_stale_snapshot_and_appends_the_message() {
        let mut poller = Poller::new();
        let now = Utc::now();
        let at = now - ChronoDuration::minutes(7);
        let resets_at = (now + ChronoDuration::hours(1)).to_rfc3339();
        let usage: UsageSnapshot = from_value(json!({
            "five_hour": { "utilization": 1.0, "resets_at": resets_at },
            "seven_day": { "utilization": 2.0, "resets_at": resets_at },
        }))
        .unwrap();
        poller.last_ok = Some((usage, at));

        let lines = poller.error_tooltip("boom", now);

        assert_eq!(lines.len(), 4);
        assert!(lines[2].contains("stale, last updated 7m ago"));
        assert_eq!(lines[3], "boom");
    }
}
