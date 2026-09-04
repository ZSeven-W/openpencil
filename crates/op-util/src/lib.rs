//! Dependency-free leaf utilities shared across the OpenPencil workspace.
//!
//! This crate exists to single-source tiny helpers that were previously
//! copy-pasted across many crates (hex-color parsing had nine divergent
//! implementations; JSON/XML escaping three and four; image-header sizing
//! lived in the editor UI). It must stay
//! dependency-free and wasm32-clean — it is in the build graph of the
//! browser host via op-editor-core / op-editor-ui.

pub mod cli_output;
pub mod collab_id;
pub mod hex_color;
pub mod image_dimensions;
pub mod json_escape;
pub mod shader_preset;
pub mod xml_escape;

pub use image_dimensions::{
    encoded_image_dimensions, encoded_svg_intrinsic_metadata, encoded_svg_view_box_ratio,
    SvgIntrinsicMetadata, MAX_INTRINSIC_IMAGE_EDGE,
};
