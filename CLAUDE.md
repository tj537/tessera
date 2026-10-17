# Tessera – notes for working on the code

Generative posters: pick a design, tweak parameters, export PNG. Rust + eframe/egui 0.31, posters drawn with tiny-skia. Architecture: `docs/ARCHITECTURE.md`; user-facing docs: `README.md`. Keep both up to date with every feature.

## Commands

| Command | What it does |
|---|---|
| `cargo test --release` | all tests (fast) |
| `cargo test` | the same in debug, like CI – tiny-skia has debug-only assertions (thin hairlines, AA rects outside the image), so run this before pushing drawing changes |
| `cargo clippy --release --all-targets` | must stay free of warnings |
| `cargo fmt` | CI checks `cargo fmt --check` |
| `cargo test --release render_ -- --ignored` | renders every design in every layout and the app window to `target/snapshots/*.png` – look at them after visual changes |
| `./scripts/build-mac.sh --install` | builds `dist/Tessera.app` and replaces `/Applications/Tessera.app` |
| `./scripts/build-mac.sh --zip` | builds `dist/Tessera-macos-<cpu>.zip` (the release asset is built on an Apple Silicon runner: `arm64`) |
| `cargo run --release --example make_icon -- --assets` | regenerates `assets/icon.png` and `assets/icon.ico` (the icon is drawn in code) |

Commit only when `cargo test` exits 0 (check the exit code; don't hide it behind a pipe).

## Conventions

- **Language:** UI, README, docs, commit messages and code comments in English.
- **egui pins:** egui/eframe 0.31, `egui-phosphor` 0.9 and `epaint_default_fonts` 0.31 go together.
- **Visual checks:** verify UI with the offscreen renderer (`src/offscreen.rs`, tests in `src/snapshot.rs`, `TesseraApp::for_snapshot`). Don't take full-screen screenshots of the user's desktop.
- **Resolution independence:** designs size everything relative to the pixmap and draw random numbers in a fixed order, so preview and export match. Never make the number of random draws depend on pixel size.
- **New `Poster` / settings fields:** the structs are `#[serde(default)]`, so older saved state still loads – keep it that way.
- **Line endings:** `.gitattributes` forces LF.

`ci.yml` runs fmt, clippy and tests on macOS, Windows and Linux for every push.

## Releases

Asset names never change (`Tessera-macos-arm64.zip`, `Tessera-windows-x64.zip`) so `releases/latest/download/…` links keep working.

1. Raise `version` in `Cargo.toml`, commit.
2. Write `.github/release-notes/vX.Y.Z.md` (English).
3. `git tag -a vX.Y.Z -m "Tessera X.Y.Z" && git push origin vX.Y.Z` – `.github/workflows/release.yml` builds both platforms, adds `SHA256SUMS.txt` and publishes the release.
