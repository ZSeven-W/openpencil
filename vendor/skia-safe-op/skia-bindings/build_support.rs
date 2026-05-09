//! Build support functions for the Rust-Skia library.

pub mod binaries_config;
#[cfg(feature = "binary-cache")]
pub mod binary_cache;
pub mod cargo;
pub mod clang;
pub mod features;
pub mod platform;
pub mod skia;
pub mod skia_bindgen;
