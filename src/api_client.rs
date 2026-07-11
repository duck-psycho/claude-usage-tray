//! Reads Claude Code's OAuth credentials and fetches usage data.
//!
//! Refreshed access tokens are kept in memory only -- never written back to
//! ~/.claude/.credentials.json, to avoid racing with the Claude Code CLI's
//! own credential management.

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::constants;
use crate::usage::UsageSnapshot;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    CredentialsNotFound(String),
    #[error("{0}")]
    Auth(String),
    #[error("{0}")]
    UsageFetch(String),
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before the Unix epoch")
        .as_millis() as i64
}

struct InMemoryToken {
    access_token: String,
    expires_at_ms: i64,
}

pub struct ApiClient {
    in_memory: Option<InMemoryToken>,
    agent: ureq::Agent,
}

impl ApiClient {
    pub fn new() -> Self {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(constants::HTTP_TIMEOUT))
            .http_status_as_error(false)
            .build()
            .into();
        Self {
            in_memory: None,
            agent,
        }
    }

    fn load_credentials() -> Result<Value, ApiError> {
        let path = constants::credentials_path();
        if !path.exists() {
            return Err(ApiError::CredentialsNotFound(path.display().to_string()));
        }

        let text = std::fs::read_to_string(&path).map_err(|e| {
            ApiError::CredentialsNotFound(format!("malformed credentials file: {e}"))
        })?;
        let data: Value = serde_json::from_str(&text).map_err(|e| {
            ApiError::CredentialsNotFound(format!("malformed credentials file: {e}"))
        })?;
        data.get("claudeAiOauth").cloned().ok_or_else(|| {
            ApiError::CredentialsNotFound("malformed credentials file: 'claudeAiOauth'".to_string())
        })
    }

    fn refresh_token(&self, refresh_token: &str) -> Result<(String, i64), ApiError> {
        let url = constants::oauth_token_url();
        let body = serde_json::json!({
            "grant_type": "refresh_token",
            "refresh_token": refresh_token,
            "client_id": constants::OAUTH_CLIENT_ID,
        });

        let mut response = self
            .agent
            .post(&url)
            .send_json(&body)
            .map_err(|e| ApiError::Auth(format!("token refresh failed: {e}")))?;

        if !response.status().is_success() {
            return Err(ApiError::Auth(format!(
                "token refresh failed: HTTP {}",
                response.status()
            )));
        }

        let data: Value = response
            .body_mut()
            .read_json()
            .map_err(|e| ApiError::Auth(format!("token refresh failed: {e}")))?;

        let access_token = data
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ApiError::Auth("token refresh failed: missing access_token".to_string())
            })?
            .to_string();
        let expires_in = data
            .get("expires_in")
            .and_then(Value::as_i64)
            .unwrap_or(3600);
        let expires_at_ms = now_ms() + expires_in * 1000;

        Ok((access_token, expires_at_ms))
    }

    fn get_access_token(&mut self) -> Result<String, ApiError> {
        let oauth = Self::load_credentials()?;

        let (mut access_token, expires_at_ms) = if let Some(tok) = &self.in_memory {
            (Some(tok.access_token.clone()), tok.expires_at_ms)
        } else {
            let access_token = oauth
                .get("accessToken")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            let expires_at_ms = oauth.get("expiresAt").and_then(Value::as_i64).unwrap_or(0);
            (access_token, expires_at_ms)
        };

        let margin_ms = constants::TOKEN_EXPIRY_SAFETY_MARGIN.as_millis() as i64;
        if access_token.is_none() || expires_at_ms - margin_ms <= now_ms() {
            let refresh_token = oauth
                .get("refreshToken")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ApiError::Auth("no refresh token available".to_string()))?;
            let (new_token, new_expiry) = self.refresh_token(refresh_token)?;
            self.in_memory = Some(InMemoryToken {
                access_token: new_token.clone(),
                expires_at_ms: new_expiry,
            });
            access_token = Some(new_token);
        }

        Ok(access_token.expect("access token must be set by now"))
    }

    fn request_usage(
        &self,
        access_token: &str,
    ) -> Result<ureq::http::Response<ureq::Body>, ureq::Error> {
        self.agent
            .get(constants::USAGE_API_URL)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("User-Agent", constants::USER_AGENT)
            .header("Content-Type", "application/json")
            .call()
    }

    pub fn fetch_usage(&mut self) -> Result<UsageSnapshot, ApiError> {
        let access_token = self.get_access_token()?;
        let mut response = self
            .request_usage(&access_token)
            .map_err(|e| ApiError::UsageFetch(e.to_string()))?;

        if response.status().as_u16() == 401 {
            // Access token might have been invalidated server-side; force one
            // refresh-and-retry before giving up.
            self.in_memory = None;
            let access_token = self.get_access_token()?;
            response = self
                .request_usage(&access_token)
                .map_err(|e| ApiError::UsageFetch(e.to_string()))?;
        }

        if !response.status().is_success() {
            return Err(ApiError::UsageFetch(format!(
                "unexpected status: HTTP {}",
                response.status()
            )));
        }

        response
            .body_mut()
            .read_json::<UsageSnapshot>()
            .map_err(|e| ApiError::UsageFetch(e.to_string()))
    }
}
