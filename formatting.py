from datetime import datetime, timezone


def parse_resets_at(value: str) -> datetime:
    if value.endswith("Z"):
        value = value[:-1] + "+00:00"
    dt = datetime.fromisoformat(value)
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    return dt


def format_countdown(target: datetime, now: datetime) -> str:
    delta = target - now
    total_seconds = delta.total_seconds()
    if total_seconds <= 0:
        return "resetting…"
    days, remainder = divmod(int(total_seconds), 86400)
    hours, remainder = divmod(remainder, 3600)
    minutes, _ = divmod(remainder, 60)
    if days >= 1:
        return f"{days}d {hours}h"
    return f"{hours}h {minutes}m"


def _format_window_line(label: str, window: dict, now: datetime) -> str:
    utilization = window.get("utilization")
    resets_at = window.get("resets_at")
    if utilization is None or resets_at is None:
        return f"{label}: unavailable"
    percent = round(utilization)
    try:
        target = parse_resets_at(resets_at)
        countdown = format_countdown(target, now)
        return f"{label}: {percent}% (resets in {countdown})"
    except (ValueError, TypeError):
        return f"{label}: {percent}%"


def build_tooltip(usage: dict, now: datetime) -> str:
    lines = [
        _format_window_line("5h session", usage.get("five_hour", {}), now),
        _format_window_line("7d total", usage.get("seven_day", {}), now),
    ]
    return "\n".join(lines)


def build_error_tooltip(message: str, last_ok_usage: dict | None, last_ok_at: datetime | None, now: datetime) -> str:
    if last_ok_usage is None or last_ok_at is None:
        return message
    age_minutes = int((now - last_ok_at).total_seconds() // 60)
    stale_note = f"(stale, last updated {age_minutes}m ago)"
    lines = build_tooltip(last_ok_usage, now).split("\n")
    lines.append(stale_note)
    lines.append(message)
    return "\n".join(lines)
