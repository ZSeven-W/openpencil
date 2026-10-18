//! Keep requested viewports separate from adaptive, content-height mobile boards.
use super::*;

fn fixed_output(policy: CleanupPolicy) -> bool {
    policy.preserve_requested_root_height && policy.roots_are_run_output
}

pub(super) fn prepare_status(sink: &mut dyn DocSink, root_id: &str, policy: CleanupPolicy) {
    if fixed_output(policy) {
        // This moves only existing direct chrome. Missing bars and unrelated
        // screens remain the responsibility of final status enforcement.
        finalize_enforce_status_bar::anchor_existing_status_bar_first(sink, root_id);
    }
}

pub(super) fn reflow_root(sink: &mut dyn DocSink, root_id: &str, policy: CleanupPolicy) {
    if fixed_output(policy) {
        crate::mobile_reflow::repair_mobile_trailing_nav_reflow_with_fixed_root_in_sink(
            sink, root_id,
        );
    } else {
        crate::mobile_reflow::repair_mobile_trailing_nav_reflow_for_root_in_sink(sink, root_id);
    }
}

pub(super) fn reflow_remaining(
    sink: &mut dyn DocSink,
    settled_roots: &[String],
    policy: CleanupPolicy,
) {
    if !fixed_output(policy) {
        crate::mobile_reflow::repair_mobile_trailing_nav_reflow_in_sink(sink);
        return;
    }
    // Whole-document catch-up must not grow this run's fixed viewports, while
    // preserving the previous adaptive behavior for roots outside this run.
    let roots: Vec<_> = sink
        .state()
        .active_children()
        .iter()
        .map(PenNodeExt::id_str)
        .map(str::to_owned)
        .collect();
    for root_id in roots {
        if settled_roots.contains(&root_id) {
            crate::mobile_reflow::repair_mobile_trailing_nav_reflow_with_fixed_root_in_sink(
                sink, &root_id,
            );
        } else {
            crate::mobile_reflow::repair_mobile_trailing_nav_reflow_for_root_in_sink(
                sink, &root_id,
            );
        }
    }
}
