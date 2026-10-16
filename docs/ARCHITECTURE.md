# Architecture

Tessera is a library crate (`src/lib.rs`) plus a tiny binary (`src/main.rs`)
that opens the window. Everything the poster looks like is computed on the CPU
with tiny-skia, independent of egui.

## Rendering

```
Poster (settings) ──► poster::render(width) ──► tiny_skia::Pixmap
                         │
                         ├─ geometry(): where the design, circle and text go
                         ├─ Design::draw(): the design into its own pixmap
                         ├─ placed on the paper (with a circle mask in Circle layout)
                         ├─ optional outline
                         └─ text::draw(): title and subtitle
```

- **Resolution independence.** All sizes are fractions of the poster (or of
  the design's shorter side). Designs draw the same random numbers in the same
  order at every size, so the preview and the 300 dpi export match.
  `poster::tests::same_settings_same_picture_at_any_size` checks this roughly.
- **Text** is drawn from glyph outlines (`ab_glyph`) as tiny-skia paths, so it
  is sharp at any size. *Weight* strokes the outline in addition to filling it.
  Fonts come from `epaint_default_fonts` (the fonts egui ships), nothing extra
  is bundled.
- **Export** (`export.rs`) renders at the chosen dpi and writes the PNG with
  the `png` crate, including the `pHYs` chunk (pixels per metre).

## Designs (`src/designs/`)

Each design is a module with

- `SPECS: &[ParamSpec]` – its knobs (key, label, hint, range, default, kind:
  float / int / toggle). The right panel builds its sliders from this list.
- `draw(pixmap, params, palette, seed)` – fills the whole pixmap.

Parameter values are stored per design in `Poster::params` (by `Design::id`
and the spec key), so switching designs keeps each one's settings. Missing or
out-of-range values fall back to the default / are clamped (`ParamSpec::clean`).

**Adding a design:** new module with `SPECS` and `draw`, a variant in
`Design` (plus `ALL`, `id`, `name`, `description`, `specs`, `draw`), a row in
the README table. `designs::tests` checks the specs and that it draws.

Shared helpers: `rng.rs` (SplitMix64 random numbers, Perlin noise with fbm),
`paint.rs` (colours, arcs, polygons, strokes), `palette.rs` (the palettes).

## Window (`src/app.rs`)

- Left panel: format, layout, text; export pinned to the bottom.
- Right panel: design cards (thumbnails rendered once per palette), seed,
  palettes, parameters.
- Middle: the preview. A worker thread renders it at the size it takes on
  screen (rounded to 64 px steps); queued requests are skipped so the newest
  settings win. A spinner shows while the shown preview is outdated.
- Settings persist through eframe storage (`Saved`, all fields
  `#[serde(default)]`).
- Keys: Space = new seed, ←/→ = previous/next seed, ⌘E = export.

## Tests and snapshots

`cargo test --release` runs the unit tests. The ignored `render_` tests in
`src/snapshot.rs` write every design in every layout and the app window
(through the software rasterizer in `src/offscreen.rs`) to
`target/snapshots/`.
