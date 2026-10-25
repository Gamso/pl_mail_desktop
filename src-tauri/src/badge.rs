use std::sync::{
    OnceLock,
    atomic::{AtomicU64, Ordering},
};

use ab_glyph::{Font, FontRef, PxScale, ScaleFont, point};
use tauri::{AppHandle, Manager, image::Image};

/// The latest unread count, painted again whenever the mail window's taskbar button may be new.
static UNREAD: AtomicU64 = AtomicU64::new(0);

// 3x5 digits for machines without Segoe UI, one row per byte, high bit on the left.
const BITMAP: [[u8; 5]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111],
    [0b010, 0b110, 0b010, 0b010, 0b111],
    [0b111, 0b001, 0b111, 0b100, 0b111],
    [0b111, 0b001, 0b111, 0b001, 0b111],
    [0b101, 0b101, 0b111, 0b001, 0b001],
    [0b111, 0b100, 0b111, 0b001, 0b111],
    [0b111, 0b100, 0b111, 0b101, 0b111],
    [0b111, 0b001, 0b001, 0b001, 0b001],
    [0b111, 0b101, 0b111, 0b101, 0b111],
    [0b111, 0b101, 0b111, 0b001, 0b111],
];

fn font_bytes() -> Option<&'static [u8]> {
    static BYTES: OnceLock<Option<Vec<u8>>> = OnceLock::new();
    BYTES
        .get_or_init(|| std::fs::read(r"C:\Windows\Fonts\segoeuib.ttf").ok())
        .as_deref()
}

struct Canvas<'a> {
    buf: &'a mut [u8],
    w: usize,
    h: usize,
}

impl Canvas<'_> {
    /// Blends white over the pixel with the given coverage, keeping its alpha.
    fn white(&mut self, x: i32, y: i32, cov: f32) {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h {
            return;
        }
        let i = (y as usize * self.w + x as usize) * 4;
        for c in 0..3 {
            let old = self.buf[i + c] as f32;
            self.buf[i + c] = (old * (1.0 - cov) + 255.0 * cov) as u8;
        }
    }
}

fn label(n: u64) -> String {
    if n > 99 {
        "99+".to_string()
    } else {
        n.to_string()
    }
}

fn draw_text(cv: &mut Canvas, cx: f32, cy: f32, r: f32, text: &str) -> bool {
    let Some(font) = font_bytes().and_then(|b| FontRef::try_from_slice(b).ok()) else {
        return false;
    };
    let width = |size: f32| {
        let scaled = font.as_scaled(PxScale::from(size));
        text.chars()
            .map(|c| scaled.h_advance(font.glyph_id(c)))
            .sum::<f32>()
    };
    // A bold, Thunderbird-sized number; narrower counts get shrunk to stay inside the disc.
    let mut size = r * 2.2;
    let max_w = r * 1.65;
    if width(size) > max_w {
        size *= max_w / width(size);
    }

    // Lay the glyphs out at the origin, measure what was really drawn, then shift the
    // whole run so its ink is centred on the disc: font metrics alone leave it low.
    let scaled = font.as_scaled(PxScale::from(size));
    let layout = |ox: f32, oy: f32| {
        let mut x = ox;
        let mut glyphs = vec![];
        for c in text.chars() {
            let id = font.glyph_id(c);
            glyphs.push(id.with_scale_and_position(PxScale::from(size), point(x, oy)));
            x += scaled.h_advance(id);
        }
        glyphs
    };
    let bounds = |glyphs: Vec<ab_glyph::Glyph>| {
        glyphs
            .into_iter()
            .filter_map(|g| font.outline_glyph(g))
            .map(|o| (o.px_bounds(), o))
            .collect::<Vec<_>>()
    };
    let probe = bounds(layout(0.0, 0.0));
    if probe.is_empty() {
        return true;
    }
    let min_x = probe.iter().map(|(b, _)| b.min.x).fold(f32::MAX, f32::min);
    let max_x = probe.iter().map(|(b, _)| b.max.x).fold(f32::MIN, f32::max);
    let min_y = probe.iter().map(|(b, _)| b.min.y).fold(f32::MAX, f32::min);
    let max_y = probe.iter().map(|(b, _)| b.max.y).fold(f32::MIN, f32::max);
    let ox = cx - (min_x + max_x) / 2.0;
    let oy = cy - (min_y + max_y) / 2.0;

    for (b, outline) in bounds(layout(ox, oy)) {
        outline.draw(|gx, gy, cov| {
            cv.white(b.min.x as i32 + gx as i32, b.min.y as i32 + gy as i32, cov)
        });
    }
    true
}

fn draw_bitmap(cv: &mut Canvas, cx: f32, cy: f32, r: f32, text: &str) {
    let digits: Vec<usize> = text
        .bytes()
        .filter(u8::is_ascii_digit)
        .map(|b| (b - b'0') as usize)
        .collect();
    let k = digits.len().max(1);
    let s = ((1.6 * r) / (4 * k - 1) as f32)
        .min(1.45 * r / 5.0)
        .floor()
        .max(1.0) as i32;
    let x0 = (cx - ((4 * k - 1) as i32 * s) as f32 / 2.0).round() as i32;
    let y0 = (cy - (5 * s) as f32 / 2.0).round() as i32;
    for (d, digit) in digits.iter().enumerate() {
        for (row, bits) in BITMAP[*digit].iter().enumerate() {
            for col in 0..3 {
                if bits >> (2 - col) & 1 == 1 {
                    for dy in 0..s {
                        for dx in 0..s {
                            cv.white(
                                x0 + d as i32 * 4 * s + col * s + dx,
                                y0 + row as i32 * s + dy,
                                1.0,
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Where the disc sits.
#[derive(Clone, Copy)]
struct Disc {
    cx: f32,
    cy: f32,
    r: f32,
}

/// A red disc with the count, anti-aliased.
fn draw(buf: &mut [u8], w: usize, h: usize, disc: Disc, n: u64) {
    let Disc { cx, cy, r } = disc;
    let cover = |d: f32, edge: f32| (edge - d + 0.5).clamp(0.0, 1.0);
    for y in 0..h {
        for x in 0..w {
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            let d = (dx * dx + dy * dy).sqrt();
            let outer = cover(d, r);
            if outer <= 0.0 {
                continue;
            }
            let i = (y * w + x) * 4;
            buf[i..i + 4].copy_from_slice(&[226, 40, 70, (outer * 255.0) as u8]);
        }
    }
    let text = label(n);
    let mut cv = Canvas { buf, w, h };
    if !draw_text(&mut cv, cx, cy, r, &text) {
        draw_bitmap(&mut cv, cx, cy, r, &text);
    }
}

/// Records the count and paints it.
pub fn set(app: &AppHandle, unread: u64) {
    UNREAD.store(unread, Ordering::Relaxed);
    apply(app);
}

/// Paints the last known count on the mail window's taskbar button.
///
/// Hiding a window removes its taskbar button and showing it creates a new one, which has no
/// overlay: so this runs every time the count is polled, and whenever the window is created,
/// shown or focused, not only when the number changes.
pub fn apply(app: &AppHandle) {
    let unread = UNREAD.load(Ordering::Relaxed);
    if let Some(win) = app.get_webview_window("web") {
        let overlay = (unread > 0).then(|| {
            let mut rgba = vec![0u8; 64 * 64 * 4];
            draw(
                &mut rgba,
                64,
                64,
                Disc {
                    cx: 32.0,
                    cy: 32.0,
                    r: 32.0,
                },
                unread,
            );
            Image::new_owned(rgba, 64, 64)
        });
        let _ = win.set_overlay_icon(overlay);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bounding box of the white ink inside a rendered badge.
    fn ink_box(buf: &[u8], size: usize) -> Option<(usize, usize, usize, usize)> {
        let (mut x0, mut y0, mut x1, mut y1) = (size, size, 0, 0);
        for y in 0..size {
            for x in 0..size {
                let px = &buf[(y * size + x) * 4..][..4];
                if px[3] > 200 && px[0] > 230 && px[1] > 230 && px[2] > 230 {
                    (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
                }
            }
        }
        (x1 >= x0 && y1 >= y0).then_some((x0, y0, x1, y1))
    }

    #[test]
    fn the_count_is_centred_in_the_disc() {
        if font_bytes().is_none() {
            return; // no Segoe UI here: the bitmap fallback is not meant to be pixel-perfect
        }
        for n in [2u64, 7, 12, 99] {
            let mut buf = vec![0u8; 64 * 64 * 4];
            draw(
                &mut buf,
                64,
                64,
                Disc {
                    cx: 32.0,
                    cy: 32.0,
                    r: 32.0,
                },
                n,
            );
            let (x0, y0, x1, y1) = ink_box(&buf, 64).expect("some ink");
            let (cx, cy) = ((x0 + x1) as f32 / 2.0, (y0 + y1) as f32 / 2.0);
            assert!((cx - 32.0).abs() <= 2.0, "{n}: horizontal centre {cx}");
            assert!((cy - 32.0).abs() <= 2.0, "{n}: vertical centre {cy}");
        }
    }

    #[test]
    fn large_counts_are_capped() {
        assert_eq!(label(7), "7");
        assert_eq!(label(99), "99");
        assert_eq!(label(100), "99+");
    }

    #[test]
    fn the_disc_is_transparent_outside_and_red_inside() {
        let mut buf = vec![0u8; 64 * 64 * 4];
        draw(
            &mut buf,
            64,
            64,
            Disc {
                cx: 32.0,
                cy: 32.0,
                r: 32.0,
            },
            5,
        );
        assert_eq!(buf[3], 0, "corner stays transparent");
        let edge = (32 * 64 + 6) * 4;
        assert_eq!(buf[edge + 3], 255, "disc is opaque");
        assert!(buf[edge] > 200 && buf[edge + 1] < 80, "disc is red");
    }
}
