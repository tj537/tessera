//! PNG export with the print resolution stored in the file, so printing an
//! A4 export at 100 % gives exactly A4.

use crate::poster::{self, Poster};
use std::path::Path;

/// Resolutions offered in the export menu
pub const DPI_CHOICES: [(u32, &str); 3] = [(300, "Print"), (150, "Draft"), (72, "Screen")];

pub fn save_png(poster: &Poster, dpi: u32, path: &Path) -> Result<(u32, u32), String> {
    let (width, height) = poster.pixels_at_dpi(dpi as f32);
    let pixmap = poster::render_sized(poster, width, height);
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut encoder = png::Encoder::new(
        std::io::BufWriter::new(file),
        pixmap.width(),
        pixmap.height(),
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let per_metre = (dpi as f64 / 0.0254).round() as u32;
    encoder.set_pixel_dims(Some(png::PixelDimensions {
        xppu: per_metre,
        yppu: per_metre,
        unit: png::Unit::Meter,
    }));
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    // The poster is opaque, so tiny-skia's premultiplied pixels are plain RGBA
    writer
        .write_image_data(pixmap.data())
        .map_err(|e| e.to_string())?;
    writer.finish().map_err(|e| e.to_string())?;
    Ok((pixmap.width(), pixmap.height()))
}

/// A file name like `tessera-flow-1234.png`
pub fn file_name(poster: &Poster) -> String {
    format!("tessera-{}-{}.png", poster.design.id(), poster.seed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_writes_png_with_dpi() {
        let dir = std::env::temp_dir().join(format!("tessera-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("poster.png");
        let poster = Poster::default();
        let (w, h) = save_png(&poster, 72, &path).unwrap();
        assert_eq!((w, h), poster.pixels_at_dpi(72.0));

        let decoder = png::Decoder::new(std::fs::File::open(&path).unwrap());
        let reader = decoder.read_info().unwrap();
        let info = reader.info();
        assert_eq!((info.width, info.height), (w, h));
        let dims = info.pixel_dims.unwrap();
        assert_eq!(dims.xppu, 2835);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
