//! Tessera – generative posters.
//!
//! The poster is drawn on the CPU with tiny-skia (`poster::render`), so the
//! preview in the window and the exported PNG come from the same code and
//! only differ in resolution.

pub mod app;
pub mod designs;
pub mod export;
#[cfg(test)]
mod offscreen;
pub mod paint;
pub mod palette;
pub mod poster;
pub mod rng;
#[cfg(test)]
mod snapshot;
pub mod text;
