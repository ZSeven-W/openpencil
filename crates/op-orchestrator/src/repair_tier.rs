//! Which repair passes are allowed to run, and on whose authority.
//!
//! Every deterministic pass in this crate answers one of two different
//! questions, and the difference decides whether it may touch an authored
//! design:
//!
//! - **[`RepairTier::Contract`]** — the pass proves a *fact* about the tree
//!   and repairs it. Text overflowing its container, two nodes occupying the
//!   same rect, a 1.2:1 text-on-background contrast, a resolved rect outside
//!   its parent. A human author cannot have meant these, and a model filling
//!   content into an authored shell still commits them, so they run always.
//! - **[`RepairTier::Intent`]** — the pass makes an *aesthetic or semantic
//!   judgement* on the author's behalf. "This wrapper's inset is redundant."
//!   "This surface repeats its parent's colour." "This white card fill is
//!   decorative." "This hex should have been a token." Each of those is a
//!   correct reading of typical weak-model output and a wrong reading of a
//!   design somebody meant.
//!
//! The criterion, in one sentence: **a pass is Contract when a screenshot of
//! the unrepaired tree would show something no author could have intended;
//! otherwise it is Intent.** `0808-gm-1` is the case that forced the split —
//! the intent-tier passes read an authored 40–80px wrapper inset as a double
//! inset and an authored dark band as a redundant surface, and stripped both.
//!
//! Intent-tier passes are skipped for a document with template provenance
//! (see [`crate::template_provenance`]): the template's visual system is
//! human-authored and audited, so it is the specification, not the defect.
//! A document imported from a live website counts as the same kind of
//! authored input (`TemplateEvidence::ImportedSite`).
//! Contract-tier passes still run — a model filling text into an audited
//! template overflows a box exactly as readily as it does anywhere else.
//!
//! The skip is recorded, never silent: [`note_intent_tier_skip`] puts a line
//! in the repair ledger so a user reading it can tell a decision from an
//! omission.

use op_editor_core::EditorState;

use crate::repair_summary::RepairSummary;
use crate::template_provenance::{template_provenance, TemplateProvenance};

/// The two kinds of authority a repair pass claims. See the module doc for the
/// judging criterion — it is stated once, here, and nowhere else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairTier {
    /// Repairs a provable defect. Always runs.
    Contract,
    /// Makes a judgement on the author's behalf. Skipped for authored input.
    Intent,
}

/// Every pass whose tier has been classified, with the classification itself
/// as the documentation.
///
/// This enum is the annotation the lead asked for: a pass appears here because
/// somebody decided which side of the criterion above it falls on, and the
/// `tier` arm below is that decision in a form the compiler checks. Adding a
/// gate at a call site means naming the pass here first.
///
/// The Contract entries are not gated by anything — they are listed so this
/// type carries both halves of the split rather than reading as a list of
/// suppressions, and so the criterion has worked examples on both sides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TieredPass {
    // ── Intent ──────────────────────────────────────────────────────────
    /// `loop_finalize::fix_theme_variable_polarity` — rewrites variable values
    /// across theme slots to match a polarity contract inferred from the first
    /// root's background. A template ships the palette it was designed with.
    ThemeVariablePolarity,
    /// `spacing_repair::strip_wrapper_double_inset` — drops a transparent
    /// wrapper's padding when its parent already pads. An authored board's
    /// 40–80px inset reads as exactly that pattern.
    WrapperDoubleInset,
    /// `role_post_pass::fix_structural_wrapper_transparency` — strips a
    /// near-white fill off a section/header wrapper. A light template's
    /// authored card surfaces are near-white on purpose.
    StructuralWrapperTransparency,
    /// `role_post_pass::enforce_surface_color_discipline` — recolours a
    /// surface that repeats its parent's token or misuses a state colour.
    /// A deliberate tonal band is indistinguishable from that.
    SurfaceColorDiscipline,
    /// `variable_binding::bind_generated_color_variables` — rewrites authored
    /// hex into design tokens. A template already names its own palette, and
    /// rebinding renames the author's system into the generator's.
    VariableBinding,
    /// `tree_heuristics::apply_tree_heuristics` — post-streaming fill /
    /// decoration heuristics (redundant section fill strip, nested-card
    /// decoration strip, invisible-band fill).
    TreeHeuristics,
    /// `hero_bleed::enforce` — pulls a hero's media to the page edges. An
    /// authored inset hero is a composition choice, not a defect.
    HeroBleed,
    /// `motion_recipes::apply` — adds entrance animations to sections and
    /// cards. It staggers identical cards apart, and authored work ships the
    /// motion it was designed with.
    MotionRecipes,
    /// `cleanup_typography::repair_overbold_text_hierarchy` — lowers heading
    /// weights to rebuild a hierarchy. An imported site's 700-weight type is
    /// its brand, not an accident.
    OverboldTextHierarchy,

    // ── Contract ────────────────────────────────────────────────────────
    /// `geometry_validation::geometry_validate_and_fix` — runs the real layout
    /// engine and repairs what the resolved rects prove wrong.
    GeometryValidation,
    /// `text_collision` — two text rects overlapping is never a composition.
    TextCollision,
    /// `text_contrast_repair::repair_text_contrast` — text at ~1:1 against its
    /// own background is not styled, it is missing.
    TextContrast,
    /// `role_layout_post_pass::fix_horizontal_overflow` — a row wider than the
    /// viewport it sits in.
    HorizontalOverflow,
}

impl TieredPass {
    /// Which authority this pass claims. The single classification table.
    pub fn tier(self) -> RepairTier {
        match self {
            TieredPass::ThemeVariablePolarity
            | TieredPass::WrapperDoubleInset
            | TieredPass::StructuralWrapperTransparency
            | TieredPass::SurfaceColorDiscipline
            | TieredPass::VariableBinding
            | TieredPass::TreeHeuristics
            | TieredPass::HeroBleed
            | TieredPass::MotionRecipes
            | TieredPass::OverboldTextHierarchy => RepairTier::Intent,
            TieredPass::GeometryValidation
            | TieredPass::TextCollision
            | TieredPass::TextContrast
            | TieredPass::HorizontalOverflow => RepairTier::Contract,
        }
    }
}

/// Which tiers may run for one document, and why not, when a tier may not.
///
/// Resolved from [`EditorState`] rather than passed in, so the orchestrator
/// path (`DocSink::state()`) and the agentic-loop path (the live state) reach
/// the same answer without either needing a new parameter — and so neither can
/// be wired up while the other silently is not.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RepairTierPolicy {
    /// `None` when the intent tier runs; `Some(provenance)` when it is
    /// skipped and that is the authored input it is deferring to.
    deferring_to: Option<TemplateProvenance>,
}

impl RepairTierPolicy {
    /// Both tiers run — ordinary generated output with no authored input.
    pub fn all() -> Self {
        Self { deferring_to: None }
    }

    /// The policy for `state`.
    pub fn for_document(state: &EditorState) -> Self {
        Self {
            deferring_to: template_provenance(state),
        }
    }

    /// Whether `tier` may run under this policy.
    pub fn runs(&self, tier: RepairTier) -> bool {
        match tier {
            RepairTier::Contract => true,
            RepairTier::Intent => self.deferring_to.is_none(),
        }
    }

    /// Whether `pass` may run — the form call sites use, so the tier lookup
    /// and the gate cannot drift apart.
    pub fn runs_pass(&self, pass: TieredPass) -> bool {
        self.runs(pass.tier())
    }

    /// The authored input the intent tier is deferring to, when it is.
    pub fn deferring_to(&self) -> Option<&TemplateProvenance> {
        self.deferring_to.as_ref()
    }

    /// The ledger line for this policy, or `None` when nothing was skipped.
    pub fn intent_skip_note(&self) -> Option<String> {
        self.deferring_to.as_ref().map(|provenance| {
            format!(
                "intent-tier passes skipped (template provenance: {}) — \
                 authored spacing, surfaces and palette kept as designed; \
                 contract-tier checks still ran",
                provenance.describe()
            )
        })
    }
}

/// Record the skip in `summary` and the log, so "did not run" reads as a
/// decision rather than as a gap. No-op when the intent tier ran.
///
/// Called once per cleanup run, from the first gated site in the driver
/// (`cleanup_root_transform::intent_theme_variable_polarity`), which is reached
/// on every path through it.
pub(crate) fn note_intent_tier_skip(summary: &mut RepairSummary, policy: &RepairTierPolicy) {
    let Some(note) = policy.intent_skip_note() else {
        return;
    };
    tracing::info!(
        provenance = %policy
            .deferring_to()
            .map(TemplateProvenance::describe)
            .unwrap_or_default(),
        "intent-tier repair passes skipped for authored template input"
    );
    summary.note(note);
}

#[cfg(test)]
#[path = "repair_tier_tests.rs"]
mod tests;
