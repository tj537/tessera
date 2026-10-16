//! Colour palettes. Every design draws on the palette's background with its
//! colours, so one palette list serves all designs.

pub type Rgb = [u8; 3];

pub struct Palette {
    pub name: &'static str,
    pub background: Rgb,
    pub colors: &'static [Rgb],
}

const fn hex(v: u32) -> Rgb {
    [(v >> 16) as u8, (v >> 8) as u8, v as u8]
}

pub const PALETTES: &[Palette] = &[
    Palette {
        name: "Bauhaus",
        background: hex(0xF1E9DA),
        colors: &[hex(0xE63B2E), hex(0x1D4E89), hex(0xF2B134), hex(0x1A1A1A)],
    },
    Palette {
        name: "Ink",
        background: hex(0xF4F1EA),
        colors: &[hex(0x161616)],
    },
    Palette {
        name: "Night",
        background: hex(0x0E0E12),
        colors: &[hex(0xF2F2F2)],
    },
    Palette {
        name: "Riso",
        background: hex(0xF7F3E8),
        colors: &[hex(0xFF48B0), hex(0x0078BF), hex(0xFFE800)],
    },
    Palette {
        name: "Sunset",
        background: hex(0x2B1B3D),
        colors: &[
            hex(0xFF6B6B),
            hex(0xFFB86B),
            hex(0xFFE66D),
            hex(0xC06C84),
            hex(0x6C5B7B),
        ],
    },
    Palette {
        name: "Ocean",
        background: hex(0x0B2545),
        colors: &[
            hex(0x8DA9C4),
            hex(0xEEF4ED),
            hex(0x3E7CB1),
            hex(0x81C3D7),
            hex(0x1F4E79),
        ],
    },
    Palette {
        name: "Forest",
        background: hex(0xEDE6D6),
        colors: &[
            hex(0x2D4A3E),
            hex(0x5B7F5A),
            hex(0xA3B18A),
            hex(0xD9A441),
            hex(0x8C4A2F),
        ],
    },
    Palette {
        name: "Neon",
        background: hex(0x0A0A0A),
        colors: &[hex(0xFF2E88), hex(0x00E5FF), hex(0xB4FF39), hex(0xFFD400)],
    },
    Palette {
        name: "Pastel",
        background: hex(0xFBF7F2),
        colors: &[
            hex(0xF4A6A6),
            hex(0xA8D5BA),
            hex(0xA7C7E7),
            hex(0xF9D89C),
            hex(0xC3B1E1),
        ],
    },
];

/// The palette at `index`, or the first one if the index is out of range
/// (e.g. saved by a newer version with more palettes)
pub fn get(index: usize) -> &'static Palette {
    PALETTES.get(index).unwrap_or(&PALETTES[0])
}
