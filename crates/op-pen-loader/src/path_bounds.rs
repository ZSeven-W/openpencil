//! Bezier path-anchor bounds for the canonical schema. The actual
//! algorithm lives in `op_editor_core::path_bounds` so the editor's
//! `refit_path_bounds` and this absolutize pass agree exactly on a
//! path's native span — this is a thin `f64 → f32` adapter.

/// `(min_x, min_y, width, height)` of a path's anchors — endpoints
/// plus cubic-Bezier extrema. Delegates to the canonical
/// implementation in `op-editor-core`.
pub(super) fn path_bounds_from_anchors(
    anchors: &[jian_ops_schema::node::PenPathAnchor],
    closed: bool,
) -> (f32, f32, f32, f32) {
    let (x, y, w, h) = op_editor_core::path_bounds::path_bounds_from_anchors(anchors, closed);
    (x as f32, y as f32, w as f32, h as f32)
}
