//! Small drawing helpers on top of tiny-skia.

use crate::palette::Rgb;
use tiny_skia::{
    FillRule, LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap, Rect, Stroke, Transform,
};

pub fn color(rgb: Rgb, alpha: f32) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba8(
        rgb[0],
        rgb[1],
        rgb[2],
        (alpha.clamp(0.0, 1.0) * 255.0) as u8,
    )
}

pub fn paint(rgb: Rgb, alpha: f32) -> Paint<'static> {
    let mut paint = Paint {
        anti_alias: true,
        ..Default::default()
    };
    paint.set_color(color(rgb, alpha));
    paint
}

pub fn fill(pixmap: &mut Pixmap, path: &Path, rgb: Rgb) {
    pixmap.fill_path(
        path,
        &paint(rgb, 1.0),
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

/// Stroke with round ends and corners
pub fn stroke(pixmap: &mut Pixmap, path: &Path, rgb: Rgb, alpha: f32, width: f32) {
    stroke_with(pixmap, path, rgb, alpha, width, LineCap::Round);
}

/// Stroke that ends exactly where the path ends (nothing sticks out)
pub fn stroke_flat(pixmap: &mut Pixmap, path: &Path, rgb: Rgb, width: f32) {
    stroke_with(pixmap, path, rgb, 1.0, width, LineCap::Butt);
}

/// Stroke with any line end. Lines thinner than a pixel are drawn one pixel
/// wide and fainter instead: that looks the same, and tiny-skia's hairline
/// code trips debug assertions on very thin lines.
pub fn stroke_with(
    pixmap: &mut Pixmap,
    path: &Path,
    rgb: Rgb,
    alpha: f32,
    width: f32,
    cap: LineCap,
) {
    if width <= 0.0 || !width.is_finite() {
        return;
    }
    let (width, alpha) = if width < 1.0 {
        (1.0, alpha * width)
    } else {
        (width, alpha)
    };
    let stroke = Stroke {
        width,
        line_cap: cap,
        line_join: LineJoin::Round,
        ..Default::default()
    };
    pixmap.stroke_path(
        path,
        &paint(rgb, alpha),
        &stroke,
        Transform::identity(),
        None,
    );
}

/// Filled rectangle. Goes through a path: tiny-skia's own anti-aliased
/// `fill_rect` trips debug assertions on rectangles that stick out of the image.
pub fn fill_rect(pixmap: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, rgb: Rgb) {
    if let Some(rect) = Rect::from_xywh(x, y, w, h) {
        fill(pixmap, &PathBuilder::from_rect(rect), rgb);
    }
}

pub fn circle(cx: f32, cy: f32, r: f32) -> Option<Path> {
    PathBuilder::from_circle(cx, cy, r.max(0.01))
}

/// Appends a circular arc from angle `a0` to `a1` (radians, clockwise on
/// screen because y points down). Starts with a line from the current point,
/// or a move if the path is empty.
pub fn arc(pb: &mut PathBuilder, cx: f32, cy: f32, r: f32, a0: f32, a1: f32) {
    let start = (cx + r * a0.cos(), cy + r * a0.sin());
    if pb.is_empty() {
        pb.move_to(start.0, start.1);
    } else {
        pb.line_to(start.0, start.1);
    }
    // Cubic Béziers of at most 90° each
    let segments = ((a1 - a0).abs() / std::f32::consts::FRAC_PI_2)
        .ceil()
        .max(1.0) as usize;
    let step = (a1 - a0) / segments as f32;
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    for i in 0..segments {
        let t0 = a0 + step * i as f32;
        let t1 = t0 + step;
        let (c0, s0, c1, s1) = (t0.cos(), t0.sin(), t1.cos(), t1.sin());
        pb.cubic_to(
            cx + r * (c0 - k * s0),
            cy + r * (s0 + k * c0),
            cx + r * (c1 + k * s1),
            cy + r * (s1 - k * c1),
            cx + r * c1,
            cy + r * s1,
        );
    }
}

/// Closed polygon through `points`
pub fn polygon(points: &[(f32, f32)]) -> Option<Path> {
    let mut pb = PathBuilder::new();
    let (first, rest) = points.split_first()?;
    pb.move_to(first.0, first.1);
    for p in rest {
        pb.line_to(p.0, p.1);
    }
    pb.close();
    pb.finish()
}

/// Perceived brightness 0…1 – to pick black or white text on a colour
pub fn luminance(rgb: Rgb) -> f32 {
    (0.299 * rgb[0] as f32 + 0.587 * rgb[1] as f32 + 0.114 * rgb[2] as f32) / 255.0
}
