# claude-usage-tray

System tray app showing Claude Code's 5-hour session usage as a percentage on the tray icon.
Click the icon to see the full 5h session / 7d total breakdown.

Reads the OAuth token Claude Code already stores in `~/.claude/.credentials.json` and calls
the same (undocumented) usage endpoint Claude Code itself uses. Requires being logged in via
the `claude` CLI already.

![Tray icon and menu](images/screenshot.png)

## System requirements

`pystray` needs a platform-specific tray backend. On macOS and Windows, pip pulls it in
automatically as a dependency of `pystray` itself -- no extra system install needed. On
Linux, the GTK/AppIndicator bindings are **not** pip-installable and must already be present
on the system:

- **Linux (Debian/Ubuntu)**:
  ```
  sudo apt install python3-gi python3-gi-cairo gir1.2-gtk-3.0 gir1.2-appindicator3-0.1 libayatana-appindicator3-1
  ```
  (Other distros: install the equivalent PyGObject + GTK3 + AppIndicator3 packages via your
  package manager, e.g. `python3-gobject`/`libappindicator-gtk3` on Fedora.)
- **macOS**: nothing extra -- `pip install pystray` pulls in `pyobjc-framework-Cocoa`.
- **Windows**: nothing extra -- `pip install pystray` pulls in `pywin32`.

## Setup & Run

```
./start.sh
```

This creates `.venv` (with `--system-site-packages`, needed on Linux so the venv can see the
system GTK/AppIndicator3 bindings -- see above), installs `requirements.txt` on first run,
then launches the tray app. Subsequent runs just activate the existing venv and start it.

### Manual setup

```
python3 -m venv --system-site-packages .venv
source .venv/bin/activate
pip install -r requirements.txt
python main.py
```

`--system-site-packages` is required so the venv can see the system GTK/AppIndicator3
bindings needed for the Linux tray backend (see above). On macOS/Windows this flag is
harmless but unnecessary.

## Notes

- Polls every 180 seconds by default (see `constants.py`).
- Token refresh uses a reverse-engineered, unofficial endpoint. If it stops working, the
  tray shows an auth-error state instead of crashing; you can override the refresh URL with
  the `CLAUDE_USAGE_TRAY_OAUTH_TOKEN_URL` environment variable.
- On Linux, left- and right-click are the same event (AppIndicator/StatusNotifierItem) --
  any click opens the menu.
