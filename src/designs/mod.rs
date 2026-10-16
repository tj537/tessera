//! The generative designs. Each one describes its knobs (`ParamSpec`) and
//! draws itself into a pixmap. All sizes are relative to the pixmap, so the
//! same settings look the same in the preview and in a 300 dpi export.

mod flow;
mod ridges;
mod tiles;

use crate::palette::Palette;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tiny_skia::Pixmap;

#[derive(Clone, Copy, PartialEq)]
pub enum ParamKind {
    Float,
    /// Whole numbers (counts)
    Int,
    /// On/off, stored as 0 or 1
    Toggle,
}

/// One knob in the right panel
pub struct ParamSpec {
    pub key: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub kind: ParamKind,
    pub min: f32,
    pub max: f32,
    pub default: f32,
}

impl ParamSpec {
    pub const fn float(key: &'static str, label: &'static str, hint: &'static str) -> Self {
        Self {
            key,
            label,
            hint,
            kind: ParamKind::Float,
            min: 0.0,
            max: 1.0,
            default: 0.5,
        }
    }

    pub const fn int(self) -> Self {
        Self {
            kind: ParamKind::Int,
            ..self
        }
    }

    pub const fn toggle(self, on: bool) -> Self {
        Self {
            kind: ParamKind::Toggle,
            min: 0.0,
            max: 1.0,
            default: if on { 1.0 } else { 0.0 },
            ..self
        }
    }

    pub const fn range(self, min: f32, max: f32, default: f32) -> Self {
        Self {
            min,
            max,
            default,
            ..self
        }
    }

    /// Bring a stored value into range (and to a whole number where needed)
    pub fn clean(&self, value: f32) -> f32 {
        let value = if value.is_finite() {
            value.clamp(self.min, self.max)
        } else {
            self.default
        };
        match self.kind {
            ParamKind::Float => value,
            ParamKind::Int | ParamKind::Toggle => value.round(),
        }
    }
}

/// Parameter values of one design, by key
pub type Values = BTreeMap<String, f32>;

/// Read access for the drawing code: missing keys fall back to the default
pub struct Params<'a> {
    specs: &'static [ParamSpec],
    values: &'a Values,
}

impl Params<'_> {
    pub fn get(&self, key: &str) -> f32 {
        let spec = self
            .specs
            .iter()
            .find(|s| s.key == key)
            .unwrap_or_else(|| panic!("unknown parameter {key}"));
        spec.clean(self.values.get(key).copied().unwrap_or(spec.default))
    }

    pub fn count(&self, key: &str) -> usize {
        self.get(key).max(0.0) as usize
    }

    pub fn on(&self, key: &str) -> bool {
        self.get(key) >= 0.5
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, Default)]
pub enum Design {
    #[default]
    Flow,
    Ridges,
    Tiles,
}

impl Design {
    pub const ALL: [Design; 3] = [Design::Flow, Design::Ridges, Design::Tiles];

    /// Stable name, used as key for the saved parameters
    pub fn id(self) -> &'static str {
        match self {
            Design::Flow => "flow",
            Design::Ridges => "ridges",
            Design::Tiles => "tiles",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Design::Flow => "Flow",
            Design::Ridges => "Ridges",
            Design::Tiles => "Bauhaus",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Design::Flow => "Thousands of lines drift through an invisible current of noise.",
            Design::Ridges => "Stacked mountain lines, like a radio signal from a pulsar.",
            Design::Tiles => "A grid of circles, quarter circles and triangles in bold colours.",
        }
    }

    pub fn specs(self) -> &'static [ParamSpec] {
        match self {
            Design::Flow => flow::SPECS,
            Design::Ridges => ridges::SPECS,
            Design::Tiles => tiles::SPECS,
        }
    }

    /// Draw into the whole pixmap
    pub fn draw(self, pixmap: &mut Pixmap, values: &Values, palette: &Palette, seed: u64) {
        let params = Params {
            specs: self.specs(),
            values,
        };
        // Mix the design into the seed, so one seed gives unrelated designs
        let seed = seed ^ (self as u64 + 1).wrapping_mul(0xA24B_AED4_963E_E407);
        match self {
            Design::Flow => flow::draw(pixmap, &params, palette, seed),
            Design::Ridges => ridges::draw(pixmap, &params, palette, seed),
            Design::Tiles => tiles::draw(pixmap, &params, palette, seed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specs_are_sane() {
        for design in Design::ALL {
            let mut keys = std::collections::HashSet::new();
            for spec in design.specs() {
                assert!(keys.insert(spec.key), "{} twice in {:?}", spec.key, design);
                assert!(spec.min < spec.max, "{}", spec.key);
                assert!(
                    (spec.min..=spec.max).contains(&spec.default),
                    "{} default out of range",
                    spec.key
                );
            }
        }
    }

    #[test]
    fn every_design_draws_something() {
        for design in Design::ALL {
            let mut pixmap = Pixmap::new(120, 160).unwrap();
            design.draw(&mut pixmap, &Values::new(), crate::palette::get(0), 1);
            let first = &pixmap.data()[..4];
            let varied = pixmap.data().chunks(4).any(|px| px != first);
            assert!(varied, "{design:?} drew a flat image");
            assert!(
                pixmap.data().chunks(4).all(|px| px[3] == 255),
                "{design:?} left holes"
            );
        }
    }
}
