from enum import Enum

from PIL import Image, ImageDraw, ImageFont

import constants

ORANGE = (217, 119, 87, 255)
GRAY = (110, 110, 110, 255)


class State(Enum):
    OK = "ok"
    LOADING = "loading"
    NOT_LOGGED_IN = "not_logged_in"
    AUTH_ERROR = "auth_error"
    NETWORK_ERROR = "network_error"


_STATE_GLYPHS = {
    State.LOADING: "…",
    State.NOT_LOGGED_IN: "?",
    State.AUTH_ERROR: "!",
    State.NETWORK_ERROR: "×",
}

_font_cache: dict[int, ImageFont.ImageFont] = {}


def _get_font(size: int) -> ImageFont.ImageFont:
    if size in _font_cache:
        return _font_cache[size]
    for name in constants.MONOSPACE_FONT_NAMES:
        try:
            font = ImageFont.truetype(name, size)
            _font_cache[size] = font
            return font
        except OSError:
            continue
    font = ImageFont.load_default(size=size)
    _font_cache[size] = font
    return font


def render_icon(percent: int | None, state: State) -> Image.Image:
    size = constants.ICON_SIZE
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    if state == State.OK and percent is not None:
        color = ORANGE
        text = f"{min(max(round(percent), 0), 100)}%"
    else:
        color = GRAY
        text = _STATE_GLYPHS.get(state, "?")

    margin = int(size * 0.01)
    radius = int(size * 0.22)
    draw.rounded_rectangle(
        (margin, margin, size - margin, size - margin), radius=radius, fill=color
    )

    max_text_width = (size - 2 * margin) * 0.82
    font_size = int(size * 0.44)
    font = _get_font(font_size)
    bbox = draw.textbbox((0, 0), text, font=font)
    text_w = bbox[2] - bbox[0]
    text_h = bbox[3] - bbox[1]
    while text_w > max_text_width and font_size > 8:
        font_size -= 2
        font = _get_font(font_size)
        bbox = draw.textbbox((0, 0), text, font=font)
        text_w = bbox[2] - bbox[0]
        text_h = bbox[3] - bbox[1]
    text_x = (size - text_w) / 2 - bbox[0]
    text_y = (size - text_h) / 2 - bbox[1]
    draw.text((text_x, text_y), text, fill="white", font=font)

    return img
