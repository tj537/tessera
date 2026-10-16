//! Poster text: glyph outlines from a bundled font, filled with tiny-skia.
//! Outlines (instead of bitmaps) keep the letters sharp at any resolution and
//! allow a heavier weight by stroking the outline as well.

use ab_glyph::{Font as _, FontRef, OutlineCurve};
use serde::{Deserialize, Serialize};
use tiny_skia::{FillRule, PathBuilder, Pixmap, Stroke, Transform};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum Typeface {
    #[default]
    Sans,
    Mono,
}

impl Typeface {
    pub const ALL: [Typeface; 2] = [Typeface::Sans, Typeface::Mono];

    pub fn name(self) -> &'static str {
        match self {
            Typeface::Sans => "Sans",
            Typeface::Mono => "Mono",
        }
    }

    fn font(self) -> FontRef<'static> {
        let bytes = match self {
            Typeface::Sans => epaint_default_fonts::UBUNTU_LIGHT,
            Typeface::Mono => epaint_default_fonts::HACK_REGULAR,
        };
        FontRef::try_from_slice(bytes).expect("bundled font is valid")
    }
}

/// How a line of text is drawn
pub struct Style {
    pub typeface: Typeface,
    /// Font size in pixels (one em)
    pub size: f32,
    /// 0 = the font as it is, 1 = very bold
    pub weight: f32,
    /// Extra space between letters, in em
    pub tracking: f32,
    pub color: tiny_skia::Color,
}

/// Height of capital letters relative to the font size (for vertical layout)
pub const CAP_HEIGHT: f32 = 0.7;

/// Width of `text` in pixels
pub fn measure(text: &str, style: &Style) -> f32 {
    let font = style.typeface.font();
    let scale = style.size / font.units_per_em().unwrap_or(1000.0);
    let mut width = 0.0;
    let mut previous = None;
    let count = text.chars().count();
    for c in text.chars() {
        let id = font.glyph_id(c);
        if let Some(prev) = previous {
            width += font.kern_unscaled(prev, id) * scale;
        }
        width += font.h_advance_unscaled(id) * scale;
        previous = Some(id);
    }
    // Tracking between letters, not after the last one
    width + style.tracking * style.size * count.saturating_sub(1) as f32
}

/// Draws `text` with its baseline starting at (x, baseline)
pub fn draw(pixmap: &mut Pixmap, text: &str, x: f32, baseline: f32, style: &Style) {
    let font = style.typeface.font();
    let scale = style.size / font.units_per_em().unwrap_or(1000.0);
    let mut pb = PathBuilder::new();
    let mut pen = x;
    let mut previous = None;
    for c in text.chars() {
        let id = font.glyph_id(c);
        if let Some(prev) = previous {
            pen += font.kern_unscaled(prev, id) * scale;
        }
        if let Some(outline) = font.outline(id) {
            // Font units point up; the pixmap's y points down
            let map = |p: ab_glyph::Point| (pen + p.x * scale, baseline - p.y * scale);
            let mut last: Option<(f32, f32)> = None;
            for curve in &outline.curves {
                let start = match curve {
                    OutlineCurve::Line(a, _)
                    | OutlineCurve::Quad(a, _, _)
                    | OutlineCurve::Cubic(a, _, _, _) => map(*a),
                };
                // A jump means a new contour (e.g. the hole in an "o")
                if last.is_none_or(|l| (l.0 - start.0).abs() + (l.1 - start.1).abs() > 1e-3) {
                    if last.is_some() {
                        pb.close();
                    }
                    pb.move_to(start.0, start.1);
                }
                let end = match curve {
                    OutlineCurve::Line(_, b) => {
                        let b = map(*b);
                        pb.line_to(b.0, b.1);
                        b
                    }
                    OutlineCurve::Quad(_, c, b) => {
                        let (c, b) = (map(*c), map(*b));
                        pb.quad_to(c.0, c.1, b.0, b.1);
                        b
                    }
                    OutlineCurve::Cubic(_, c1, c2, b) => {
                        let (c1, c2, b) = (map(*c1), map(*c2), map(*b));
                        pb.cubic_to(c1.0, c1.1, c2.0, c2.1, b.0, b.1);
                        b
                    }
                };
                last = Some(end);
            }
            if last.is_some() {
                pb.close();
            }
        }
        pen += font.h_advance_unscaled(id) * scale + style.tracking * style.size;
        previous = Some(id);
    }

    let Some(path) = pb.finish() else {
        return;
    };
    let mut paint = tiny_skia::Paint {
        anti_alias: true,
        ..Default::default()
    };
    paint.set_color(style.color);
    pixmap.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
    if style.weight > 0.0 {
        let stroke = Stroke {
            width: style.weight * style.size * 0.07,
            line_join: tiny_skia::LineJoin::Round,
            ..Default::default()
        };
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_has_width_and_ink() {
        let style = Style {
            typeface: Typeface::Sans,
            size: 40.0,
            weight: 0.5,
            tracking: 0.1,
            color: tiny_skia::Color::BLACK,
        };
        let width = measure("TESSERA", &style);
        assert!(width > 100.0 && width < 400.0, "{width}");
        let mut pixmap = Pixmap::new(400, 80).unwrap();
        draw(&mut pixmap, "TESSERA", 10.0, 60.0, &style);
        let ink = pixmap.data().chunks(4).filter(|px| px[3] > 128).count();
        assert!(ink > 500, "only {ink} pixels drawn");
        // Wider tracking makes it wider
        let wide = Style {
            tracking: 0.4,
            ..style
        };
        assert!(measure("TESSERA", &wide) > width);
    }
}
