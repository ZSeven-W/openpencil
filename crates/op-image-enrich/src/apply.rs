//! Write-back of a resolved image result onto its target node.
//!
//! Moved here from `op-host-desktop/src/image_search_session.rs` (pure code
//! motion) so the headless MCP `enrich_images` tool and the desktop
//! image-search session land a url on a slot identically: an `Image` node's
//! `src`, a placeholder frame's / rectangle's authored image fill, or a fresh
//! crop-mode image fill when the slot carried none.

use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::{ImageFillBody, ImageFillMode, PenFill};
use op_editor_core::{
    walkers, CollabDocumentMutation, CollabEditSource, CollabGateAction, CollabGateReason,
    CollabUnsupportedFeature, EditorState, NodeId,
};

use crate::targets::{
    has_empty_image_fill, is_frame_placeholder_still_unfilled, is_image_area_rectangle_by_heuristic,
};

/// Sentinel `src` written when EVERY search avenue failed (junk-only
/// results, network error, empty corpus). The canvas paints it as the
/// theme-adaptive dashed placeholder (see `canvas_viewport_image`), so a
/// failed slot reads as "image goes here" instead of a bare grey box. The
/// bound query stays on the node for manual re-search.
pub const SEARCH_FAILED_PLACEHOLDER_SRC: &str = "placeholder://image-search-failed";

/// The collaboration gate every automatic enrichment write must pass.
/// Automatic enrichment is an external-asset mutation, which M1 does not
/// admit into a bound collaboration document.
pub fn collaboration_image_result_gate(state: &EditorState) -> Result<(), CollabGateReason> {
    state.editor_ui.collab.gate(
        CollabGateAction::Document(CollabDocumentMutation::Unsupported(
            CollabUnsupportedFeature::ExternalAssets,
        )),
        CollabEditSource::ExternalSync,
    )
}

/// Land a resolved stock-search url onto the target node in place.
///
/// `Image` nodes get their `src`; a still-unfilled placeholder frame or
/// rectangle (or a container whose only fill is an image fill with an empty
/// url) gets the url in place — an already-authored image-fill body keeps its
/// mode / crop / adjustments, a slot without one gets a fresh crop-mode image
/// fill and its children cleared. Returns `false` when the write was refused
/// (collaboration gate), the node vanished, or the url was already there.
pub fn apply_result(state: &mut EditorState, node_id: &NodeId, url: &str) -> bool {
    if let Err(reason) = collaboration_image_result_gate(state) {
        state.editor_ui.collab.set_notice(reason.notice_kind(), 0);
        return false;
    }
    let url = url.trim();
    if url.is_empty() {
        return false;
    }
    let Some(node) = walkers::find_node_mut(state.active_children_mut(), node_id) else {
        return false;
    };
    let is_unfilled_placeholder_frame = is_frame_placeholder_still_unfilled(node);
    let is_unfilled_placeholder_rectangle = is_image_area_rectangle_by_heuristic(node);
    let has_empty_image_fill = has_empty_image_fill(node);
    let changed = match node {
        PenNode::Image(image) => {
            if image.src == url {
                return false;
            }
            image.src = url.into();
            true
        }
        PenNode::Frame(frame) if is_unfilled_placeholder_frame || has_empty_image_fill => {
            match frame.container.fill.as_deref_mut() {
                // A slot already authored as a single image fill keeps its
                // body — only the still-empty url lands.
                Some([PenFill::Image(body)]) => {
                    body.url = url.into();
                }
                _ => {
                    frame.container.fill = Some(vec![PenFill::Image(ImageFillBody {
                        url: url.into(),
                        mode: Some(ImageFillMode::Crop),
                        original_size: None,
                        transform: None,
                        tile_scale: None,
                        explain: None,
                        opacity: None,
                        blend_mode: None,
                        exposure: None,
                        contrast: None,
                        saturation: None,
                        temperature: None,
                        tint: None,
                        highlights: None,
                        shadows: None,
                    })]);
                    frame.children = Some(Vec::new());
                }
            }
            true
        }
        PenNode::Rectangle(rect) if is_unfilled_placeholder_rectangle || has_empty_image_fill => {
            match rect.container.fill.as_deref_mut() {
                // Same in-place url overwrite as the frame branch: the authored
                // image-fill body (mode, crop, adjustments) survives.
                Some([PenFill::Image(body)]) => {
                    body.url = url.into();
                }
                _ => {
                    rect.container.fill = Some(vec![PenFill::Image(ImageFillBody {
                        url: url.into(),
                        mode: Some(ImageFillMode::Crop),
                        original_size: None,
                        transform: None,
                        tile_scale: None,
                        explain: None,
                        opacity: None,
                        blend_mode: None,
                        exposure: None,
                        contrast: None,
                        saturation: None,
                        temperature: None,
                        tint: None,
                        highlights: None,
                        shadows: None,
                    })]);
                    rect.children = Some(Vec::new());
                }
            }
            true
        }
        _ => false,
    };
    if changed {
        // This writes document content through raw `active_children_mut()`
        // outside the command/history path, so bump the revision. The
        // layer-panel row cache + save-dirty tracking key on
        // `document_revision()`; the placeholder-frame/rectangle branches
        // also clear `children`, which changes the visible layer rows.
        state.mark_document_changed();
    }
    changed
}
