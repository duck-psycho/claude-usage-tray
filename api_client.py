import json
import time

import requests

import constants


class CredentialsNotFoundError(Exception):
    pass


class AuthError(Exception):
    pass


class UsageFetchError(Exception):
    pass


class ApiClient:
    """Reads Claude Code's OAuth credentials and fetches usage data.

    Refreshed access tokens are kept in memory only -- never written back to
    ~/.claude/.credentials.json, to avoid racing with the Claude Code CLI's
    own credential management.
    """

    def __init__(self):
        self._in_memory_access_token = None
        self._in_memory_expires_at_ms = None

    def _load_credentials(self) -> dict:
        if not constants.CREDENTIALS_PATH.exists():
            raise CredentialsNotFoundError(str(constants.CREDENTIALS_PATH))

        try:
            with open(constants.CREDENTIALS_PATH) as f:
                data = json.load(f)
            return data["claudeAiOauth"]
        except (json.JSONDecodeError, KeyError) as e:
            raise CredentialsNotFoundError(f"malformed credentials file: {e}")

    def _refresh_token(self, refresh_token: str) -> tuple[str, int]:
        try:
            resp = requests.post(
                constants.OAUTH_TOKEN_URL,
                json={
                    "grant_type": "refresh_token",
                    "refresh_token": refresh_token,
                    "client_id": constants.OAUTH_CLIENT_ID,
                },
                timeout=constants.HTTP_TIMEOUT_SECONDS,
            )
            resp.raise_for_status()
            data = resp.json()
            access_token = data["access_token"]
            expires_in = data.get("expires_in", 3600)
            expires_at_ms = int((time.time() + expires_in) * 1000)

            return access_token, expires_at_ms
        except (requests.RequestException, KeyError, ValueError) as e:
            raise AuthError(f"token refresh failed: {e}")

    def _get_access_token(self) -> str:
        oauth = self._load_credentials()

        if self._in_memory_access_token and self._in_memory_expires_at_ms:
            access_token = self._in_memory_access_token
            expires_at_ms = self._in_memory_expires_at_ms
        else:
            access_token = oauth.get("accessToken")
            expires_at_ms = oauth.get("expiresAt", 0)

        now_ms = time.time() * 1000
        margin_ms = constants.TOKEN_EXPIRY_SAFETY_MARGIN_SECONDS * 1000
        if not access_token or expires_at_ms - margin_ms <= now_ms:
            refresh_token = oauth.get("refreshToken")
            if not refresh_token:
                raise AuthError("no refresh token available")
            access_token, expires_at_ms = self._refresh_token(refresh_token)
            self._in_memory_access_token = access_token
            self._in_memory_expires_at_ms = expires_at_ms

        return access_token

    def fetch_usage(self) -> dict:
        access_token = self._get_access_token()

        try:
            resp = requests.get(
                constants.USAGE_API_URL,
                headers={
                    "Authorization": f"Bearer {access_token}",
                    "User-Agent": constants.USER_AGENT,
                    "Content-Type": "application/json",
                },
                timeout=constants.HTTP_TIMEOUT_SECONDS,
            )
            if resp.status_code == 401:
                # Access token might have been invalidated server-side; force one
                # refresh-and-retry before giving up.
                self._in_memory_access_token = None
                self._in_memory_expires_at_ms = None
                access_token = self._get_access_token()
                resp = requests.get(
                    constants.USAGE_API_URL,
                    headers={
                        "Authorization": f"Bearer {access_token}",
                        "User-Agent": constants.USER_AGENT,
                        "Content-Type": "application/json",
                    },
                    timeout=constants.HTTP_TIMEOUT_SECONDS,
                )
            resp.raise_for_status()
            return resp.json()
        except (requests.RequestException, ValueError) as e:
            raise UsageFetchError(str(e))
