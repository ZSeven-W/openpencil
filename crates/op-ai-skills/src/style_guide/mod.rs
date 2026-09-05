//! Style guides — port of `pen-ai-skills/src/style-guide`.
//!
//! A style guide is a markdown file (frontmatter + prose) describing a
//! visual aesthetic. This module loads the embedded `style-guides/`
//! corpus, selects one by name or tag score, and extracts structured
//! colour / typography / radius values for style-switching.

//! The catalogue has two halves — the corpus baked in here and the guides a
//! user imported at runtime ([`user_registry`]). Resolve a name through
//! [`find_style_guide`] rather than [`style_guide_registry`] whenever the name
//! could have come from a user: pins, plans, and sub-agent specs all can.

pub mod card;
pub mod design_md_import;
pub mod key_aesthetics;
pub mod loader;
pub mod mapping;
pub mod parser;
pub mod summary;
pub mod token_table;
pub mod types;
pub mod user_registry;

#[cfg(test)]
mod audit_tests;

pub use card::{style_guide_card, StyleGuideCard, STYLE_CARD_SWATCH_CAP};
pub use design_md_import::{
    parse_design_md, slugify, DesignMdImportError, ImportedDesignMd, MAX_DESIGN_MD_BYTES,
};
pub use key_aesthetics::key_aesthetics;
pub use loader::{parse_style_guide_file, select_style_guide, style_guide_registry, SelectOptions};
pub use mapping::{build_style_mapping, ColorReplacement, PropertyReplacement, RadiusReplacement};
pub use parser::{extract_style_guide_values, StyleGuideValues};
pub use summary::{style_guide_summary, StyleGuideSummary};
pub use token_table::{sample_palette, PaletteSample};
pub use types::{ParsedStyleGuide, Platform, StyleGuideMeta, STYLE_GUIDE_TAGS};
pub use user_registry::{
    find_style_guide, has_user_style_guides, import_design_md, load_user_style_guide,
    remove_user_style_guide, set_user_style_guides, user_style_guides, StyleGuideRef,
    UserStyleGuide, USER_STYLE_ID_PREFIX,
};
