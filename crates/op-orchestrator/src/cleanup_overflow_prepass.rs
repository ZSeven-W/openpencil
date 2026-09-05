//! Overflow pre-pass of the cleanup driver: the two fact-based repairs that
//! run BEFORE the geometry-validation loop so its card-overflow clip fallback
//! only ever sees what a move or a shrink cannot fix.

use crate::design_type::DesignForm;
use crate::repair_summary::{CheckCategory, RepairCounter, RepairSummary};
use crate::types::DocSink;

pub(super) fn run_overflow_prepass(
    sink: &mut dyn DocSink,
    root_id: &str,
    summary: &mut RepairSummary,
    counter: &mut RepairCounter,
) {
    // Absolutely-pinned, fixed-size controls that poke out of a
    // `layout: "none"` parent are shifted back INSIDE the parent: the clip
    // fallback would otherwise crop the control (a real run chopped the right
    // half off a 44x44 "locate me" button pinned at x=307 in a 327px map).
    crate::geometry_validation::clamp_absolute_children_into_parent(sink, root_id);
    counter.checkpoint(summary, CheckCategory::Overflow, "absolute-child-clamp");

    // A single-line text leaf measured wider than its nearest clipping or
    // fixed-width ancestor is shrunk proportionally. Screens only: deck and
    // card boards keep their headline size and are wrapped by the later
    // `board-text-wrap` pass (semantic repair before fallback), so this
    // shrink must not take those cases first.
    let form = crate::geometry_validation::root_design_form(sink.state(), root_id);
    if matches!(form, DesignForm::MobileScreen | DesignForm::Page) {
        crate::geometry_validation::repair_text_fit(sink, root_id);
    }
    counter.checkpoint(summary, CheckCategory::Overflow, "text-fit");
}
