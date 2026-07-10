import os
from pathlib import Path

CREDENTIALS_PATH = Path.home() / ".claude" / ".credentials.json"

USAGE_API_URL = "https://api.anthropic.com/api/oauth/usage"
OAUTH_TOKEN_URL = os.environ.get(
    "CLAUDE_USAGE_TRAY_OAUTH_TOKEN_URL", "https://console.anthropic.com/v1/oauth/token"
)
OAUTH_CLIENT_ID = "9d1c250a-e61b-44d9-88ed-5944d1962f5e"
USER_AGENT = "claude-cli/2.1.206"

HTTP_TIMEOUT_SECONDS = 10
POLL_INTERVAL_SECONDS = 180
TOKEN_EXPIRY_SAFETY_MARGIN_SECONDS = 60

BACKOFF_INITIAL_SECONDS = 30
BACKOFF_MAX_SECONDS = 900

ICON_SIZE = 128

# Filenames only (no directories) -- Pillow's ImageFont.truetype() searches the
# OS's standard font directories for a matching basename on Linux/macOS/Windows.
# Falls back to ImageFont.load_default() (proportional) if none are found.
MONOSPACE_FONT_NAMES = [
    "DejaVuSansMono-Bold.ttf",
    "LiberationMono-Bold.ttf",
    "consolab.ttf",
    "Consolas Bold.ttf",
    "courbd.ttf",
    "Menlo-Bold.ttf",
]
