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
/// selected boards in place (`"refine"`). Anything else is the ordinary
/// classified route.
pub(super) fn parse_launch_route(obj: &serde_json::Map<String, Value>) -> LaunchRoute {
    match obj.get("launchRoute").and_then(Value::as_str) {
        Some("orchestrator") => LaunchRoute::Orchestrator,
        Some("refine") => LaunchRoute::Refine,
        _ => LaunchRoute::Auto,
    }
}

/// The intent a pinned route decides by itself, so the classifier gets no
/// vote. A refine without a plan (nothing selected to edit) falls back to
/// the classifier rather than inventing a target.
pub(super) fn pinned_intent(route: LaunchRoute, has_modify_plan: bool) -> Option<DesignIntent> {
    match route {
        LaunchRoute::Auto => None,
        LaunchRoute::Orchestrator => Some(DesignIntent::New),
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
