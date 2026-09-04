//! Track C-1 of the interactive-preview plan — auto-wire App Mode on
//! preview entry.
//!
//! `PreviewSession::enter` already turns a document with authored
//! `screen` markers into a routed multi-screen App Mode session (see the
//! module doc on `super::PreviewSession::enter`). Most generated documents
//! now pick up those markers for free — Track A's
//! `op_orchestrator::wire_screen_navigation::wire_screen_navigation` runs at
//! the end of every generation turn (`op_orchestrator::cleanup::
//! run_cleanup_passes`) — but a hand-drawn document, an import, or anything
//! authored before Track A shipped still has none, so preview falls back to
//! the classic single-page workbench view even though the doc plainly has
//! several screen-shaped top-level frames.
//!
//! [`auto_wire_for_preview`] closes that gap AT PREVIEW-ENTRY TIME: if the
//! document carries NO authored `screen` marker at all (on the page
//! `enter` is about to preview), it runs the SAME deterministic pass over
//! an `EditorState` built from a clone of the document and hands the
//! result back. `enter` then builds the runtime from that wired clone
//! instead of the original — the caller-visible (saved) document is never
//! touched, matching `enter`'s existing "never mutates the saved doc"
//! invariant (see the module doc on `preview/mod.rs`).
//!
//! ## Why "any authored marker → skip entirely", not Track A's own
//! "fill only the gaps" rule
//!
//! `wire_screen_navigation` itself is additive-only per node (contract
//! point 4 in `op_orchestrator::wire_screen_navigation`): it fills in
//! whatever is missing even when SOME screens already carry a marker. At
//! generation time that is exactly right — a multi-turn document
//! accumulates screens incrementally and each turn should link the new
//! ones in. At PREVIEW-ENTRY time it is deliberately stricter: an author
//! who has started marking screens by hand is mid-way through an
//! intentional App Mode setup, and silently binding additional nav taps
//! behind their back on every preview open would be a confusing,
//! preview-only side effect they never asked for and can't see in the
//! editor. So this module's gate is document-wide ("does ANY authored
//! marker exist"), not per-node — user/model intent wins outright rather
//! than being merged with.
//! Generation-only back/card completion is deliberately not part of this
//! cloned-state fallback; those interactions are persisted by cleanup.
//!
//! ## Why op-host-native can depend on op-orchestrator
//!
//! `op-host-native` is not a wasm32 crate (`op-host-web` does not depend on
//! it), and `op-orchestrator`'s own dependency graph never reaches back to
//! `op-host-native` (verified: `op-editor-core` / `op-design-lint` /
//! `op-mcp` / `op-ai-skills` / `op-pen-loader` / jian schema-only crates —
//! no cycle). The dependency is gated in the same
//! `[target.'cfg(not(target_arch = "wasm32"))']` block as `jian-core` /
//! `op-pen-loader` in `Cargo.toml`.

use jian_ops_schema::node::PenNode;
use jian_ops_schema::PenDocument;
use op_editor_core::{EditorCommand, EditorState, NodeId};
use op_orchestrator::wire_screen_navigation::wire_screen_navigation;
use op_orchestrator::DocSink;

/// Minimal [`DocSink`] wrapping a borrowed [`EditorState`] — mirrors
/// `op_orchestrator::loop_finalize::StateDocSink`, duplicated here because
/// that type is `pub(crate)` to `op-orchestrator` and this preview-entry
/// hook is the only caller outside that crate.
struct PreviewDocSink<'a> {
    state: &'a mut EditorState,
}

impl DocSink for PreviewDocSink<'_> {
    fn state(&self) -> &EditorState {
        self.state
    }
    fn apply(&mut self, cmd: EditorCommand) -> bool {
        self.state.apply(cmd)
    }
    fn insert_subtree_returning_root_ids(
        &mut self,
        nodes: Vec<PenNode>,
        parent_id: &NodeId,
    ) -> Option<Vec<String>> {
        self.state
            .insert_subtree_returning_root_ids(nodes, parent_id)
    }
    fn begin_undo_batch(&mut self) {}
    fn end_undo_batch(&mut self) {}
}

/// True if any top-level frame on the active page already carries an
/// authored `screen` marker — the escape hatch that makes this whole
/// module a no-op once an author has started marking screens by hand.
fn has_authored_screen_marker(state: &EditorState) -> bool {
    state
        .active_children()
        .iter()
        .any(|node| matches!(node, PenNode::Frame(f) if f.screen.is_some()))
}

/// If `doc` carries no authored `screen` marker on `active_page_index`,
/// run Track A's deterministic screen/nav wiring pass over an `EditorState`
/// CLONE and return the wired document; otherwise `None` — the caller
/// keeps using the original `doc` unmodified (zero cost, zero risk of
/// layering preview-only bindings atop an author's in-progress App Mode
/// work). Never mutates `doc` itself.
pub(super) fn auto_wire_for_preview(
    doc: &PenDocument,
    active_page_index: usize,
) -> Option<PenDocument> {
    let mut state = EditorState::from_document(doc.clone());
    state.ui.active_page_index = active_page_index;
    if has_authored_screen_marker(&state) {
        return None;
    }
    let mut sink = PreviewDocSink { state: &mut state };
    wire_screen_navigation(&mut sink);
    Some(state.doc)
}

#[cfg(test)]
#[path = "auto_wire_tests.rs"]
mod tests;
