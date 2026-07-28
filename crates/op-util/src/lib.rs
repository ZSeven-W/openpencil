//! Dependency-free leaf utilities shared across the OpenPencil workspace.
//!
//! This crate exists to single-source tiny helpers that were previously
//! copy-pasted across many crates (hex-color parsing had nine divergent
//! implementations; JSON/XML escaping three and four). It must stay
//! dependency-free and wasm32-clean — it is in the build graph of the
//! browser host via op-editor-core / op-editor-ui.

pub mod collab_id;
pub mod hex_color;
pub mod json_escape;
pub mod xml_escape;
