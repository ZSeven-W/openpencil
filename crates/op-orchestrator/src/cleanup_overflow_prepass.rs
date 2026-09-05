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
    // A `fill_container` child of a `layout: "none"` stack that is also
    // pinned at a positive x/y gets the stack's FULL size from jian, so the
    // offset pushes it past the right/bottom edge (a real run clipped the
    // taxi search card at x=24 in the 375-wide "搜索区堆叠"). Rewrite the
    // keyword to the parent's resolved size minus twice the offset BEFORE the
    // clamp, so the clamp and the clip fallback never see the overflow.
    crate::geometry_validation::repair_none_stack_insets(sink, root_id);
    counter.checkpoint(summary, CheckCategory::Overflow, "none-stack-inset");

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

    // Phone controls whose resolved touch area is below the mobile floor get a
    // numeric height before the geometry loop runs. This is deliberately a
    // separate layout checkpoint because the pass is a target-size repair, not
    // overflow handling.
    crate::geometry_validation::repair_touch_target_floor(sink, root_id);
    counter.checkpoint(summary, CheckCategory::Layout, "touch-target-floor");

    // A painted, rounded card with no authored padding lets its content sit
    // flush against the card edge (a real run left the fitness exercise
    // rows' thumbnails touching the card's left edge). Give it the standard
    // card inset [12, 16] — a layout repair, not overflow handling, so it is
    // checkpointed under Layout.
    crate::geometry_validation::repair_card_inner_padding(sink, root_id);
    counter.checkpoint(summary, CheckCategory::Layout, "card-inner-padding");
}
