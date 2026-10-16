//! The poster: format, layout, text and design settings, and `render`, which
//! turns them into pixels.

use crate::designs::{Design, Values};
use crate::paint::{self, luminance};
use crate::palette::{self, Rgb};
use crate::text::{self, Style, Typeface, CAP_HEIGHT};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tiny_skia::{FillRule, Mask, PathBuilder, Pixmap, PixmapPaint, Rect, Transform};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum PaperSize {
    A5,
    #[default]
    A4,
    A3,
    A2,
    Square,
    /// 4:5, the portrait format of social media posts
    Social,
}

impl PaperSize {
    pub const ALL: [PaperSize; 6] = [
        PaperSize::A5,
        PaperSize::A4,
        PaperSize::A3,
        PaperSize::A2,
        PaperSize::Square,
        PaperSize::Social,
    ];

    pub fn name(self) -> &'static str {
        match self {
            PaperSize::A5 => "A5",
            PaperSize::A4 => "A4",
            PaperSize::A3 => "A3",
            PaperSize::A2 => "A2",
            PaperSize::Square => "Square",
            PaperSize::Social => "4 : 5",
        }
    }

    /// Width and height in millimetres, portrait
    pub fn mm(self) -> (f32, f32) {
        match self {
            PaperSize::A5 => (148.0, 210.0),
            PaperSize::A4 => (210.0, 297.0),
            PaperSize::A3 => (297.0, 420.0),
            PaperSize::A2 => (420.0, 594.0),
            PaperSize::Square => (300.0, 300.0),
            PaperSize::Social => (240.0, 300.0),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum Layout {
    /// The design covers the whole page
    FullPage,
    /// The design in a rectangle with a margin around it
    Frame,
    /// The design in a circle, the rest is paper
    #[default]
    Circle,
}

impl Layout {
    pub const ALL: [Layout; 3] = [Layout::FullPage, Layout::Frame, Layout::Circle];

    pub fn name(self) -> &'static str {
        match self {
            Layout::FullPage => "Full page",
            Layout::Frame => "Frame",
            Layout::Circle => "Circle",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum PaperColor {
    #[default]
    White,
    Cream,
    Black,
}

impl PaperColor {
    pub const ALL: [PaperColor; 3] = [PaperColor::White, PaperColor::Cream, PaperColor::Black];

    pub fn name(self) -> &'static str {
        match self {
            PaperColor::White => "White",
            PaperColor::Cream => "Cream",
            PaperColor::Black => "Black",
        }
    }

    pub fn rgb(self) -> Rgb {
        match self {
            PaperColor::White => [255, 255, 255],
            PaperColor::Cream => [245, 240, 228],
            PaperColor::Black => [12, 12, 12],
        }
    }

    /// Text and outline colour that reads well on this paper
    pub fn ink(self) -> Rgb {
        if luminance(self.rgb()) > 0.5 {
            [20, 20, 20]
        } else {
            [242, 242, 242]
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum Align {
    #[default]
    Center,
    Left,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct TextSettings {
    pub show: bool,
    pub title: String,
    pub subtitle: String,
    pub typeface: Typeface,
    /// Title size in percent of the poster's shorter side
    pub size: f32,
    /// Subtitle size relative to the title
    pub subtitle_size: f32,
    /// 0 = light … 1 = bold
    pub weight: f32,
    /// Letter spacing in em
    pub tracking: f32,
    pub uppercase: bool,
    pub align: Align,
}

impl Default for TextSettings {
    fn default() -> Self {
        Self {
            show: true,
            title: "Tessera".into(),
            subtitle: "Generative series · No. 001".into(),
            typeface: Typeface::Sans,
            size: 7.0,
            subtitle_size: 0.3,
            weight: 0.35,
            tracking: 0.2,
            uppercase: true,
            align: Align::Center,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Poster {
    pub design: Design,
    pub seed: u64,
    /// Index into `palette::PALETTES`
    pub palette: usize,
    /// Parameter values per design (by `Design::id`), kept when switching
    pub params: BTreeMap<String, Values>,

    pub paper: PaperSize,
    pub landscape: bool,
    pub paper_color: PaperColor,
    pub layout: Layout,
    /// Margin in percent of the shorter side
    pub margin: f32,
    /// Circle diameter in percent of the space it can take
    pub circle_size: f32,
    /// Outline around the circle or frame, in thousandths of the shorter side
    pub outline: f32,
    pub text: TextSettings,
}

impl Default for Poster {
    fn default() -> Self {
        Self {
            design: Design::Flow,
            seed: 1,
            palette: 0,
            params: BTreeMap::new(),
            paper: PaperSize::A4,
            landscape: false,
            paper_color: PaperColor::White,
            layout: Layout::Circle,
            margin: 9.0,
            circle_size: 92.0,
            outline: 0.0,
            text: TextSettings::default(),
        }
    }
}

static NO_VALUES: Values = BTreeMap::new();

impl Poster {
    /// Width and height in millimetres, as oriented
    pub fn size_mm(&self) -> (f32, f32) {
        let (w, h) = self.paper.mm();
        if self.landscape {
            (h, w)
        } else {
            (w, h)
        }
    }

    /// Height divided by width
    pub fn aspect(&self) -> f32 {
        let (w, h) = self.size_mm();
        h / w
    }

    /// Pixel size when printed at `dpi`
    pub fn pixels_at_dpi(&self, dpi: f32) -> (u32, u32) {
        let (w, h) = self.size_mm();
        let px = |mm: f32| (mm / 25.4 * dpi).round().max(1.0) as u32;
        (px(w), px(h))
    }

    pub fn values(&self) -> &Values {
        self.params.get(self.design.id()).unwrap_or(&NO_VALUES)
    }

    pub fn values_mut(&mut self) -> &mut Values {
        self.params.entry(self.design.id().to_string()).or_default()
    }

    /// Current value of a parameter of the current design
    pub fn value(&self, key: &str) -> f32 {
        let spec = self.design.specs().iter().find(|s| s.key == key);
        let stored = self.values().get(key).copied();
        match spec {
            Some(spec) => spec.clean(stored.unwrap_or(spec.default)),
            None => stored.unwrap_or(0.0),
        }
    }
}

/// Where things go on the page, in pixels
struct Geometry {
    /// The design's rectangle
    art: Rect,
    /// Set in circle layout: centre and radius
    circle: Option<(f32, f32, f32)>,
    /// Top of the text block
    text_top: f32,
    /// Left edge for left-aligned text, and the width text may take
    text_left: f32,
    text_width: f32,
}

/// Pixel sizes of the two text lines (0 when the line is empty)
fn text_sizes(poster: &Poster, unit: f32) -> (f32, f32) {
    let t = &poster.text;
    if !t.show {
        return (0.0, 0.0);
    }
    let title = t.size / 100.0 * unit;
    (
        if t.title.trim().is_empty() {
            0.0
        } else {
            title
        },
        if t.subtitle.trim().is_empty() {
            0.0
        } else {
            title * t.subtitle_size
        },
    )
}

fn text_block_height(title: f32, subtitle: f32) -> f32 {
    let gap = if title > 0.0 && subtitle > 0.0 {
        title * 0.45
    } else {
        0.0
    };
    title * CAP_HEIGHT + gap + subtitle * CAP_HEIGHT
}

fn geometry(poster: &Poster, w: f32, h: f32) -> Geometry {
    let unit = w.min(h);
    let margin = poster.margin.clamp(0.0, 30.0) / 100.0 * unit;
    let (title, subtitle) = text_sizes(poster, unit);
    let block = text_block_height(title, subtitle);
    // Breathing room above and below the text
    let pad = margin.max(0.05 * unit);
    let zone = if block > 0.0 { block + 2.0 * pad } else { 0.0 };

    let (art, circle) = match poster.layout {
        Layout::FullPage => (Rect::from_xywh(0.0, 0.0, w, h - zone), None),
        Layout::Frame | Layout::Circle => {
            let bottom = h - zone.max(margin);
            let area = Rect::from_ltrb(margin, margin, w - margin, bottom);
            if poster.layout == Layout::Frame {
                (area, None)
            } else {
                let area = area.unwrap_or(Rect::from_xywh(0.0, 0.0, w, h).unwrap());
                let r = poster.circle_size.clamp(10.0, 100.0) / 100.0
                    * area.width().min(area.height())
                    / 2.0;
                let (cx, cy) = (
                    area.x() + area.width() / 2.0,
                    area.y() + area.height() / 2.0,
                );
                (
                    Rect::from_xywh(cx - r, cy - r, 2.0 * r, 2.0 * r),
                    Some((cx, cy, r)),
                )
            }
        }
    };
    Geometry {
        art: art.unwrap_or(Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap()),
        circle,
        text_top: h - zone + pad,
        text_left: pad,
        text_width: w - 2.0 * pad,
    }
}

/// Draws the poster `width` pixels wide (the height follows from the format)
pub fn render(poster: &Poster, width: u32) -> Pixmap {
    let height = (width as f32 * poster.aspect()).round() as u32;
    render_sized(poster, width, height)
}

/// Draws the poster at exactly `width` × `height` pixels
pub fn render_sized(poster: &Poster, width: u32, height: u32) -> Pixmap {
    let (width, height) = (width.max(1), height.max(1));
    let mut pixmap = Pixmap::new(width, height).expect("poster size is valid");
    let paper = poster.paper_color.rgb();
    let ink = poster.paper_color.ink();
    pixmap.fill(paint::color(paper, 1.0));
    let (w, h) = (width as f32, height as f32);
    let unit = w.min(h);
    let geo = geometry(poster, w, h);

    // The design, drawn into its own pixmap and placed on the page
    let (ax, ay) = (geo.art.x().round(), geo.art.y().round());
    let (aw, ah) = (
        (geo.art.right().round() - ax) as u32,
        (geo.art.bottom().round() - ay) as u32,
    );
    if let Some(mut art) = Pixmap::new(aw, ah) {
        poster.design.draw(
            &mut art,
            poster.values(),
            palette::get(poster.palette),
            poster.seed,
        );
        let mask = geo.circle.and_then(|(cx, cy, r)| {
            let mut mask = Mask::new(width, height)?;
            let path = PathBuilder::from_circle(cx, cy, r)?;
            mask.fill_path(&path, FillRule::Winding, true, Transform::identity());
            Some(mask)
        });
        pixmap.draw_pixmap(
            ax as i32,
            ay as i32,
            art.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            mask.as_ref(),
        );
    }

    // Outline around the circle or the frame
    if poster.outline > 0.0 && poster.layout != Layout::FullPage {
        let width = poster.outline / 1000.0 * unit;
        let path = match geo.circle {
            Some((cx, cy, r)) => PathBuilder::from_circle(cx, cy, r + width / 2.0),
            None => geo
                .art
                .outset(width / 2.0, width / 2.0)
                .map(PathBuilder::from_rect),
        };
        if let Some(path) = path {
            paint::stroke_with(
                &mut pixmap,
                &path,
                ink,
                1.0,
                width,
                tiny_skia::LineCap::Butt,
            );
        }
    }

    draw_text(&mut pixmap, poster, &geo, unit, ink);
    pixmap
}

fn draw_text(pixmap: &mut Pixmap, poster: &Poster, geo: &Geometry, unit: f32, ink: Rgb) {
    let t = &poster.text;
    let (title_size, subtitle_size) = text_sizes(poster, unit);
    let case = |s: &str| {
        let s = s.trim();
        if t.uppercase {
            s.to_uppercase()
        } else {
            s.to_string()
        }
    };
    let mut baseline = geo.text_top;
    for (line, size) in [
        (case(&t.title), title_size),
        (case(&t.subtitle), subtitle_size),
    ] {
        if size <= 0.0 {
            continue;
        }
        let mut style = Style {
            typeface: t.typeface,
            size,
            weight: t.weight,
            tracking: t.tracking,
            color: paint::color(ink, 1.0),
        };
        // Long lines shrink to fit the width
        let width = text::measure(&line, &style);
        if width > geo.text_width && width > 0.0 {
            style.size *= geo.text_width / width;
        }
        let width = text::measure(&line, &style);
        if baseline > geo.text_top {
            baseline += title_size * 0.45;
        }
        baseline += size * CAP_HEIGHT;
        let x = match t.align {
            Align::Center => (pixmap.width() as f32 - width) / 2.0,
            Align::Left => geo.text_left,
        };
        text::draw(pixmap, &line, x, baseline, &style);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a4_at_300_dpi() {
        let poster = Poster::default();
        assert_eq!(poster.pixels_at_dpi(300.0), (2480, 3508));
        let landscape = Poster {
            landscape: true,
            ..Poster::default()
        };
        assert_eq!(landscape.pixels_at_dpi(300.0), (3508, 2480));
    }

    #[test]
    fn every_layout_renders() {
        for layout in Layout::ALL {
            for show in [true, false] {
                let mut poster = Poster {
                    layout,
                    outline: 4.0,
                    ..Poster::default()
                };
                poster.text.show = show;
                let pixmap = render(&poster, 200);
                assert_eq!(pixmap.height(), 283);
            }
        }
    }

    #[test]
    fn circle_layout_keeps_corners_paper_coloured() {
        for paper_color in PaperColor::ALL {
            let poster = Poster {
                paper_color,
                ..Poster::default()
            };
            let pixmap = render(&poster, 120);
            let rgb = paper_color.rgb();
            assert_eq!(&pixmap.data()[..3], &rgb, "{paper_color:?}");
        }
    }

    #[test]
    fn same_settings_same_picture_at_any_size() {
        // Coarse check: the average colour barely changes with resolution
        let poster = Poster {
            layout: Layout::FullPage,
            ..Poster::default()
        };
        let mean = |px: &Pixmap| {
            let n = (px.width() * px.height()) as f64;
            px.data().iter().step_by(4).map(|&v| v as f64).sum::<f64>() / n
        };
        let small = mean(&render(&poster, 150));
        let large = mean(&render(&poster, 600));
        assert!((small - large).abs() < 12.0, "{small} vs {large}");
    }

    #[test]
    fn old_settings_without_new_fields_still_load() {
        // `#[serde(default)]` fills in what a saved poster doesn't have
        use serde::de::{value::Error, IntoDeserializer};
        let saved: BTreeMap<&str, u64> = [("seed", 42)].into_iter().collect();
        let deserializer: serde::de::value::MapDeserializer<_, Error> = saved.into_deserializer();
        let poster = Poster::deserialize(deserializer).unwrap();
        assert_eq!(poster.seed, 42);
        assert_eq!(poster.paper, PaperSize::A4);
    }
}
