//! Root-level property patches the cleanup driver applies in place.
//!
//! Carved off `cleanup.rs` (spine + siblings, 800-line cap) as pure code
//! motion. What holds these three together is the SHAPE of the edit, not the
//! defect they fix: each reads one root, decides one property, and writes it
//! with a `PatchNodeData` — deliberately NOT an `apply_root_transform`, which
//! rebuilds the subtree and hands the root a fresh id. Anything holding the
//! root id (the caller's `root_ids`, the loop's screen bookkeeping) would be
//! left pointing at a node that no longer exists, so a pass that only sets a
//! number or a keyword must not use one.

use op_editor_core::{EditorCommand, NodeId, PenNodeExt};

use crate::types::DocSink;

/// Env-gated (`OPENPENCIL_DEBUG_CLEANUP=1`) probe: log the named child's
/// current height under `root_id`, tagged with the pass that just ran.
/// Lives here (not in the `cleanup.rs` spine) to keep the spine under the
/// 800-line cap; the driver calls it through the existing
/// `use cleanup_root_patches::*` glob.
pub(super) fn debug_probe_child_height(sink: &dyn DocSink, root_id: &str, tag: &str) {
    if std::env::var("OPENPENCIL_DEBUG_CLEANUP").is_err() {
        return;
    }
    let Some(root) = sink
        .state()
        .active_children()
        .iter()
        .find(|n| n.id_str() == root_id)
    else {
        eprintln!("[CLEANUP-PROBE] {tag}: root {root_id} NOT FOUND");
        return;
    };
    let Ok(v) = serde_json::to_value(root) else {
        return;
    };
    for c in v
        .get("children")
        .and_then(|c| c.as_array())
        .into_iter()
        .flatten()
    {
        let name = c.get("name").and_then(|n| n.as_str()).unwrap_or("?");
        if name.to_lowercase().contains("sidebar") {
            eprintln!("[CLEANUP-PROBE] {tag}: {name} height={:?}", c.get("height"));
        }
    }
}

/// The geometric half of the deck judgement, per root.
///
/// [`crate::cleanup::CleanupPolicy::is_deck`] answers "the REQUEST asked for a
/// deck", which is prompt keywords and therefore blind three ways: the agentic
/// loop has no prompt at all (its policy is `Default`, so `is_deck` was
/// permanently false), a scene template that reaches cleanup carries no
/// request, and a user who says "1920×1080 主视觉" without ever typing PPT
/// still drew a board. Unioned in at the point of use rather than baked into
/// the policy so the answer is taken PER ROOT — one document can hold a board
/// and a page, and a single run-wide flag cannot describe that.
pub(super) fn root_is_deck_board(sink: &dyn DocSink, root_id: &str) -> bool {
    crate::geometry_validation::root_design_form(sink.state(), root_id).is_deck_board()
}

/// Centre a deck board's content on its fixed 16:9 surface.
///
/// A slide root is 1080 tall no matter how much content it holds, and its
/// sections hug their own height, so without this they stack from the top and
/// leave the lower half blank — measured on every generated deck.
///
/// Only `justifyContent` is written. Stretching the sections to fill instead
/// would distort whatever the model composed; centring changes where the block
/// sits, not what it is.
pub(super) fn centre_deck_board_content(sink: &mut dyn DocSink, root_id: &str) {
    let Some(root) = sink
        .state()
        .active_children()
        .iter()
        .find(|node| node.id_str() == root_id)
    else {
        return;
    };
    let value = serde_json::to_value(root).unwrap_or(serde_json::Value::Null);
    // Respect an explicit distribution: a board that deliberately pushes
    // content apart (space_between) or pins it low is a composition, not the
    // default top-stack this repairs.
    if value
        .get("justifyContent")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|mode| !mode.is_empty())
    {
        return;
    }
    let node_id = NodeId::new(root.id_str());
    sink.apply(EditorCommand::PatchNodeData {
        node_id,
        patch_json: r#"{"justifyContent":"center"}"#.to_string(),
        page_id: None,
    });
}

/// Centre a card board's content on its fixed portrait surface (DS P2-b B).
///
/// The deck precedent centres on form alone; a card is a small independent
/// board where an authored top stack can be the whole composition, so the
/// card version fires only when the REAL layout proves a trailing void of at
/// least [`CARD_VOID_CENTRE_FLOOR`] of the board height — that void is the
/// same "content stacked from the top on a fixed board" defect the deck
/// version exists for, measured instead of assumed. The repair is the same
/// one-line `justifyContent:"center"` patch: it splits the void in half, it
/// never restructures or rescales. Mounted in the driver AFTER the padding
/// floor so the void is measured on settled margins; deck boards never fall
/// into this gate (their centre sits earlier).
pub(super) fn centre_card_board_content(sink: &mut dyn DocSink, root_id: &str) {
    // Deck boards belong to the deck centre mounted earlier in the driver.
    if !crate::geometry_validation::root_design_form(sink.state(), root_id).is_card_board() {
        return;
    }
    let Some(root) = sink
        .state()
        .active_children()
        .iter()
        .find(|node| node.id_str() == root_id)
    else {
        return;
    };
    // Fixed board only: a hug-height root resolves to its content, where
    // there is no void to centre away.
    if root.height_px().is_none() {
        return;
    }
    let value = serde_json::to_value(root).unwrap_or(serde_json::Value::Null);
    // Respect an explicit distribution (deck precedent): a board that
    // deliberately pushes content apart or pins it low is a composition.
    if value
        .get("justifyContent")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|mode| !mode.is_empty())
    {
        return;
    }
    let Some(void) = crate::board_trailing_void::root_trailing_void_ratio(sink.state(), root_id)
    else {
        return;
    };
    if void < crate::board_trailing_void::CARD_VOID_CENTRE_FLOOR {
        return;
    }
    sink.apply(EditorCommand::PatchNodeData {
        node_id: NodeId::new(root.id_str()),
        patch_json: r#"{"justifyContent":"center"}"#.to_string(),
        page_id: None,
    });
}

/// Write the repaired root gap as a property patch.
///
/// Deliberately NOT an `apply_root_transform`: that rebuilds the subtree and
/// hands the root a fresh id, which is the right shape for passes that
/// restructure but wrong for setting one number — anything holding the root id
/// (the caller's `root_ids`, the loop's screen bookkeeping) would be left
/// pointing at a node that no longer exists.
pub(super) fn patch_root_section_gap(sink: &mut dyn DocSink, root_id: &str) {
    let Some(root) = sink
        .state()
        .active_children()
        .iter()
        .find(|node| node.id_str() == root_id)
    else {
        return;
    };
    let node_id = NodeId::new(root.id_str());
    let Ok(mut value) = serde_json::to_value(root) else {
        return;
    };
    if !crate::root_section_gap::fix_root_section_gap(&mut value) {
        return;
    }
    let Some(gap) = value.get("gap") else {
        return;
    };
    sink.apply(EditorCommand::PatchNodeData {
        node_id,
        patch_json: format!(r#"{{"gap":{gap}}}"#),
        page_id: None,
    });
}

#[cfg(test)]
#[path = "cleanup_card_board_tests.rs"]
mod tests;
