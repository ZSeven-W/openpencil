//! Route selection for the web standard turn: the classifier's verdict, the
//! route a browser may pin on the turn, and the starter-frame housekeeping a
//! new design needs.
//!
//! Split out of `web_chat_standard.rs` to keep that module under the
//! 800-line cap.

use op_editor_core::{EditorState, LaunchRoute};
use serde_json::Value;

use crate::chat_intent::DesignIntent;
use crate::web_canvas_server::WebCanvasState;

/// The route the browser pinned on the turn (`"launchRoute"`), mirroring the
/// desktop launcher's `chat.launch_route`: Studio Home briefs are whole
/// designs (`"orchestrator"`), and the one-click draft's refine edits the
/// selected boards in place (`"refine"`), and the directions toggle asks
/// for side-by-side directions (`"variants"` + `"variantCount"`, clamped to
/// the desktop's range; absent means the default count). Anything else is
/// the ordinary classified route.
pub(super) fn parse_launch_route(obj: &serde_json::Map<String, Value>) -> LaunchRoute {
    match obj.get("launchRoute").and_then(Value::as_str) {
        Some("orchestrator") => LaunchRoute::Orchestrator,
        Some("refine") => LaunchRoute::Refine,
        Some("variants") => {
            let count = obj
                .get("variantCount")
                .and_then(Value::as_u64)
                .map_or(op_editor_core::DEFAULT_VARIANT_COUNT, |n| {
                    n.min(u64::from(u8::MAX)) as u8
                });
            LaunchRoute::Variants(op_editor_core::clamp_variant_count(count))
        }
        _ => LaunchRoute::Auto,
    }
}

/// The route this deployment actually runs: a mode that does not offer
/// side-by-side directions (see `ServeMode::allows_design_variants`) runs
/// the brief as one orchestrated design instead of refusing it.
pub(super) fn route_for_mode(
    route: LaunchRoute,
    mode: crate::web_canvas_server::ServeMode,
) -> LaunchRoute {
    match route {
        LaunchRoute::Variants(_) if !mode.allows_design_variants() => LaunchRoute::Orchestrator,
        other => other,
    }
}

/// The intent a pinned route decides by itself, so the classifier gets no
/// vote. A refine without a plan (nothing selected to edit) falls back to
/// the classifier rather than inventing a target.
pub(super) fn pinned_intent(route: LaunchRoute, has_modify_plan: bool) -> Option<DesignIntent> {
    match route {
        LaunchRoute::Auto => None,
        LaunchRoute::Orchestrator | LaunchRoute::Variants(_) => Some(DesignIntent::New),
        LaunchRoute::Refine if has_modify_plan => Some(DesignIntent::Modify),
        LaunchRoute::Refine => None,
    }
}

pub(super) fn clear_fresh_starter_frame_for_design(state: &mut EditorState) -> bool {
    if state.doc != EditorState::starter().doc {
        return false;
    }
    state.active_children_mut().clear();
    state.clear_selection();
    // Raw `active_children_mut()` bypasses the command/history path, so it
    // must advance the content revision explicitly. Save acknowledgements
    // use that revision to avoid marking newer edits as saved.
    state.mark_document_changed();
    true
}

pub(super) fn clear_live_starter_frame_for_design(state: &mut WebCanvasState) -> Option<u64> {
    if !clear_fresh_starter_frame_for_design(&mut state.editor) {
        return None;
    }
    state.version += 1;
    Some(state.version)
}

pub(super) fn resolve_standard_route(
    classified: DesignIntent,
    page_children_empty: bool,
    has_modify_plan: bool,
) -> DesignIntent {
    match classified {
        DesignIntent::Modify if page_children_empty => DesignIntent::New,
        DesignIntent::Modify if !has_modify_plan => DesignIntent::New,
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(json: &str) -> serde_json::Map<String, Value> {
        serde_json::from_str::<Value>(json)
            .unwrap()
            .as_object()
            .cloned()
            .unwrap()
    }

    #[test]
    fn launch_route_parses_the_pinned_values_and_defaults_to_auto() {
        assert_eq!(
            parse_launch_route(&body(r#"{"launchRoute":"orchestrator"}"#)),
            LaunchRoute::Orchestrator
        );
        assert_eq!(
            parse_launch_route(&body(r#"{"launchRoute":"refine"}"#)),
            LaunchRoute::Refine
        );
        assert_eq!(
            parse_launch_route(&body(r#"{"launchRoute":null}"#)),
            LaunchRoute::Auto
        );
        assert_eq!(
            parse_launch_route(&body(r#"{"launchRoute":"x"}"#)),
            LaunchRoute::Auto
        );
        assert_eq!(parse_launch_route(&body("{}")), LaunchRoute::Auto);
    }

    #[test]
    fn a_variants_route_carries_a_clamped_direction_count() {
        assert_eq!(
            parse_launch_route(&body(r#"{"launchRoute":"variants","variantCount":4}"#)),
            LaunchRoute::Variants(4)
        );
        // Missing → the default; out of range → clamped like desktop.
        assert_eq!(
            parse_launch_route(&body(r#"{"launchRoute":"variants"}"#)),
            LaunchRoute::Variants(op_editor_core::DEFAULT_VARIANT_COUNT)
        );
        assert_eq!(
            parse_launch_route(&body(r#"{"launchRoute":"variants","variantCount":1}"#)),
            LaunchRoute::Variants(2)
        );
        assert_eq!(
            parse_launch_route(&body(r#"{"launchRoute":"variants","variantCount":9000}"#)),
            LaunchRoute::Variants(op_editor_core::MAX_VARIANT_COUNT)
        );
        // Still a whole-design request: the classifier gets no vote.
        assert_eq!(
            pinned_intent(LaunchRoute::Variants(3), false),
            Some(DesignIntent::New)
        );
    }

    #[test]
    fn only_a_mode_that_offers_directions_keeps_the_variants_route() {
        use crate::web_canvas_server::ServeMode;
        let variants = LaunchRoute::Variants(3);
        assert_eq!(route_for_mode(variants, ServeMode::Local), variants);
        assert_eq!(route_for_mode(variants, ServeMode::Managed), variants);
        assert_eq!(
            route_for_mode(variants, ServeMode::Online),
            LaunchRoute::Orchestrator
        );
        assert_eq!(
            route_for_mode(LaunchRoute::Refine, ServeMode::Online),
            LaunchRoute::Refine
        );
        // The route decides which runner the new-design branch takes.
        assert_eq!(variants.variant_count(), Some(3));
        assert_eq!(LaunchRoute::Orchestrator.variant_count(), None);
    }

    #[test]
    fn a_pinned_route_decides_the_intent_without_the_classifier() {
        assert_eq!(
            pinned_intent(LaunchRoute::Orchestrator, false),
            Some(DesignIntent::New)
        );
        assert_eq!(
            pinned_intent(LaunchRoute::Refine, true),
            Some(DesignIntent::Modify)
        );
        assert_eq!(pinned_intent(LaunchRoute::Refine, false), None);
        assert_eq!(pinned_intent(LaunchRoute::Auto, true), None);
    }
}
