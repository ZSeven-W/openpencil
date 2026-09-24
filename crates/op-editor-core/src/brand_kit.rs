//! Applying a brand kit to the document: its design variables, its theme
//! axis, and (when it owns it) the document's design.md — as ONE undo step.
//!
//! The kit's variables use the bundled semantic palette's names, so every
//! node generated against that palette (`$--primary`, `$--background` …)
//! picks the brand up at paint time, and the orchestrator's palette seed
//! (existing variables win) keeps it for the next generation. Swapping a
//! brand is applying another kit: the variable values change, the bound
//! nodes do not.
//!
//! The extraction itself lives in the native-only `op-brand` crate; this
//! module only knows the payload, so the wasm bundle carries no parser.

use std::collections::BTreeMap;

use jian_ops_schema::variable::VariableDefinition;
use jian_ops_schema::DesignMdSpec;

use crate::command::EditorCommand;
use crate::state::EditorState;

/// Marker line a kit-written design.md starts with (`op-brand` writes the
/// same literal). A design.md without it is the user's own brief and is
/// never replaced by a kit.
pub const BRAND_KIT_MARKER: &str = "<!-- openpencil:brand-kit -->";

/// What applying a kit writes. Hosts build it from an `op_brand::BrandKit`
/// (`variables()`, `themes()`, `design_md()`); `label` / `swatches` are
/// display-only (chips, reports).
#[derive(Debug, Clone, PartialEq)]
pub struct BrandKitPayload {
    pub label: String,
    pub swatches: Vec<String>,
    pub variables: BTreeMap<String, VariableDefinition>,
    pub themes: BTreeMap<String, Vec<String>>,
    pub design_md: Option<DesignMdSpec>,
}

/// Whether a design.md was written by a kit (and may be replaced by one).
pub fn design_md_is_brand_kit(spec: &DesignMdSpec) -> bool {
    spec.raw.trim_start().starts_with(BRAND_KIT_MARKER)
}

impl BrandKitPayload {
    /// Whether this payload's design.md should land on a document whose
    /// current brief is `current`: only over nothing or over a kit's own.
    pub fn writes_design_md(&self, current: Option<&DesignMdSpec>) -> bool {
        self.design_md.is_some() && current.is_none_or(design_md_is_brand_kit)
    }

    /// The single command a host applier runs (MCP `brand_extract` with
    /// `apply`): variables + theme axis merged (kit values win), design.md
    /// set when [`Self::writes_design_md`]. A `Batch` is one undo step.
    pub fn to_command(&self, current_design_md: Option<&DesignMdSpec>) -> EditorCommand {
        let mut commands = vec![EditorCommand::MergeThemePreset {
            variables: self.variables.clone(),
            themes: self.themes.clone(),
        }];
        if self.writes_design_md(current_design_md) {
            if let Some(spec) = &self.design_md {
                commands.push(EditorCommand::SetDesignMd {
                    spec: Box::new(spec.clone()),
                });
            }
        }
        EditorCommand::Batch { commands }
    }
}

impl EditorState {
    /// Apply `kit` as one undoable step. Returns whether the document
    /// changed. The `Batch` path records the single pre-apply snapshot
    /// itself and rolls everything back on failure, so the local apply
    /// and a host's MCP applier land identical history.
    pub fn apply_brand_kit(&mut self, kit: &BrandKitPayload) -> bool {
        let command = kit.to_command(self.doc.design_md.as_ref());
        self.apply(command)
    }
}

#[cfg(test)]
#[path = "brand_kit_tests.rs"]
mod tests;
