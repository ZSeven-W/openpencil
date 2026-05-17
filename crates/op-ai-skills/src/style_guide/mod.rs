//! Style guides — port of `pen-ai-skills/src/style-guide`.
//!
//! A style guide is a markdown file (frontmatter + prose) describing a
//! visual aesthetic. This module loads the embedded `style-guides/`
//! corpus, selects one by name or tag score, and extracts structured
//! colour / typography / radius values for style-switching.

pub mod loader;
pub mod mapping;
pub mod parser;
pub mod types;

pub use loader::{parse_style_guide_file, select_style_guide, style_guide_registry, SelectOptions};
pub use mapping::{build_style_mapping, ColorReplacement, PropertyReplacement, RadiusReplacement};
pub use parser::{extract_style_guide_values, StyleGuideValues};
pub use types::{ParsedStyleGuide, Platform, StyleGuideMeta, STYLE_GUIDE_TAGS};
