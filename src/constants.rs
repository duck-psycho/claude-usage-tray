use std::time::Duration;

pub const APP_NAME: &str = "Claude Usage Tray";

pub const OAUTH_TOKEN_URL_DEFAULT: &str = "https://console.anthropic.com/v1/oauth/token";
pub const OAUTH_TOKEN_URL_ENV: &str = "CLAUDE_USAGE_TRAY_OAUTH_TOKEN_URL";
pub const USAGE_API_URL: &str = "https://api.anthropic.com/api/oauth/usage";
pub const OAUTH_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
pub const USER_AGENT: &str = "claude-cli/2.1.206";

pub const HTTP_TIMEOUT: Duration = Duration::from_secs(10);
pub const POLL_INTERVAL: Duration = Duration::from_secs(180);
pub const TOKEN_EXPIRY_SAFETY_MARGIN: Duration = Duration::from_secs(60);

pub const BACKOFF_INITIAL: Duration = Duration::from_secs(30);
pub const BACKOFF_MAX: Duration = Duration::from_secs(900);

pub const ICON_SIZE: u32 = 128;

pub fn oauth_token_url() -> String {
    std::env::var(OAUTH_TOKEN_URL_ENV).unwrap_or_else(|_| OAUTH_TOKEN_URL_DEFAULT.to_string())
}

pub fn credentials_path() -> std::path::PathBuf {
    dirs::home_dir()
        .expect("home directory could not be determined")
        .join(".claude")
        .join(".credentials.json")
}
