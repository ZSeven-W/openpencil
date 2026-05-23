//! OpenPencil design diagnostics.
//!
//! Pure, wasm-clean detectors + fixes over the jian `PenDocument`. No I/O,
//! no editor state, no LLM. Ported from the TS `pen-ai-skills` diagnostics
//! layer; see `superpowers/specs/2026-05-17-pen-ai-skills-s1-design-lint.md`.

pub mod color;
pub mod detectors;
pub mod fixes;
pub mod issue;
pub mod node_mut;
pub mod node_util;

pub use color::{color_contrast, parse_hex_color, relative_luminance, Rgb};
pub use detectors::{
    detect_all, detect_edge_section_padding, detect_empty_paths, detect_excessive_frame_effects,
    detect_invisible_containers, detect_mixed_sibling_corner_radius, detect_mixed_sibling_padding,
    detect_sibling_inconsistencies, detect_stacked_horizontal_padding, detect_text_bg_contrast,
    detect_text_corner_radius, detect_text_effect, detect_text_explicit_heights,
    detect_text_stroke, detect_unexpected_rotation,
};
pub use fixes::{apply_fixes, detect_and_fix};
pub use issue::{FixProperty, FixReport, Issue, IssueCategory, IssueSeverity};
