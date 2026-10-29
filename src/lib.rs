//! A pyrotechnic screensaver for the terminal.
//!
//! The binary is a thin shell around this library: [`app::App`] owns the
//! simulation and the renderer, [`snapshot`] can rasterise a frame headlessly.

pub mod app;
pub mod fireworks;
pub mod fx;
pub mod input;
pub mod palette;
pub mod render;
pub mod sky;
pub mod snapshot;
pub mod ui;
