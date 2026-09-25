//! Whether the document a repair pass is about to edit came from a shipped
//! scene template.
//!
//! A template is not model output. Its padding, its dark bands, its variable
//! table and its surface hierarchy were authored by a person and audited (the
//! `templates/step0` set ships at zero lint findings), so they are the design
//! *specification* for anything generated from them — not a defect the
//! deterministic passes are entitled to second-guess. `0808-gm-1` is the case
//! that made this concrete: the intent-tier passes read an authored 40–80px
//! wrapper inset as a double inset and an authored dark band as a redundant
//! surface, and stripped both.
//!
//! ## The two doors, and why the signal has to cover both
//!
//! - **Add to canvas** (`scene_template_append`) puts the template's boards
//!   into the user's document and namespaces every variable the template
//!   declares as `"<template-id>--<name>"`. A later generation turn appends
//!   beside those boards, and the doc-global passes in `cleanup` (the variable
//!   polarity fix, the whole-doc chrome passes) reach them.
//! - **Generate from this** (`scene_template_generate`) resets the canvas to a
//!   blank starter and pins the template's style guide, so nothing of the
//!   template is on the tree — only the generated imitation of it. There is
//!   nothing to detect in the tree, so the signal for that door is the pin
//!   itself, carried on `editor_ui.scene_template_center.generate_basis`.
//!
//! Both live on [`EditorState`], which is what every repair path already has
//! (`DocSink::state()` on the orchestrator path, the live state on the agentic
//! loop path) — so neither path needs a new parameter and neither can be wired
//! up while the other silently is not. That is the same reason
//! [`crate::design_type::classify_root_form`] reads the artboard rather than
//! the plan: the agentic loop has no plan to read.
//!
//! ## The third door: an imported website
//!
//! Studio Home's "import this website" produces a document that is authored
//! input in exactly the same sense — somebody designed and shipped that page —
//! and `finalize_authored_import` already runs only the contract tier over it.
//! What made a LATER AI edit restyle it anyway (heading weights, entrance
//! motion, hero spacing) was that nothing on the document remembered where it
//! came from. The import now records its sanitized origin on
//! `editor_ui.home.imported_from`, persisted as `editorMeta.importedFrom`, and
//! [`TemplateEvidence::ImportedSite`] reads it back. The "template id" of that
//! provenance is the origin URL: there is no catalogue entry to resolve, and
//! the origin is what a user reading the ledger recognises.
//!
//! ## Exact, never heuristic
//!
//! Every evidence branch below resolves against
//! [`scene_template_catalogue`](op_editor_core::scene_template_catalog::scene_template_catalogue).
//! A variable named `my--thing` or a style pinned by hand from the style menu
//! is NOT template provenance. A "looks like a template" name match would be
//! the eighth inline judgement call this crate does not need. The imported-site
//! branch is exact in the same way: it is the recorded origin or nothing — a
//! document that merely looks like a website is ordinary generated output.

use op_editor_core::scene_template_catalog::{scene_template_by_id, scene_template_catalogue};
use op_editor_core::EditorState;

/// The separator `scene_template_append::variable_renames` puts between a
/// template id and the variable name it namespaces. Kept as a local constant
/// rather than imported so this module states the contract it matches on;
/// `namespaced_variable_names_are_the_contract_this_matches` locks the two
/// together against the real append path.
const TEMPLATE_VARIABLE_SEPARATOR: &str = "--";

/// How a document was found to have come from a template.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateEvidence {
    /// The document's variable table carries a template's namespaced palette,
    /// so the template's own boards are on this page.
    NamespacedVariables,
    /// The Asset Center's generate row is working from this template: its
    /// style guide is pinned and the generation is asked to reproduce it.
    GenerateBasis,
    /// The document was imported from a live website (Studio Home's "import
    /// this website"); the provenance id is the sanitized origin URL.
    ImportedSite,
}

impl TemplateEvidence {
    /// Stable token for the repair ledger and logs.
    pub fn key(self) -> &'static str {
        match self {
            TemplateEvidence::NamespacedVariables => "namespaced-variables",
            TemplateEvidence::GenerateBasis => "generate-basis",
            TemplateEvidence::ImportedSite => "imported-site",
        }
    }
}

/// A resolved template origin: which template, and what proved it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateProvenance {
    pub template_id: String,
    pub evidence: TemplateEvidence,
}

impl TemplateProvenance {
    /// One-line reason, for the ledger note and the INFO log.
    pub fn describe(&self) -> String {
        format!("{} via {}", self.template_id, self.evidence.key())
    }
}

/// Resolve `state`'s template origin, or `None` when it has none.
///
/// The generate basis is checked first: when both hold, the basis is the more
/// specific fact (this turn is *about* that template), and it is the one a
/// user reading the ledger would recognise. A shipped template's boards on the
/// page come next, then the website the document was imported from — every
/// branch defers the same intent tier, so the order only picks the ledger
/// line.
pub fn template_provenance(state: &EditorState) -> Option<TemplateProvenance> {
    if let Some(id) = generate_basis_template_id(state) {
        return Some(TemplateProvenance {
            template_id: id,
            evidence: TemplateEvidence::GenerateBasis,
        });
    }
    if let Some(id) = namespaced_variable_template_id(state) {
        return Some(TemplateProvenance {
            template_id: id,
            evidence: TemplateEvidence::NamespacedVariables,
        });
    }
    imported_site_origin(state).map(|origin| TemplateProvenance {
        template_id: origin,
        evidence: TemplateEvidence::ImportedSite,
    })
}

/// The website the open document was imported from, re-sanitized so a value
/// that reached the state by any path other than the loader (a host setting it
/// directly, a test) is held to the same scheme + host + path contract.
fn imported_site_origin(state: &EditorState) -> Option<String> {
    state
        .editor_ui
        .home
        .imported_from
        .as_deref()
        .and_then(op_editor_core::sanitize_import_origin)
}

/// The template the generate row is working from, when it resolves in the
/// catalogue. An id the catalogue no longer knows is not provenance — it is a
/// stale label, exactly as the basis chip treats it.
fn generate_basis_template_id(state: &EditorState) -> Option<String> {
    let id = state
        .editor_ui
        .scene_template_center
        .generate_basis
        .as_deref()?;
    scene_template_by_id(id).map(|template| template.id.clone())
}

/// The first shipped template whose namespaced palette is in the document's
/// variable table.
///
/// Matching on the `"<id>--"` prefix rather than on the whole name is what
/// makes this survive a template growing a variable: the namespace is the
/// evidence, the individual names are not.
fn namespaced_variable_template_id(state: &EditorState) -> Option<String> {
    let variables = state.doc.variables.as_ref()?;
    if variables.is_empty() {
        return None;
    }
    scene_template_catalogue()
        .iter()
        .find(|template| {
            let prefix = format!("{}{TEMPLATE_VARIABLE_SEPARATOR}", template.id);
            variables.keys().any(|name| name.starts_with(&prefix))
        })
        .map(|template| template.id.clone())
}

#[cfg(test)]
#[path = "template_provenance_tests.rs"]
mod tests;
