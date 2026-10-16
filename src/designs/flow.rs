//! Flow: lines that follow a field of Perlin noise.

use super::{ParamSpec, Params};
use crate::paint::{color, stroke};
use crate::palette::Palette;
use crate::rng::{Noise, Rng};
use std::f32::consts::PI;
use tiny_skia::{PathBuilder, Pixmap};

pub const SPECS: &[ParamSpec] = &[
    ParamSpec::float("lines", "Lines", "How many lines are drawn")
        .int()
        .range(50.0, 4000.0, 1400.0),
    ParamSpec::float(
        "scale",
        "Zoom",
        "Size of the currents – low values give big, calm waves",
    )
    .range(0.3, 8.0, 1.6),
    ParamSpec::float("swirl", "Swirl", "How sharply the lines turn").range(0.5, 6.0, 2.4),
    ParamSpec::float("detail", "Turbulence", "Adds smaller eddies to the current")
        .range(0.0, 1.0, 0.3),
    ParamSpec::float("length", "Length", "How far each line travels").range(0.02, 1.5, 0.3),
    ParamSpec::float(
        "width",
        "Line width",
        "Thickness in thousandths of the design",
    )
    .range(0.3, 15.0, 2.0),
    ParamSpec::float(
        "opacity",
        "Opacity",
        "Lower values let crossing lines shine through",
    )
    .range(0.1, 1.0, 0.85),
    ParamSpec::float(
        "direction",
        "Direction",
        "Turns the whole current (degrees)",
    )
    .range(0.0, 360.0, 0.0),
];

pub fn draw(pixmap: &mut Pixmap, p: &Params, palette: &Palette, seed: u64) {
    pixmap.fill(color(palette.background, 1.0));
    let (w, h) = (pixmap.width() as f32, pixmap.height() as f32);
    // Work in units of the shorter side, so the look doesn't depend on pixels
    let s = w.min(h);
    let (wn, hn) = (w / s, h / s);

    let noise = Noise::new(seed);
    let mut rng = Rng::new(seed);
    let scale = p.get("scale");
    let swirl = p.get("swirl") * PI;
    let gain = p.get("detail") * 0.7;
    let direction = p.get("direction").to_radians();
    let width = p.get("width") / 1000.0 * s;
    let opacity = p.get("opacity");
    let step = 0.003;
    let steps = ((p.get("length") / step) as usize).max(2);

    for _ in 0..p.count("lines") {
        // Start a little outside, so the edges get lines too
        let mut x = rng.range(-0.05, wn + 0.05);
        let mut y = rng.range(-0.05, hn + 0.05);
        let rgb = *rng.pick(palette.colors);
        let line_width = width * rng.range(0.5, 1.0);

        let mut pb = PathBuilder::new();
        pb.move_to(x * s, y * s);
        for _ in 0..steps {
            let angle = noise.fbm(x * scale, y * scale, 3, gain) * swirl + direction;
            x += angle.cos() * step;
            y += angle.sin() * step;
            if !(-0.1..=wn + 0.1).contains(&x) || !(-0.1..=hn + 0.1).contains(&y) {
                break;
            }
            pb.line_to(x * s, y * s);
        }
        if let Some(path) = pb.finish() {
            stroke(pixmap, &path, rgb, opacity, line_width);
        }
    }
}
