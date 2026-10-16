//! The window: format on the left, the poster in the middle, design and
//! parameters on the right.
//!
//! The preview is rendered on a worker thread, so dragging a slider never
//! blocks the window; the newest settings always win.

use crate::designs::{Design, ParamKind};
use crate::export;
use crate::palette::{self, PALETTES};
use crate::poster::{self, Align, Layout, PaperColor, PaperSize, Poster};
use crate::rng::Rng;
use crate::text::Typeface;
use eframe::egui::{
    self, pos2, vec2, Color32, CornerRadius, Key, Margin, Modifiers, Rect, RichText, Sense, Stroke,
    StrokeKind, TextureHandle, TextureOptions,
};
use egui_phosphor::regular as icon;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};

pub const APP_NAME: &str = "Tessera";
const STATE_KEY: &str = "tessera";

const ACCENT: Color32 = Color32::from_rgb(232, 85, 62);
const PANEL: Color32 = Color32::from_rgb(28, 28, 32);
const CANVAS: Color32 = Color32::from_rgb(18, 18, 21);

/// Result of a finished export: the file and its pixel size
type ExportResult = Result<(PathBuf, u32, u32), String>;

/// What is remembered between launches
#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Saved {
    poster: Poster,
    dpi: u32,
}

impl Default for Saved {
    fn default() -> Self {
        Self {
            poster: Poster::default(),
            dpi: 300,
        }
    }
}

/// A finished preview: the settings it shows and its pixels
struct Rendered {
    poster: Poster,
    width: u32,
    image: egui::ColorImage,
}

fn render_image(poster: &Poster, width: u32) -> egui::ColorImage {
    let pixmap = poster::render(poster, width);
    egui::ColorImage::from_rgba_premultiplied(
        [pixmap.width() as usize, pixmap.height() as usize],
        pixmap.data(),
    )
}

/// Renders previews in the background
struct Worker {
    jobs: Sender<(Poster, u32)>,
    done: Receiver<Rendered>,
}

impl Worker {
    fn spawn(ctx: egui::Context) -> Self {
        let (jobs, inbox) = mpsc::channel::<(Poster, u32)>();
        let (outbox, done) = mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(mut job) = inbox.recv() {
                // Skip settings that are already outdated
                while let Ok(newer) = inbox.try_recv() {
                    job = newer;
                }
                let (poster, width) = job;
                let image = render_image(&poster, width);
                if outbox
                    .send(Rendered {
                        poster,
                        width,
                        image,
                    })
                    .is_err()
                {
                    break;
                }
                ctx.request_repaint();
            }
        });
        Self { jobs, done }
    }
}

pub struct TesseraApp {
    poster: Poster,
    dpi: u32,
    preview: Option<TextureHandle>,
    /// Settings and width of the preview that is shown / last requested
    shown: Option<(Poster, u32)>,
    requested: Option<(Poster, u32)>,
    /// None renders synchronously (for the snapshot tests)
    worker: Option<Worker>,
    thumbnails: HashMap<(Design, usize), TextureHandle>,
    export: Option<Receiver<ExportResult>>,
    status: Option<String>,
}

impl TesseraApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_style(&cc.egui_ctx);
        let saved: Saved = cc
            .storage
            .and_then(|s| eframe::get_value(s, STATE_KEY))
            .unwrap_or_default();
        Self {
            worker: Some(Worker::spawn(cc.egui_ctx.clone())),
            ..Self::with_poster(saved.poster, saved.dpi)
        }
    }

    fn with_poster(poster: Poster, dpi: u32) -> Self {
        Self {
            poster,
            dpi,
            preview: None,
            shown: None,
            requested: None,
            worker: None,
            thumbnails: HashMap::new(),
            export: None,
            status: None,
        }
    }

    /// For the snapshot tests: renders previews synchronously
    #[cfg(test)]
    pub fn for_snapshot(ctx: &egui::Context, poster: Poster) -> Self {
        setup_style(ctx);
        Self::with_poster(poster, 300)
    }

    fn new_seed(&mut self) {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        // Short numbers are easier to note down
        self.poster.seed = Rng::new(nanos ^ self.poster.seed).next_u64() % 100_000;
    }

    /// Random parameter values around the defaults (the extremes rarely look good)
    fn shuffle_params(&mut self) {
        let mut rng = Rng::new(self.poster.seed.wrapping_mul(31).wrapping_add(7));
        let specs = self.poster.design.specs();
        let values = self.poster.values_mut();
        for spec in specs {
            let value = match spec.kind {
                ParamKind::Toggle => {
                    if rng.chance(0.5) {
                        1.0
                    } else {
                        0.0
                    }
                }
                _ => spec.default + rng.range(-0.35, 0.35) * (spec.max - spec.min),
            };
            values.insert(spec.key.to_string(), spec.clean(value));
        }
        self.new_seed();
    }

    fn handle_keys(&mut self, ctx: &egui::Context) {
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::E)) {
            self.start_export(ctx);
        }
        if ctx.wants_keyboard_input() {
            return;
        }
        let (space, left, right) = ctx.input(|i| {
            (
                i.key_pressed(Key::Space),
                i.key_pressed(Key::ArrowLeft),
                i.key_pressed(Key::ArrowRight),
            )
        });
        if space {
            self.new_seed();
        }
        if left {
            self.poster.seed = self.poster.seed.saturating_sub(1);
        }
        if right {
            self.poster.seed = self.poster.seed.saturating_add(1);
        }
    }

    fn poll(&mut self, ctx: &egui::Context) {
        let finished: Vec<Rendered> = self
            .worker
            .as_ref()
            .map(|w| w.done.try_iter().collect())
            .unwrap_or_default();
        for rendered in finished {
            self.show_preview(ctx, rendered);
        }
        if let Some(rx) = &self.export {
            if let Ok(result) = rx.try_recv() {
                self.status = Some(match result {
                    Ok((path, w, h)) => format!(
                        "Saved {} ({w} × {h} px)",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    ),
                    Err(e) => format!("Export failed: {e}"),
                });
                self.export = None;
            }
        }
    }

    fn show_preview(&mut self, ctx: &egui::Context, rendered: Rendered) {
        match &mut self.preview {
            Some(texture) => texture.set(rendered.image, TextureOptions::LINEAR),
            None => {
                self.preview =
                    Some(ctx.load_texture("poster", rendered.image, TextureOptions::LINEAR))
            }
        }
        self.shown = Some((rendered.poster, rendered.width));
    }

    fn request_preview(&mut self, ctx: &egui::Context, width: u32) {
        let key = (self.poster.clone(), width);
        if self.requested.as_ref() == Some(&key) {
            return;
        }
        self.requested = Some(key);
        match &self.worker {
            Some(worker) => {
                let _ = worker.jobs.send((self.poster.clone(), width));
            }
            None => {
                let image = render_image(&self.poster, width);
                let poster = self.poster.clone();
                self.show_preview(
                    ctx,
                    Rendered {
                        poster,
                        width,
                        image,
                    },
                );
            }
        }
    }

    fn start_export(&mut self, ctx: &egui::Context) {
        if self.export.is_some() {
            return;
        }
        let Some(mut path) = rfd::FileDialog::new()
            .add_filter("PNG image", &["png"])
            .set_file_name(export::file_name(&self.poster))
            .save_file()
        else {
            return;
        };
        if path.extension().is_none() {
            path.set_extension("png");
        }
        let (poster, dpi, ctx) = (self.poster.clone(), self.dpi, ctx.clone());
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = export::save_png(&poster, dpi, &path).map(|(w, h)| (path, w, h));
            let _ = tx.send(result);
            ctx.request_repaint();
        });
        self.export = Some(rx);
        self.status = Some("Exporting…".into());
    }

    fn thumbnail(&mut self, ctx: &egui::Context, design: Design) -> TextureHandle {
        let key = (design, self.poster.palette);
        self.thumbnails
            .entry(key)
            .or_insert_with(|| {
                let mut pixmap = tiny_skia::Pixmap::new(144, 144).unwrap();
                design.draw(&mut pixmap, &Default::default(), palette::get(key.1), 7);
                let image = egui::ColorImage::from_rgba_premultiplied([144, 144], pixmap.data());
                ctx.load_texture(
                    format!("thumb-{}-{}", design.id(), key.1),
                    image,
                    TextureOptions::LINEAR,
                )
            })
            .clone()
    }

    // ── Left: format ────────────────────────────────────────────────────

    fn format_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Tessera").size(22.0).strong());
            ui.label(RichText::new("generative posters").weak());
        });

        section(ui, "Paper");
        let p = &mut self.poster;
        egui::ComboBox::from_id_salt("paper")
            .width(ui.available_width())
            .selected_text(paper_label(p.paper))
            .show_ui(ui, |ui| {
                for size in PaperSize::ALL {
                    ui.selectable_value(&mut p.paper, size, paper_label(size));
                }
            });
        if p.paper != PaperSize::Square {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut p.landscape, false, "Portrait");
                ui.selectable_value(&mut p.landscape, true, "Landscape");
            });
        }
        ui.horizontal(|ui| {
            for color in PaperColor::ALL {
                swatch_choice(ui, &mut p.paper_color, color, color.rgb(), color.name());
            }
        });

        section(ui, "Layout");
        ui.horizontal(|ui| {
            for (layout, glyph) in [
                (Layout::FullPage, icon::CORNERS_OUT),
                (Layout::Frame, icon::FRAME_CORNERS),
                (Layout::Circle, icon::CIRCLE),
            ] {
                ui.selectable_value(&mut p.layout, layout, format!("{glyph} {}", layout.name()));
            }
        });
        if p.layout != Layout::FullPage {
            slider(ui, "Margin", &mut p.margin, 0.0..=25.0, " %");
        }
        if p.layout == Layout::Circle {
            slider(ui, "Circle size", &mut p.circle_size, 30.0..=100.0, " %");
        }
        if p.layout != Layout::FullPage {
            slider(ui, "Outline", &mut p.outline, 0.0..=20.0, "");
        }

        section(ui, "Text");
        let t = &mut p.text;
        ui.checkbox(&mut t.show, "Show text under the design");
        if t.show {
            ui.add_space(4.0);
            ui.label("Title");
            ui.add(egui::TextEdit::singleline(&mut t.title).desired_width(f32::INFINITY));
            ui.label("Subtitle");
            ui.add(egui::TextEdit::singleline(&mut t.subtitle).desired_width(f32::INFINITY));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                for face in Typeface::ALL {
                    ui.selectable_value(&mut t.typeface, face, face.name());
                }
                ui.separator();
                ui.selectable_value(&mut t.align, Align::Center, icon::TEXT_ALIGN_CENTER)
                    .on_hover_text("Centred");
                ui.selectable_value(&mut t.align, Align::Left, icon::TEXT_ALIGN_LEFT)
                    .on_hover_text("Left");
                ui.separator();
                ui.toggle_value(&mut t.uppercase, "AA")
                    .on_hover_text("Capital letters");
            });
            slider(ui, "Size", &mut t.size, 2.0..=16.0, " %");
            slider(ui, "Subtitle size", &mut t.subtitle_size, 0.15..=0.8, "");
            slider(ui, "Weight", &mut t.weight, 0.0..=1.0, "");
            slider(ui, "Letter spacing", &mut t.tracking, -0.05..=0.8, "");
        }
    }

    fn export_ui(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        section(ui, "Export");
        let (w, h) = self.poster.pixels_at_dpi(self.dpi as f32);
        let label = |dpi: u32, name: &str| format!("{dpi} dpi · {name}");
        let current = export::DPI_CHOICES
            .iter()
            .find(|c| c.0 == self.dpi)
            .map(|c| label(c.0, c.1))
            .unwrap_or_else(|| format!("{} dpi", self.dpi));
        egui::ComboBox::from_id_salt("dpi")
            .width(ui.available_width())
            .selected_text(current)
            .show_ui(ui, |ui| {
                for (dpi, name) in export::DPI_CHOICES {
                    ui.selectable_value(&mut self.dpi, dpi, label(dpi, name));
                }
            });
        ui.label(RichText::new(format!("{w} × {h} pixels")).weak());
        ui.add_space(6.0);
        let busy = self.export.is_some();
        let button = egui::Button::new(
            RichText::new(format!("{}  Export PNG", icon::DOWNLOAD_SIMPLE))
                .size(15.0)
                .color(Color32::WHITE),
        )
        .fill(ACCENT)
        .min_size(vec2(ui.available_width(), 34.0));
        if ui.add_enabled(!busy, button).on_hover_text("⌘E").clicked() {
            self.start_export(ui.ctx());
        }
        if let Some(status) = &self.status {
            ui.horizontal_wrapped(|ui| {
                if busy {
                    ui.spinner();
                }
                ui.label(RichText::new(status).weak());
            });
        }
        ui.add_space(4.0);
    }

    // ── Right: design ───────────────────────────────────────────────────

    fn design_ui(&mut self, ui: &mut egui::Ui) {
        section(ui, "Design");
        let ctx = ui.ctx().clone();
        let thumbs: Vec<(Design, TextureHandle)> = Design::ALL
            .iter()
            .map(|&d| (d, self.thumbnail(&ctx, d)))
            .collect();
        ui.horizontal(|ui| {
            let size = (ui.available_width() - 2.0 * ui.spacing().item_spacing.x) / 3.0;
            for (design, texture) in thumbs {
                let selected = self.poster.design == design;
                if design_card(ui, &texture, design.name(), size, selected).clicked() {
                    self.poster.design = design;
                }
            }
        });
        ui.add_space(4.0);
        ui.label(RichText::new(self.poster.design.description()).weak());

        section(ui, "Seed");
        ui.horizontal(|ui| {
            if ui
                .button(icon::CARET_LEFT)
                .on_hover_text("Previous (←)")
                .clicked()
            {
                self.poster.seed = self.poster.seed.saturating_sub(1);
            }
            ui.add(egui::DragValue::new(&mut self.poster.seed).speed(1.0));
            if ui
                .button(icon::CARET_RIGHT)
                .on_hover_text("Next (→)")
                .clicked()
            {
                self.poster.seed = self.poster.seed.saturating_add(1);
            }
            if ui
                .button(format!("{} Surprise me", icon::DICE_FIVE))
                .on_hover_text("New random seed (Space)")
                .clicked()
            {
                self.new_seed();
            }
        });

        section(ui, "Palette");
        for (index, palette) in PALETTES.iter().enumerate() {
            let selected = self.poster.palette == index;
            if palette_row(ui, palette, selected).clicked() {
                self.poster.palette = index;
            }
        }

        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("PARAMETERS").small().strong().weak());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button(format!("{} Reset", icon::ARROW_COUNTER_CLOCKWISE))
                    .clicked()
                {
                    self.poster.values_mut().clear();
                }
                if ui
                    .small_button(format!("{} Shuffle", icon::SHUFFLE))
                    .on_hover_text("Random parameters and seed")
                    .clicked()
                {
                    self.shuffle_params();
                }
            });
        });
        ui.add_space(2.0);
        for spec in self.poster.design.specs() {
            let mut value = self.poster.value(spec.key);
            let changed = match spec.kind {
                ParamKind::Toggle => {
                    let mut on = value >= 0.5;
                    let changed = ui
                        .checkbox(&mut on, spec.label)
                        .on_hover_text(spec.hint)
                        .changed();
                    value = if on { 1.0 } else { 0.0 };
                    changed
                }
                kind => {
                    ui.label(spec.label).on_hover_text(spec.hint);
                    ui.spacing_mut().slider_width = ui.available_width() - 64.0;
                    let mut slider = egui::Slider::new(&mut value, spec.min..=spec.max);
                    if kind == ParamKind::Int {
                        slider = slider.integer();
                    }
                    ui.add(slider).on_hover_text(spec.hint).changed()
                }
            };
            if changed {
                self.poster
                    .values_mut()
                    .insert(spec.key.to_string(), spec.clean(value));
            }
        }
        ui.add_space(8.0);
    }

    // ── Middle: the poster ──────────────────────────────────────────────

    fn canvas_ui(&mut self, ui: &mut egui::Ui) {
        let full = ui.available_rect_before_wrap();
        let area = full.shrink2(vec2(40.0, 48.0));
        let aspect = self.poster.aspect();
        let mut size = vec2(area.width(), area.width() * aspect);
        if size.y > area.height() {
            size = vec2(area.height() / aspect, area.height());
        }
        let rect = Rect::from_center_size(area.center() - vec2(0.0, 6.0), size);

        // Render at the size the poster takes on screen, in steps (fewer re-renders while resizing)
        let pixels = size.x * ui.ctx().pixels_per_point();
        let width = ((pixels / 64.0).ceil() as u32 * 64).clamp(128, 2048);
        self.request_preview(ui.ctx(), width);

        let painter = ui.painter_at(full);
        painter.add(
            egui::epaint::Shadow {
                offset: [0, 10],
                blur: 36,
                spread: 0,
                color: Color32::from_black_alpha(140),
            }
            .as_shape(rect, CornerRadius::ZERO),
        );
        match &self.preview {
            Some(texture) => {
                let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                painter.image(texture.id(), rect, uv, Color32::WHITE);
            }
            None => {
                let [r, g, b] = self.poster.paper_color.rgb();
                painter.rect_filled(rect, CornerRadius::ZERO, Color32::from_rgb(r, g, b));
            }
        }

        let (w, h) = self.poster.size_mm();
        let caption = format!(
            "{} · {:.0} × {:.0} mm · {} · {} · seed {}",
            self.poster.paper.name(),
            w,
            h,
            self.poster.design.name(),
            palette::get(self.poster.palette).name,
            self.poster.seed
        );
        painter.text(
            pos2(rect.center().x, rect.bottom() + 18.0),
            egui::Align2::CENTER_CENTER,
            caption,
            egui::FontId::proportional(12.0),
            Color32::from_gray(120),
        );
        if self.shown != self.requested {
            let spinner = Rect::from_min_size(rect.right_top() + vec2(8.0, 0.0), vec2(16.0, 16.0));
            egui::Spinner::new()
                .color(Color32::from_gray(140))
                .paint_at(ui, spinner);
        }
        ui.allocate_rect(full, Sense::hover());
    }
}

impl TesseraApp {
    /// One frame of the whole window
    pub fn show(&mut self, ctx: &egui::Context) {
        self.handle_keys(ctx);
        self.poll(ctx);

        let panel = egui::Frame::new()
            .fill(PANEL)
            .inner_margin(Margin::symmetric(16, 12));
        egui::SidePanel::left("format")
            .resizable(false)
            .exact_width(286.0)
            .frame(panel)
            .show(ctx, |ui| {
                egui::TopBottomPanel::bottom("export")
                    .frame(egui::Frame::new().fill(PANEL))
                    .show_separator_line(true)
                    .show_inside(ui, |ui| self.export_ui(ui));
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| self.format_ui(ui));
            });
        egui::SidePanel::right("design")
            .resizable(false)
            .exact_width(300.0)
            .frame(panel)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| self.design_ui(ui));
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(CANVAS))
            .show(ctx, |ui| self.canvas_ui(ui));
    }
}

impl eframe::App for TesseraApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.show(ctx);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(
            storage,
            STATE_KEY,
            &Saved {
                poster: self.poster.clone(),
                dpi: self.dpi,
            },
        );
    }
}

fn setup_style(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    ctx.set_fonts(fonts);
    ctx.set_visuals(egui::Visuals::dark());
    ctx.style_mut(|style| {
        style.spacing.item_spacing = vec2(8.0, 6.0);
        style.spacing.button_padding = vec2(8.0, 4.0);
        style.spacing.interact_size.y = 24.0;
        style.visuals.panel_fill = PANEL;
        style.visuals.selection.bg_fill = ACCENT.linear_multiply(0.85);
        style.visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
        style.visuals.slider_trailing_fill = true;
    });
}

fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(14.0);
    ui.label(RichText::new(title.to_uppercase()).small().strong().weak());
    ui.add_space(2.0);
}

fn paper_label(size: PaperSize) -> String {
    let (w, h) = size.mm();
    format!("{} · {:.0} × {:.0} mm", size.name(), w, h)
}

fn slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    suffix: &str,
) {
    ui.label(label);
    ui.spacing_mut().slider_width = ui.available_width() - 64.0;
    ui.add(egui::Slider::new(value, range).suffix(suffix));
}

/// A round colour swatch with a name, selectable
fn swatch_choice<T: PartialEq + Copy>(
    ui: &mut egui::Ui,
    current: &mut T,
    value: T,
    rgb: [u8; 3],
    name: &str,
) {
    let selected = *current == value;
    let (rect, response) = ui.allocate_exact_size(vec2(78.0, 26.0), Sense::click());
    let visuals = ui.style().interact_selectable(&response, selected);
    let painter = ui.painter();
    if selected || response.hovered() {
        painter.rect_filled(rect, 6, visuals.weak_bg_fill);
    }
    let dot = pos2(rect.left() + 13.0, rect.center().y);
    painter.circle(
        dot,
        7.0,
        Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
        Stroke::new(1.0_f32, Color32::from_gray(110)),
    );
    painter.text(
        pos2(rect.left() + 26.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        name,
        egui::FontId::proportional(13.0),
        visuals.text_color(),
    );
    if response.clicked() {
        *current = value;
    }
}

fn design_card(
    ui: &mut egui::Ui,
    texture: &TextureHandle,
    name: &str,
    size: f32,
    selected: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(size, size + 22.0), Sense::click());
    let image = Rect::from_min_size(rect.min, vec2(size, size));
    let painter = ui.painter();
    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    painter.image(texture.id(), image, uv, Color32::WHITE);
    let stroke = if selected {
        Stroke::new(2.5_f32, ACCENT)
    } else if response.hovered() {
        Stroke::new(1.5_f32, Color32::from_gray(160))
    } else {
        Stroke::new(1.0_f32, Color32::from_gray(60))
    };
    painter.rect_stroke(image, 4, stroke, StrokeKind::Outside);
    painter.text(
        pos2(image.center().x, image.bottom() + 12.0),
        egui::Align2::CENTER_CENTER,
        name,
        egui::FontId::proportional(13.0),
        if selected {
            Color32::WHITE
        } else {
            Color32::from_gray(170)
        },
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn palette_row(ui: &mut egui::Ui, palette: &palette::Palette, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::click());
    let visuals = ui.style().interact_selectable(&response, selected);
    let painter = ui.painter();
    if selected || response.hovered() {
        painter.rect_filled(rect, 6, visuals.weak_bg_fill);
    }
    let color = |c: [u8; 3]| Color32::from_rgb(c[0], c[1], c[2]);
    // Background as a wide chip, the colours as dots on it
    let chip = Rect::from_min_size(pos2(rect.left() + 6.0, rect.top() + 4.0), vec2(112.0, 18.0));
    painter.rect(
        chip,
        9,
        color(palette.background),
        Stroke::new(1.0_f32, Color32::from_gray(80)),
        StrokeKind::Inside,
    );
    for (i, &c) in palette.colors.iter().enumerate() {
        painter.circle_filled(
            pos2(chip.left() + 13.0 + i as f32 * 20.0, chip.center().y),
            6.0,
            color(c),
        );
    }
    painter.text(
        pos2(chip.right() + 12.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        palette.name,
        egui::FontId::proportional(13.0),
        visuals.text_color(),
    );
    if selected {
        painter.text(
            pos2(rect.right() - 10.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            icon::CHECK,
            egui::FontId::proportional(14.0),
            ACCENT,
        );
    }
    response
}
