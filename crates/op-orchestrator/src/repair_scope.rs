//! Which ROOTS each repair tier may touch — the scope half of the tier split.
//!
//! [`crate::repair_tier`] answers "may this pass run at all". This module
//! answers "over which part of the document", and the two answers are not the
//! same question: append mode narrows the *scope* without suspending any tier.
//!
//! **The defect this exists to close.** Append mode scopes cleanup to the
//! roots the run inserted, so a continuation never restyles nodes the user
//! already had. That is right for the INTENT tier — rewriting a pre-existing
//! section's spacing, surfaces or palette on the strength of a heuristic is
//! exactly the damage the tier split was drawn to prevent. It is wrong for the
//! CONTRACT tier: those passes only repair what the resolved geometry PROVES,
//! and a screenshot showing two text blocks on the same pixels is no less
//! broken for sitting in a part of the page this run did not author.
//!
//! Worse, the old scoping degraded silently. `inserted_root_ids` can come back
//! empty (nothing inserted, or a buffered sink), and the empty slice made the
//! whole per-root cleanup block a no-op — `geometry_validate_and_fix`, the
//! text-collision echo and every other contract check with it. The comment at
//! the call site called that "a safe no-op"; it is safe only for the intent
//! tier. Measured on `0808-gm-2.op`, whose footer shipped with four text
//! blocks painted on top of each other while the fix loop that repairs exactly
//! that never ran over it.
//!
//! So: **intent scope = what this run inserted; contract scope = the whole
//! target root.** Stated once, here.

use crate::plan::OrchestratorPlan;
use crate::repair_summary::{CheckCategory, RepairCounter, RepairSummary};
use crate::types::DocSink;

/// Run the cleanup stage for an APPEND run.
///
/// `inserted_roots` are this run's own roots (post-remap ids); `target_roots`
/// is the frame the user appended INTO. The full cleanup driver runs over the
/// former — both tiers, unchanged behaviour — and the contract-tier passes are
/// then swept over the latter.
///
/// The sweep is deliberately a re-run rather than a carve-out of the driver:
/// every contract pass here is idempotent (each loops until its detectors go
/// quiet), so covering the inserted roots twice costs a no-op pass and keeps
/// this module from having to mirror the driver's ordering.
pub(crate) fn finalize_appended_design(
    sink: &mut dyn DocSink,
    plan: &OrchestratorPlan,
    inserted_roots: &[&str],
    target_roots: &[&str],
    summary: &mut RepairSummary,
) {
    // `inserted_roots` are this run's own (the doc comment above says so, and
    // the empty-slice case is already a no-op), so the intent-tier passes may
    // key off their geometry. `target_roots` — the user's existing frame —
    // never reaches this driver: `contract_sweep` below runs a named
    // contract-tier pair over it instead, which is what keeps an appended-into
    // board from being re-composed.
    crate::cleanup::finalize_design_with_summary_and_policy(
        sink,
        plan,
        inserted_roots,
        summary,
        crate::cleanup::CleanupPolicy {
            roots_are_run_output: true,
            ..Default::default()
        },
    );
    let swept = contract_sweep(sink, target_roots, summary);
    if swept {
        summary.note(scope_note(inserted_roots.len(), target_roots.len()));
    }
}

/// Finalize a document imported from an AUTHORED source (a live website):
/// the contract tier only, over every top-level root.
///
/// The loop finalizer's other passes are right for model output and wrong
/// here: they re-weight headings, add entrance motion, widen hero gaps and
/// restyle surfaces — a judgement on the author's behalf about a page the
/// author already shipped, and one that also breaks the "identical copies"
/// component recognition relies on. What a screenshot of the import proves
/// broken (text on its own background colour, overflow, collapsed boxes) is
/// still repaired. The skip is recorded as a note naming `source`.
pub fn finalize_authored_import(
    state: &mut op_editor_core::EditorState,
    source: &str,
) -> RepairSummary {
    use op_editor_core::PenNodeExt;
    let mut summary = RepairSummary::default();
    let roots: Vec<String> = state
        .active_children()
        .iter()
        .map(|node| node.id_str().to_string())
        .collect();
    let root_refs: Vec<&str> = roots.iter().map(String::as_str).collect();
    let mut sink = crate::loop_finalize::StateDocSink { state };
    if contract_sweep(&mut sink, &root_refs, &mut summary) {
        summary.note(format!(
            "intent-tier passes skipped (authored import: {source}) — the site's own \
             spacing, type, surfaces and palette kept as designed; contract-tier checks ran"
        ));
    }
    summary
}

/// The ledger line. A narrowed scope is a DECISION, and a reader comparing two
/// runs' credentials must be able to tell it from a gap — especially the
/// zero-inserted-roots case, whose only other trace is the absence of two
/// categories nobody would think to look for.
fn scope_note(inserted: usize, targets: usize) -> String {
    let roots = if targets == 1 { "root" } else { "roots" };
    if inserted == 0 {
        format!(
            "cleanup scoped to 0 inserted roots — intent-tier passes ran on nothing; \
             contract-tier checks ran on the full target {roots}"
        )
    } else {
        format!(
            "intent-tier passes scoped to the {inserted} appended root(s) so pre-existing \
             content keeps its authored styling; contract-tier checks ran on the full \
             target {roots}"
        )
    }
}

/// Contract-tier passes over `roots`. Returns whether the sweep ran at all.
///
/// The membership is [`crate::repair_tier::TieredPass`]'s Contract arm, minus
/// the one pass that is not a cleanup-driver pass: `HorizontalOverflow` runs
/// per-subtask in `role_layout_post_pass`, on the generated subtree BEFORE it
/// is inserted, so append scoping never took it away in the first place.
/// `TextCollision` is detect-only by design — it reports into the geometry
/// echo, and `geometry_validate_and_fix` below is what repairs the geometry
/// that produces those collisions.
fn contract_sweep(sink: &mut dyn DocSink, roots: &[&str], summary: &mut RepairSummary) -> bool {
    if roots.is_empty() {
        return false;
    }
    let mut counter = RepairCounter::new();
    let mut counting = counter.wrap(sink);
    let sink: &mut dyn DocSink = &mut counting;
    for root_id in roots {
        crate::text_contrast_repair::repair_text_contrast(sink, root_id);
        counter.checkpoint(summary, CheckCategory::Layout, "text_contrast_repair");
        crate::geometry_validation::geometry_validate_and_fix(sink, root_id);
        counter.checkpoint(
            summary,
            CheckCategory::Overflow,
            "geometry_validate_and_fix",
        );
    }
    true
}

#[cfg(test)]
#[path = "repair_scope_tests.rs"]
mod tests;
