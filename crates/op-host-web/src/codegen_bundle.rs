//! Live AI structure-bundle export for the web Code panel.
//!
//! TS parity: `code-panel.tsx` `handleDownloadStructureBundle` builds
//! a fresh bundle from `getTargetNodes()` at click time — the live
//! selection, else the active page — with no completed generation
//! required. The bundle layout comes from the shared
//! [`op_codegen::ai::bundle`] builder (manifest + raw/sanitized views
//! + assets), zipped by the dependency-free STORED encoder
//!
//! ([`op_codegen::ai::stored_zip`]) so the wasm build stays under the
//! bundle-size ceiling.
//!
//! Divergence (documented): selection ids resolve against the ACTIVE
//! page only (TS `getActivePageChildren` + `findNodeInTree` — same
//! scope), while the web Generate input builder searches all pages;
//! the bundle follows TS here.

use jian_ops_schema::node::PenNode;
use op_codegen::ai::assets::extract_codegen_assets;
use op_codegen::ai::bundle::{build_structure_bundle, BundleScope};
use op_codegen::ai::stored_zip::build_stored_zip;
use op_editor_core::walkers::find_node;
use op_editor_core::EditorState;

/// Build the structure-bundle zip from the LIVE editor state: the
/// selection's subtrees, else the active page's children. `None` when
/// there is nothing to bundle (TS returns silently on
/// `nodes.length === 0`).
pub fn build_live_bundle_zip(state: &EditorState) -> Option<Vec<u8>> {
    let children = state.active_children();
    let selected: Vec<&PenNode> = state
        .selection
        .set
        .iter()
        .filter_map(|id| find_node(children, id))
        .collect();
    let (scope, roots): (BundleScope, Vec<&PenNode>) = if selected.is_empty() {
        (BundleScope::Page, children.iter().collect())
    } else {
        (BundleScope::Selection, selected)
    };
    if roots.is_empty() {
        return None;
    }
    let raw = serde_json::to_string(&roots).ok()?;
    let (sanitized, assets) = extract_codegen_assets(&raw);
    let bundle = build_structure_bundle(&raw, &sanitized, &assets, scope);
    Some(build_stored_zip(&bundle.files))
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::NodeId;

    /// Two-rect doc via the canonical loader (op-editor-core's
    /// `test_support` is `#[cfg(test)]`-private to its crate).
    fn two_rect_state() -> EditorState {
        let doc = jian_ops_schema::load_str(
            r#"{"version":"1.0.0","children":[
                {"type":"rectangle","id":"n1","name":"A","x":0,"y":0,"width":10,"height":10},
                {"type":"rectangle","id":"n2","name":"B","x":20,"y":0,"width":10,"height":10}
            ]}"#,
        )
        .expect("fixture parses")
        .value;
        EditorState::from_document(doc)
    }

    #[test]
    fn selection_bundles_only_the_selected_subtrees() {
        let mut s = two_rect_state();
        s.set_single_selection(NodeId::new("n1"));
        let bytes = build_live_bundle_zip(&s).expect("bundle");
        // STORED zip with the manifest + both views present.
        assert_eq!(&bytes[0..4], &[0x50, 0x4B, 0x03, 0x04]);
        let hay = String::from_utf8_lossy(&bytes);
        assert!(hay.contains("manifest.json"));
        assert!(hay.contains("views/raw.json"));
        assert!(hay.contains(r#""id":"n1""#), "selected node in raw view");
        assert!(!hay.contains(r#""id":"n2""#), "unselected node excluded");
    }

    #[test]
    fn empty_selection_falls_back_to_the_active_page() {
        let s = two_rect_state();
        let bytes = build_live_bundle_zip(&s).expect("page bundle");
        let hay = String::from_utf8_lossy(&bytes);
        assert!(hay.contains(r#""id":"n1""#));
        assert!(hay.contains(r#""id":"n2""#), "page scope bundles all roots");
    }

    #[test]
    fn empty_page_returns_none() {
        let s = EditorState::new();
        assert!(build_live_bundle_zip(&s).is_none());
    }
}
