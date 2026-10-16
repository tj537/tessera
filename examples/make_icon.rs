//! Draws the app icon – four Bauhaus tiles in a rounded square.
//!
//!     cargo run --release --example make_icon -- <output.iconset>
//!
//! With `--assets` instead of a folder it rewrites `assets/icon.png` (window
//! icon) and `assets/icon.ico` (Windows, several sizes).

use std::f32::consts::FRAC_PI_2;
use std::path::{Path, PathBuf};
use tessera::paint::{arc, circle, fill, fill_rect, polygon};
use tiny_skia::{FillRule, Mask, PathBuilder, Pixmap, PixmapPaint, Rect, Transform};

const CREAM: [u8; 3] = [241, 233, 218];
const RED: [u8; 3] = [230, 59, 46];
const BLUE: [u8; 3] = [29, 78, 137];
const YELLOW: [u8; 3] = [242, 177, 52];
const BLACK: [u8; 3] = [26, 26, 26];

fn render(px: u32) -> Pixmap {
    // Drawn on a 1024 grid like macOS icons: 824 px body with a soft corner
    let s = px as f32 / 1024.0;
    let mut art = Pixmap::new(px, px).unwrap();
    let (x0, t) = (100.0 * s, 412.0 * s);
    let quarter = |art: &mut Pixmap, cx: f32, cy: f32, r: f32, start: f32, rgb| {
        let mut pb = PathBuilder::new();
        pb.move_to(cx, cy);
        arc(&mut pb, cx, cy, r, start, start + FRAC_PI_2);
        pb.close();
        fill(art, &pb.finish().unwrap(), rgb);
    };
    // Top left: blue with a yellow quarter circle
    fill_rect(&mut art, x0, x0, t, t, BLUE);
    quarter(&mut art, x0 + t, x0 + t, t, 2.0 * FRAC_PI_2, YELLOW);
    // Top right: cream with a red half circle
    fill_rect(&mut art, x0 + t, x0, t, t, CREAM);
    let mut pb = PathBuilder::new();
    arc(
        &mut pb,
        x0 + t,
        x0 + t / 2.0,
        t / 2.0,
        -FRAC_PI_2,
        FRAC_PI_2,
    );
    pb.close();
    fill(&mut art, &pb.finish().unwrap(), RED);
    // Bottom left: red with a cream circle
    fill_rect(&mut art, x0, x0 + t, t, t, RED);
    fill(
        &mut art,
        &circle(x0 + t / 2.0, x0 + 1.5 * t, t * 0.3).unwrap(),
        CREAM,
    );
    // Bottom right: yellow with a black triangle
    fill_rect(&mut art, x0 + t, x0 + t, t, t, YELLOW);
    let tri = [
        (x0 + t, x0 + 2.0 * t),
        (x0 + 2.0 * t, x0 + 2.0 * t),
        (x0 + 2.0 * t, x0 + t),
    ];
    fill(&mut art, &polygon(&tri).unwrap(), BLACK);

    // Cut out the rounded square
    let mut mask = Mask::new(px, px).unwrap();
    let body = Rect::from_xywh(x0, x0, 2.0 * t, 2.0 * t).unwrap();
    mask.fill_path(
        &rounded(body, 185.0 * s),
        FillRule::Winding,
        true,
        Transform::identity(),
    );
    let mut icon = Pixmap::new(px, px).unwrap();
    icon.draw_pixmap(
        0,
        0,
        art.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        Some(&mask),
    );
    icon
}

fn rounded(r: Rect, radius: f32) -> tiny_skia::Path {
    let mut pb = PathBuilder::new();
    let (l, t, rt, b) = (r.left(), r.top(), r.right(), r.bottom());
    arc(&mut pb, rt - radius, t + radius, radius, -FRAC_PI_2, 0.0);
    arc(&mut pb, rt - radius, b - radius, radius, 0.0, FRAC_PI_2);
    arc(
        &mut pb,
        l + radius,
        b - radius,
        radius,
        FRAC_PI_2,
        2.0 * FRAC_PI_2,
    );
    arc(
        &mut pb,
        l + radius,
        t + radius,
        radius,
        2.0 * FRAC_PI_2,
        3.0 * FRAC_PI_2,
    );
    pb.close();
    pb.finish().unwrap()
}

/// ICO file with embedded PNGs (understood by Windows since Vista)
fn write_ico(path: &Path) {
    let sizes = [16u32, 24, 32, 48, 64, 128, 256];
    let pngs: Vec<Vec<u8>> = sizes
        .iter()
        .map(|&px| render(px).encode_png().expect("could not encode PNG"))
        .collect();
    let mut ico = Vec::new();
    ico.extend_from_slice(&[0, 0, 1, 0]);
    ico.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * sizes.len() as u32;
    for (&px, png) in sizes.iter().zip(&pngs) {
        // 256 is stored as 0
        let side = if px >= 256 { 0 } else { px as u8 };
        ico.extend_from_slice(&[side, side, 0, 0]);
        ico.extend_from_slice(&1u16.to_le_bytes());
        ico.extend_from_slice(&32u16.to_le_bytes());
        ico.extend_from_slice(&(png.len() as u32).to_le_bytes());
        ico.extend_from_slice(&offset.to_le_bytes());
        offset += png.len() as u32;
    }
    for png in &pngs {
        ico.extend_from_slice(png);
    }
    std::fs::write(path, ico).expect("could not write ICO");
}

fn main() {
    let target = std::env::args()
        .nth(1)
        .expect("usage: make_icon <output.iconset> | --assets");
    if target == "--assets" {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        render(256).save_png(assets.join("icon.png")).unwrap();
        write_ico(&assets.join("icon.ico"));
        println!("wrote assets/icon.png and assets/icon.ico");
        return;
    }
    let dir = PathBuf::from(target);
    std::fs::create_dir_all(&dir).expect("could not create the iconset folder");
    for base in [16u32, 32, 128, 256, 512] {
        render(base)
            .save_png(dir.join(format!("icon_{base}x{base}.png")))
            .unwrap();
        render(base * 2)
            .save_png(dir.join(format!("icon_{base}x{base}@2x.png")))
            .unwrap();
    }
}
