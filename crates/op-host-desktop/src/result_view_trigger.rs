//! The pure decision behind the post-generation 成品视图 trigger.
//!
//! `redraw.rs` observes the "everything is done" edge every frame; this
//! module keeps the open/shut logic testable without a window:

/// Whether the idle edge should open the result view for a Home-launched
/// generation that just finished.
///
/// Every "idle" flag must be true — the view must not take over the canvas
/// while a chat turn, the design orchestrator, or any sub-agent is still
/// running — and the generation must actually have produced at least one
/// top-level board. An empty outcome keeps the user on the canvas with the
/// (already cleared) intent dropped.
pub(crate) fn should_open_result_view(
    awaiting: bool,
    chat_idle: bool,
    design_idle: bool,
    subagents_idle: bool,
    board_count: usize,
) -> bool {
    awaiting && chat_idle && design_idle && subagents_idle && board_count > 0
}

#[cfg(test)]
mod tests {
    use super::should_open_result_view;

    #[test]
    fn opens_only_when_awaiting_fully_idle_and_non_empty() {
        assert!(should_open_result_view(true, true, true, true, 3));
    }

    #[test]
    fn a_still_running_turn_never_takes_over_the_canvas() {
        assert!(!should_open_result_view(true, false, true, true, 3));
        assert!(!should_open_result_view(true, true, false, true, 3));
        assert!(!should_open_result_view(true, true, true, false, 3));
    }

    #[test]
    fn an_unarmed_generation_or_empty_outcome_stays_on_the_canvas() {
        assert!(!should_open_result_view(false, true, true, true, 3));
        assert!(!should_open_result_view(true, true, true, true, 0));
        assert!(!should_open_result_view(false, false, false, false, 0));
    }
}
