//! Pictures for checking the look by eye – not run by default:
//!
//!     cargo test --release render_ -- --ignored
//!
//! Writes the posters and the app window to `target/snapshots/*.png`.

use crate::app::TesseraApp;
use crate::designs::Design;
use crate::offscreen::Offscreen;
use crate::poster::{self, Layout, PaperColor, Poster};
use std::path::PathBuf;

fn out(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/snapshots");
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

/// Every design in every layout, plus a few palettes
#[test]
#[ignore]
fn render_posters() {
    for design in Design::ALL {
        for (layout, paper_color, palette) in [
            (Layout::Circle, PaperColor::White, 0),
            (Layout::FullPage, PaperColor::White, 3),
            (Layout::Frame, PaperColor::Black, 4),
            (Layout::Circle, PaperColor::Black, 2),
        ] {
            let poster = Poster {
                design,
                layout,
                paper_color,
                palette,
                seed: 12,
                ..Poster::default()
            };
            let name = format!(
                "poster-{}-{}-{}.png",
                design.id(),
                layout.name().replace(' ', "").to_lowercase(),
                paper_color.name().to_lowercase()
            );
            poster::render(&poster, 900).save_png(out(&name)).unwrap();
        }
    }
}

/// The whole window, as it opens
#[test]
#[ignore]
fn render_app_window() {
    for (name, poster) in [
        ("app-flow", Poster::default()),
        (
            "app-tiles",
            Poster {
                design: Design::Tiles,
                layout: Layout::FullPage,
                ..Poster::default()
            },
        ),
        (
            "app-ridges",
            Poster {
                design: Design::Ridges,
                palette: 2,
                paper_color: PaperColor::Black,
                ..Poster::default()
            },
        ),
    ] {
        let mut screen = Offscreen::retina(1360.0, 860.0, 2.0);
        let mut app = TesseraApp::for_snapshot(&screen.ctx, poster);
        let mut primitives = Vec::new();
        // A few frames, so layout and textures settle
        for frame in 0..3 {
            primitives = screen.frame(frame as f64 * 0.1, |ctx| app.show(ctx));
        }
        screen.save(&primitives, &out(&format!("{name}.png")));
    }
}
