//! Bauhaus: a grid of square tiles, each with a colour and one or more
//! simple shapes – quarter circles, half circles, rings, triangles, stripes.

use super::{ParamSpec, Params};
use crate::paint::{arc, circle, color, fill, fill_rect, polygon, stroke_flat};
use crate::palette::{Palette, Rgb};
use crate::rng::Rng;
use std::f32::consts::{FRAC_PI_2, PI};
use tiny_skia::{PathBuilder, Pixmap};

pub const SPECS: &[ParamSpec] = &[
    ParamSpec::float(
        "columns",
        "Columns",
        "Tiles across – rows follow from the format",
    )
    .int()
    .range(1.0, 16.0, 5.0),
    ParamSpec::float("layers", "Layers", "Shapes stacked on each tile")
        .int()
        .range(1.0, 3.0, 2.0),
    ParamSpec::float(
        "curves",
        "Curves",
        "Share of round shapes versus angular ones",
    )
    .range(0.0, 1.0, 0.7),
    ParamSpec::float(
        "plain",
        "Plain tiles",
        "Share of tiles in the background colour",
    )
    .range(0.0, 1.0, 0.35),
    ParamSpec::float("calm", "Calm", "Share of tiles without any shape").range(0.0, 0.9, 0.1),
    ParamSpec::float("gutter", "Gutter", "Gap between the tiles").range(0.0, 0.3, 0.0),
];

pub fn draw(pixmap: &mut Pixmap, p: &Params, palette: &Palette, seed: u64) {
    pixmap.fill(color(palette.background, 1.0));
    let (w, h) = (pixmap.width() as f32, pixmap.height() as f32);
    let mut rng = Rng::new(seed);

    let columns = p.count("columns").max(1);
    let tile = w / columns as f32;
    let rows = (h / tile).ceil() as usize;
    // Centre the rows; the outer ones may be cut off
    let top = (h - rows as f32 * tile) / 2.0;
    let gap = p.get("gutter") * tile * 0.5;
    let size = tile - 2.0 * gap;
    let all: Vec<Rgb> = std::iter::once(palette.background)
        .chain(palette.colors.iter().copied())
        .collect();

    for row in 0..rows {
        for col in 0..columns {
            let x = col as f32 * tile + gap;
            let y = top + row as f32 * tile + gap;
            let ground = if rng.chance(p.get("plain")) {
                palette.background
            } else {
                *rng.pick(palette.colors)
            };
            fill_rect(pixmap, x, y, size, size, ground);
            if rng.chance(p.get("calm")) {
                continue;
            }
            let mut previous = ground;
            for layer in 0..p.count("layers").max(1) {
                let rgb = other_color(&mut rng, &all, previous);
                let round = rng.chance(p.get("curves"));
                // Upper layers sometimes sit smaller in the middle
                let (sx, sy, ss) = if layer > 0 && rng.chance(0.5) {
                    let inset = size * 0.22;
                    (x + inset, y + inset, size - 2.0 * inset)
                } else {
                    (x, y, size)
                };
                shape(pixmap, &mut rng, sx, sy, ss, rgb, round);
                previous = rgb;
            }
        }
    }
}

/// A random colour that differs from `not` (when the palette allows it)
fn other_color(rng: &mut Rng, all: &[Rgb], not: Rgb) -> Rgb {
    let choices: Vec<Rgb> = all.iter().copied().filter(|&c| c != not).collect();
    if choices.is_empty() {
        not
    } else {
        *rng.pick(&choices)
    }
}

/// One shape in the square (x, y, t)
fn shape(pixmap: &mut Pixmap, rng: &mut Rng, x: f32, y: f32, t: f32, rgb: Rgb, round: bool) {
    // Which corner / side the shape leans on: 0 = top left, then clockwise
    let turn = rng.below(4);
    let corners = [(x, y), (x + t, y), (x + t, y + t), (x, y + t)];
    let kind = rng.below(if round { 5 } else { 4 });
    if round {
        match kind {
            // Quarter circle from a corner, filling the tile
            0 => {
                let (cx, cy) = corners[turn];
                let start = turn as f32 * FRAC_PI_2;
                let mut pb = PathBuilder::new();
                pb.move_to(cx, cy);
                arc(&mut pb, cx, cy, t, start, start + FRAC_PI_2);
                pb.close();
                if let Some(path) = pb.finish() {
                    fill(pixmap, &path, rgb);
                }
            }
            // Half circle standing on a side
            1 => {
                let sides = [
                    (x + t / 2.0, y),
                    (x + t, y + t / 2.0),
                    (x + t / 2.0, y + t),
                    (x, y + t / 2.0),
                ];
                let (cx, cy) = sides[turn];
                let start = turn as f32 * FRAC_PI_2;
                let mut pb = PathBuilder::new();
                arc(&mut pb, cx, cy, t / 2.0, start, start + PI);
                pb.close();
                if let Some(path) = pb.finish() {
                    fill(pixmap, &path, rgb);
                }
            }
            // Full circle
            2 => {
                let r = t * if rng.chance(0.5) { 0.5 } else { 0.32 };
                if let Some(path) = circle(x + t / 2.0, y + t / 2.0, r) {
                    fill(pixmap, &path, rgb);
                }
            }
            // Ring
            3 => {
                if let Some(path) = circle(x + t / 2.0, y + t / 2.0, t * 0.34) {
                    stroke_flat(pixmap, &path, rgb, t * 0.16);
                }
            }
            // Two quarter arcs in opposite corners (Truchet tile)
            _ => {
                for k in [turn % 2, turn % 2 + 2] {
                    let (cx, cy) = corners[k];
                    let start = k as f32 * FRAC_PI_2;
                    let mut pb = PathBuilder::new();
                    arc(&mut pb, cx, cy, t / 2.0, start, start + FRAC_PI_2);
                    if let Some(path) = pb.finish() {
                        stroke_flat(pixmap, &path, rgb, t * 0.16);
                    }
                }
            }
        }
    } else {
        match kind {
            // Half the tile, cut along the diagonal
            0 => {
                let pts = [
                    corners[turn],
                    corners[(turn + 1) % 4],
                    corners[(turn + 2) % 4],
                ];
                if let Some(path) = polygon(&pts) {
                    fill(pixmap, &path, rgb);
                }
            }
            // Triangle pointing from one side to the middle of the opposite one
            1 => {
                let a = corners[turn];
                let b = corners[(turn + 1) % 4];
                let c = corners[(turn + 2) % 4];
                let d = corners[(turn + 3) % 4];
                let tip = ((c.0 + d.0) / 2.0, (c.1 + d.1) / 2.0);
                if let Some(path) = polygon(&[a, b, tip]) {
                    fill(pixmap, &path, rgb);
                }
            }
            // Diamond
            2 => {
                let (cx, cy, r) = (x + t / 2.0, y + t / 2.0, t * 0.42);
                if let Some(path) =
                    polygon(&[(cx, cy - r), (cx + r, cy), (cx, cy + r), (cx - r, cy)])
                {
                    fill(pixmap, &path, rgb);
                }
            }
            // Three stripes
            _ => {
                let bar = t / 7.0;
                for k in 0..3 {
                    let offset = bar * (1.0 + 2.0 * k as f32);
                    if turn.is_multiple_of(2) {
                        fill_rect(pixmap, x, y + offset, t, bar, rgb);
                    } else {
                        fill_rect(pixmap, x + offset, y, bar, t, rgb);
                    }
                }
            }
        }
    }
}
