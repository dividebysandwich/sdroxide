//! The sdroxide GUI: an egui app that talks to any [`sdroxide_types::RadioController`].
//!
//! Compiles native and for wasm32. All custom wgpu rendering is written to
//! WebGL2 downlevel limits (fragment-only, sampled textures + uniforms).

mod app;
pub mod chrome;
mod colormap;
mod download;
mod help;
#[cfg(feature = "remote")]
mod remote;
mod rf_paint;
mod sstv;
pub mod theme;
mod view;
mod waterfall_gpu;
mod widgets;

pub use app::SdroxideApp;
#[cfg(feature = "remote")]
pub use remote::{AudioBridge, RemoteController};

/// Wgpu access must go through this re-export so every crate agrees on the
/// wgpu version (project rule).
pub use eframe::egui_wgpu;
