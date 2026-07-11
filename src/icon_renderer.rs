use ab_glyph::{Font, FontRef, Glyph, OutlinedGlyph, PxScale, ScaleFont, point};

use crate::constants;

static FONT_BYTES: &[u8] = include_bytes!("../assets/DejaVuSansMono-Bold.ttf");

const ORANGE: [u8; 4] = [217, 119, 87, 255];
const GRAY: [u8; 4] = [110, 110, 110, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];

#[derive(Clone, Copy)]
pub enum State {
    Ok,
    Loading,
    NotLoggedIn,
    AuthError,
    NetworkError,
}

fn state_glyph(state: State) -> &'static str {
    match state {
        State::Loading => "...",
        State::AuthError => "!",
        State::NetworkError => "x",
        // `State::Ok` only reaches here when `percent` is None (e.g. right at
        // startup); mirrors the Python dict lookup's `.get(state, "?")` fallback.
        State::NotLoggedIn | State::Ok => "?",
    }
}

pub struct RenderedIcon {
    pub width: u32,
    pub height: u32,
    /// Row-major RGBA8, non-premultiplied.
    pub rgba: Vec<u8>,
}

/// Draws tray icons using the embedded font, parsed once and reused for
/// every render instead of on every poll tick.
pub struct IconRenderer {
    font: FontRef<'static>,
}

impl IconRenderer {
    pub fn new() -> Self {
        Self {
            font: FontRef::try_from_slice(FONT_BYTES).expect("embedded font must parse"),
        }
    }

    pub fn render(&self, percent: Option<f64>, state: State) -> RenderedIcon {
        let size = constants::ICON_SIZE;
        let mut buf = vec![0u8; (size * size * 4) as usize];

        let (color, text) = match (state, percent) {
            (State::Ok, Some(p)) => (ORANGE, format!("{}%", p.round().clamp(0.0, 100.0) as i64)),
            _ => (GRAY, state_glyph(state).to_string()),
        };

        let margin = (size as f32 * 0.01).max(1.0);
        let radius = size as f32 * 0.22;
        let rect = Rect {
            x0: margin,
            y0: margin,
            x1: size as f32 - margin,
            y1: size as f32 - margin,
        };
        fill_rounded_rect(&mut buf, size, rect, radius, color);

        let max_text_width = (size as f32 - 2.0 * margin) * 0.94;
        let mut font_size = size as f32 * 0.5;

        loop {
            let text_w = self.measure_text_width(&text, font_size);
            if text_w <= max_text_width || font_size <= 8.0 {
                break;
            }
            font_size -= 2.0;
        }

        self.draw_text_centered(&mut buf, size, &text, font_size, WHITE);

        RenderedIcon {
            width: size,
            height: size,
            rgba: buf,
        }
    }

    fn layout_glyphs(&self, text: &str, px_size: f32) -> Vec<Glyph> {
        let scale = PxScale::from(px_size);
        let scaled = self.font.as_scaled(scale);
        let mut caret = 0.0f32;
        let mut glyphs = Vec::with_capacity(text.chars().count());
        for c in text.chars() {
            let id = scaled.glyph_id(c);
            glyphs.push(id.with_scale_and_position(scale, point(caret, scaled.ascent())));
            caret += scaled.h_advance(id);
        }
        glyphs
    }

    fn measure_text_width(&self, text: &str, px_size: f32) -> f32 {
        if text.is_empty() {
            return 0.0;
        }
        let scale = PxScale::from(px_size);
        let scaled = self.font.as_scaled(scale);
        text.chars()
            .map(|c| scaled.h_advance(scaled.glyph_id(c)))
            .sum()
    }

    fn draw_text_centered(
        &self,
        buf: &mut [u8],
        size: u32,
        text: &str,
        px_size: f32,
        fill: [u8; 4],
    ) {
        if text.is_empty() {
            return;
        }

        let glyphs = self.layout_glyphs(text, px_size);
        let outlines: Vec<OutlinedGlyph> = glyphs
            .into_iter()
            .filter_map(|g| self.font.outline_glyph(g))
            .collect();
        if outlines.is_empty() {
            return;
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for o in &outlines {
            let b = o.px_bounds();
            min_x = min_x.min(b.min.x);
            min_y = min_y.min(b.min.y);
            max_x = max_x.max(b.max.x);
            max_y = max_y.max(b.max.y);
        }

        let dest_x = (size as f32 - (max_x - min_x)) / 2.0 - min_x;
        let dest_y = (size as f32 - (max_y - min_y)) / 2.0 - min_y;

        for o in &outlines {
            let bounds = o.px_bounds();
            let base_x = (bounds.min.x + dest_x) as i32;
            let base_y = (bounds.min.y + dest_y) as i32;
            o.draw(|gx, gy, coverage| {
                blend_pixel(
                    buf,
                    size,
                    base_x + gx as i32,
                    base_y + gy as i32,
                    fill,
                    coverage,
                );
            });
        }
    }
}

fn blend_pixel(buf: &mut [u8], size: u32, x: i32, y: i32, color: [u8; 4], coverage: f32) {
    if x < 0 || y < 0 || x as u32 >= size || y as u32 >= size || coverage <= 0.0 {
        return;
    }
    let idx = ((y as u32 * size + x as u32) * 4) as usize;
    let a = coverage.clamp(0.0, 1.0);
    for c in 0..3 {
        let src = color[c] as f32;
        let dst = buf[idx + c] as f32;
        buf[idx + c] = (src * a + dst * (1.0 - a)).round() as u8;
    }
    buf[idx + 3] = 255;
}

#[derive(Clone, Copy)]
struct Rect {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
}

impl Rect {
    fn contains_rounded(self, x: f32, y: f32, radius: f32) -> bool {
        if x < self.x0 || x > self.x1 || y < self.y0 || y > self.y1 {
            return false;
        }
        let cx = x.clamp(self.x0 + radius, self.x1 - radius);
        let cy = y.clamp(self.y0 + radius, self.y1 - radius);
        let dx = x - cx;
        let dy = y - cy;
        dx * dx + dy * dy <= radius * radius
    }
}

fn fill_rounded_rect(buf: &mut [u8], size: u32, rect: Rect, radius: f32, color: [u8; 4]) {
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            if rect.contains_rounded(fx, fy, radius) {
                let idx = ((y * size + x) * 4) as usize;
                buf[idx..idx + 4].copy_from_slice(&color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECT: Rect = Rect {
        x0: 0.0,
        y0: 0.0,
        x1: 10.0,
        y1: 10.0,
    };

    #[test]
    fn rect_contains_center_regardless_of_radius() {
        assert!(RECT.contains_rounded(5.0, 5.0, 2.0));
    }

    #[test]
    fn rect_contains_flat_edge_midpoints() {
        // On the straight (non-corner) part of an edge, rounding never excludes a point.
        assert!(RECT.contains_rounded(5.0, 0.1, 2.0));
        assert!(RECT.contains_rounded(0.1, 5.0, 2.0));
    }

    #[test]
    fn rect_excludes_the_cut_corner() {
        // Just inside the bounding box but outside the rounded corner's radius.
        assert!(!RECT.contains_rounded(0.2, 0.2, 3.0));
    }

    #[test]
    fn rect_includes_a_point_well_inside_the_corner_arc() {
        // Comfortably inside the radius (not on its boundary, to avoid float
        // precision flakiness): distance from the arc center (3, 3) is 2.
        assert!(RECT.contains_rounded(1.0, 3.0, 3.0));
    }

    #[test]
    fn rect_excludes_points_outside_its_bounds() {
        assert!(!RECT.contains_rounded(-0.1, 5.0, 2.0));
        assert!(!RECT.contains_rounded(5.0, 10.1, 2.0));
    }

    #[test]
    fn state_glyph_shares_the_fallback_between_ok_and_not_logged_in() {
        assert_eq!(state_glyph(State::Ok), state_glyph(State::NotLoggedIn));
        assert_eq!(state_glyph(State::Loading), "...");
        assert_eq!(state_glyph(State::AuthError), "!");
        assert_eq!(state_glyph(State::NetworkError), "x");
    }

    #[test]
    fn render_icon_produces_a_fully_opaque_square_of_the_configured_size() {
        let icon = IconRenderer::new().render(Some(42.0), State::Ok);
        assert_eq!(icon.width, constants::ICON_SIZE);
        assert_eq!(icon.height, constants::ICON_SIZE);
        assert_eq!(icon.rgba.len(), (icon.width * icon.height * 4) as usize);

        // The center pixel sits inside the rounded rect, which is always fully opaque.
        let center = (icon.height / 2 * icon.width + icon.width / 2) as usize * 4;
        assert_eq!(icon.rgba[center + 3], 255);
    }

    #[test]
    fn render_icon_falls_back_to_the_glyph_when_percent_is_missing() {
        // Should not panic even though State::Ok has no dedicated glyph.
        let icon = IconRenderer::new().render(None, State::Ok);
        assert_eq!(icon.width, constants::ICON_SIZE);
    }
}
