import logging
import threading
from datetime import datetime, timezone

import pystray

import constants
import formatting
from api_client import ApiClient, AuthError, CredentialsNotFoundError, UsageFetchError
from icon_render import State, render_icon

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s")
logger = logging.getLogger("claude_usage_tray")


class TrayApp:
    def __init__(self):
        self.client = ApiClient()
        self.stop_event = threading.Event()
        self.refresh_event = threading.Event()

        self._last_render_key = None
        self._last_ok_usage = None
        self._last_ok_at = None
        self._backoff_seconds = constants.BACKOFF_INITIAL_SECONDS

        initial_image = render_icon(None, State.LOADING)
        self.icon = pystray.Icon(
            "claude_usage_tray",
            initial_image,
            "Loading Claude usage…",
            menu=pystray.Menu(self._build_menu),
        )

    def _build_menu(self):
        lines = (self.icon.title or "").split("\n")
        items = [pystray.MenuItem(line, None, enabled=False) for line in lines if line]
        items.append(pystray.Menu.SEPARATOR)
        items.append(pystray.MenuItem("Quit", self._on_quit))
        return items

    def _on_quit(self, icon, item):
        self.stop_event.set()
        self.refresh_event.set()
        icon.stop()

    def _apply(self, percent, state: State, tooltip: str):
        key = (percent, state)
        if key != self._last_render_key:
            self.icon.icon = render_icon(percent, state)
            self._last_render_key = key
        self.icon.title = tooltip
        # The GTK/AppIndicator backend builds its native menu once and does not
        # re-poll our dynamic Menu(callable) on its own -- it must be told to
        # rebuild whenever the underlying data (tooltip lines) changes.
        self.icon.update_menu()

    def _poll_once(self):
        now = datetime.now(timezone.utc)
        try:
            usage = self.client.fetch_usage()
        except CredentialsNotFoundError:
            self._apply(
                None,
                State.NOT_LOGGED_IN,
                "Not logged in — run 'claude' to authenticate",
            )
            self._backoff_seconds = constants.BACKOFF_INITIAL_SECONDS
            return constants.POLL_INTERVAL_SECONDS
        except AuthError as e:
            tooltip = formatting.build_error_tooltip(
                f"Auth error: {e}, retrying…", self._last_ok_usage, self._last_ok_at, now
            )
            self._apply(None, State.AUTH_ERROR, tooltip)
            return self._next_backoff()
        except UsageFetchError as e:
            tooltip = formatting.build_error_tooltip(
                f"Network error: {e}, retrying…", self._last_ok_usage, self._last_ok_at, now
            )
            self._apply(None, State.NETWORK_ERROR, tooltip)
            return self._next_backoff()
        except Exception:
            logger.exception("unexpected error during poll")
            tooltip = formatting.build_error_tooltip(
                "Unexpected error, retrying…", self._last_ok_usage, self._last_ok_at, now
            )
            self._apply(None, State.NETWORK_ERROR, tooltip)
            return self._next_backoff()

        self._backoff_seconds = constants.BACKOFF_INITIAL_SECONDS
        self._last_ok_usage = usage
        self._last_ok_at = now

        percent = usage.get("five_hour", {}).get("utilization")
        tooltip = formatting.build_tooltip(usage, now)
        self._apply(percent, State.OK, tooltip)
        return constants.POLL_INTERVAL_SECONDS

    def _next_backoff(self) -> float:
        wait = self._backoff_seconds
        self._backoff_seconds = min(self._backoff_seconds * 2, constants.BACKOFF_MAX_SECONDS)
        return wait

    def _poll_loop(self, icon):
        icon.visible = True
        while not self.stop_event.is_set():
            wait_seconds = self._poll_once()
            self.refresh_event.wait(timeout=wait_seconds)
            self.refresh_event.clear()

    def run(self):
        self.icon.run(setup=self._poll_loop)


def main():
    TrayApp().run()
