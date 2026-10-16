//! Ridges: stacked lines with peaks in the middle – each line hides the ones
//! behind it, like a mountain range seen from the side.

use super::{ParamSpec, Params};
use crate::paint::{color, fill, polygon, stroke};
use crate::palette::Palette;
use crate::rng::{Noise, Rng};
use tiny_skia::{PathBuilder, Pixmap};

pub const SPECS: &[ParamSpec] = &[
    ParamSpec::float("lines", "Lines", "Number of stacked lines")
        .int()
        .range(8.0, 150.0, 60.0),
    ParamSpec::float("height", "Height", "How tall the peaks grow").range(0.0, 3.0, 1.0),
    ParamSpec::float("focus", "Focus", "Width of the busy middle part").range(0.05, 1.0, 0.32),
    ParamSpec::float("roughness", "Roughness", "Number of peaks per line").range(1.0, 30.0, 9.0),
    ParamSpec::float(
        "width",
        "Line width",
        "Thickness in thousandths of the design",
    )
    .range(0.3, 10.0, 2.2),
    ParamSpec::float("padding", "Padding", "Empty space around the lines").range(0.0, 0.35, 0.14),
    ParamSpec::float("solid", "Solid", "Lines in front hide the ones behind").toggle(true),
    ParamSpec::float(
        "colorful",
        "Colourful",
        "Use every palette colour, not just the first",
    )
    .toggle(false),
];

/// Points along each line – fixed, so every resolution gets the same shape
const SAMPLES: usize = 220;

pub fn draw(pixmap: &mut Pixmap, p: &Params, palette: &Palette, seed: u64) {
    pixmap.fill(color(palette.background, 1.0));
    let (w, h) = (pixmap.width() as f32, pixmap.height() as f32);
    let s = w.min(h);
    let noise = Noise::new(seed);
    let mut rng = Rng::new(seed);

    let lines = p.count("lines").max(1);
    let pad = p.get("padding") * s;
    let (x0, x1, y0, y1) = (pad, w - pad, pad, h - pad);
    let area = (y1 - y0).max(1.0);
    // Peaks may rise several line spacings; the top line needs room for them
    let peak = p.get("height") * 7.0 * area / lines as f32;
    let headroom = peak.min(area * 0.4);
    let spacing = (area - headroom) / lines as f32;
    let focus = p.get("focus") * 0.5;
    let roughness = p.get("roughness");
    let width = p.get("width") / 1000.0 * s;

    for i in 0..lines {
        let base = y0 + headroom + (i as f32 + 0.5) * spacing;
        let center = 0.5 + rng.range(-0.08, 0.08);
        let row = i as f32 * 0.37;
        let rgb = if p.on("colorful") {
            palette.colors[i % palette.colors.len()]
        } else {
            palette.colors[0]
        };

        let points: Vec<(f32, f32)> = (0..=SAMPLES)
            .map(|j| {
                let t = j as f32 / SAMPLES as f32;
                let envelope = (-((t - center) / focus).powi(2)).exp();
                let n = noise.fbm(t * roughness, row, 3, 0.5) * 0.5 + 0.5;
                let calm = (noise.get(t * roughness * 2.5 + 50.0, row) * 0.5 + 0.5) * 0.05;
                let lift = peak * (envelope * n * n * 1.6 + calm);
                (x0 + t * (x1 - x0), base - lift)
            })
            .collect();

        if p.on("solid") {
            // Cover everything behind this line, down to its own baseline area
            let mut outline = points.clone();
            outline.push((x1, base + spacing * 0.5));
            outline.push((x0, base + spacing * 0.5));
            if let Some(path) = polygon(&outline) {
                fill(pixmap, &path, palette.background);
            }
        }
        let mut pb = PathBuilder::new();
        pb.move_to(points[0].0, points[0].1);
        for &(x, y) in &points[1..] {
            pb.line_to(x, y);
        }
        if let Some(path) = pb.finish() {
            stroke(pixmap, &path, rgb, 1.0, width);
        }
    }
}
