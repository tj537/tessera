// On Windows, don't open a console window next to the app (release builds only)
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;

fn main() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1360.0, 860.0])
        .with_min_inner_size([900.0, 560.0])
        .with_title("Tessera");
    // Icon for the window, taskbar and Dock (when started without the .app bundle)
    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")) {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        tessera::app::APP_NAME,
        options,
        Box::new(|cc| Ok(Box::new(tessera::app::TesseraApp::new(cc)))),
    )
}
