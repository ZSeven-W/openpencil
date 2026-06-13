//! Bezier path-editing ops shared by Pen-tool authoring and the
//! Select-tool path editor.
//!
//! Ports the TS sources verbatim:
//! - `packages/pen-core/src/path-anchors.ts` — `anchorsToPathData`
//!   (the persisted `d` string), `inferPathAnchorPointType`.
//! - `apps/web/src/canvas/skia/path-editing.ts` — `setPathPointType`,
//!   `resetPathPointHandles`, `buildDefaultHandles`,
//!   `applyMirroredPointType`, `movePathControl` (handle branch).
//!
//! TS runs these in scene-anchor space and bakes the result back into
//! the node (`bakeSceneAnchorsToPathNode`). The Rust path model keeps
//! its anchors in the same scale-1 space (`pen.rs::refit_path_bounds`
//! pins `base.x/y` to the anchors' handle-aware bounds min), so the
//! handle deltas these ops produce are numerically identical — only
//! the bake step (bounds re-fit + `d` regeneration) differs in where
//! the local origin sits. Both conventions normalize on read (the
//! loader and the TS renderer subtract the bounds min), so files are
//! cross-compatible either way.

use crate::node_id::NodeId;
use crate::state::EditorState;
use crate::walkers::find_node_mut;
use jian_ops_schema::node::{PenNode, PenPathAnchor, PenPathHandle, PenPathPointType};

/// TS `DEFAULT_HANDLE_RATIO` (`path-editing.ts:27`).
const DEFAULT_HANDLE_RATIO: f64 = 1.0 / 3.0;
/// TS `HANDLE_EPSILON` / `EPSILON` (`path-editing.ts:28`,
/// `path-anchors.ts:18`).
const HANDLE_EPSILON: f64 = 1e-6;

/// Serialize anchors into an SVG `d` string — port of
/// `path-anchors.ts::anchorsToPathData`. Segments whose endpoints
/// carry no handles emit `L`; any handle promotes the segment to a
/// cubic `C`. A closed path appends the last→first segment plus `Z`.
pub fn anchors_to_path_data(anchors: &[PenPathAnchor], closed: bool) -> String {
    if anchors.is_empty() {
        return String::new();
    }
    let mut parts = vec![format!("M {} {}", anchors[0].x, anchors[0].y)];
    for i in 1..anchors.len() {
        append_segment(&mut parts, &anchors[i - 1], &anchors[i]);
    }
    if closed && anchors.len() > 1 {
        append_segment(&mut parts, &anchors[anchors.len() - 1], &anchors[0]);
        parts.push("Z".to_string());
    }
    parts.join(" ")
}

/// One segment of [`anchors_to_path_data`] — TS `appendSegment`.
fn append_segment(parts: &mut Vec<String>, from: &PenPathAnchor, to: &PenPathAnchor) {
    if from.handle_out.is_none() && to.handle_in.is_none() {
        parts.push(format!("L {} {}", to.x, to.y));
        return;
    }
    let (hox, hoy) = from
        .handle_out
        .as_ref()
        .map(|h| (h.x, h.y))
        .unwrap_or((0.0, 0.0));
    let (hix, hiy) = to
        .handle_in
        .as_ref()
        .map(|h| (h.x, h.y))
        .unwrap_or((0.0, 0.0));
    parts.push(format!(
        "C {} {} {} {} {} {}",
        from.x + hox,
        from.y + hoy,
        to.x + hix,
        to.y + hiy,
        to.x,
        to.y
    ));
}

/// TS `hasMeaningfulHandle` — `None` or a sub-epsilon handle is "no
/// handle".
fn has_meaningful_handle(handle: Option<&PenPathHandle>) -> bool {
    handle
        .map(|h| h.x.abs() > HANDLE_EPSILON || h.y.abs() > HANDLE_EPSILON)
        .unwrap_or(false)
}

/// TS `areMirroredHandles` — opposite directions within a relative
/// tolerance.
fn are_mirrored_handles(a: &PenPathHandle, b: &PenPathHandle) -> bool {
    let len_a = a.x.hypot(a.y);
    let len_b = b.x.hypot(b.y);
    if len_a <= HANDLE_EPSILON || len_b <= HANDLE_EPSILON {
        return false;
    }
    let tol = HANDLE_EPSILON.max(len_a.max(len_b) * 1e-3);
    (a.x + b.x).abs() <= tol && (a.y + b.y).abs() <= tol
}

/// Resolve an anchor's effective point type — port of
/// `path-anchors.ts::inferPathAnchorPointType`. Used when the stored
/// `point_type` is absent.
pub fn infer_path_anchor_point_type(anchor: &PenPathAnchor) -> PenPathPointType {
    let has_in = has_meaningful_handle(anchor.handle_in.as_ref());
    let has_out = has_meaningful_handle(anchor.handle_out.as_ref());
    if !has_in && !has_out {
        return PenPathPointType::Corner;
    }
    if has_in && has_out {
        if let (Some(hi), Some(ho)) = (anchor.handle_in.as_ref(), anchor.handle_out.as_ref()) {
            if are_mirrored_handles(hi, ho) {
                return PenPathPointType::Mirrored;
            }
        }
    }
    PenPathPointType::Independent
}

/// TS `getAdjacentAnchor` — neighbor at `idx + delta`, wrapping only
/// on closed paths.
fn adjacent_anchor(
    anchors: &[PenPathAnchor],
    idx: usize,
    delta: isize,
    closed: bool,
) -> Option<&PenPathAnchor> {
    let next = idx as isize + delta;
    if next >= 0 && (next as usize) < anchors.len() {
        return anchors.get(next as usize);
    }
    if !closed || anchors.is_empty() {
        return None;
    }
    let n = anchors.len() as isize;
    anchors.get(((next % n + n) % n) as usize)
}

fn distance(a: &PenPathAnchor, b: &PenPathAnchor) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

fn normalize(x: f64, y: f64) -> Option<(f64, f64)> {
    let len = x.hypot(y);
    if len <= HANDLE_EPSILON {
        return None;
    }
    Some((x / len, y / len))
}

/// TS `buildDefaultHandles` — tangent-aligned default handles at 1/3
/// of the neighbor distances. Returns `(handle_in, handle_out)`.
fn build_default_handles(
    anchors: &[PenPathAnchor],
    idx: usize,
    closed: bool,
) -> (Option<PenPathHandle>, Option<PenPathHandle>) {
    let Some(current) = anchors.get(idx) else {
        return (None, None);
    };
    let prev = adjacent_anchor(anchors, idx, -1, closed);
    let next = adjacent_anchor(anchors, idx, 1, closed);
    match (prev, next) {
        (None, None) => (None, None),
        (Some(prev), Some(next)) => {
            if let Some((tx, ty)) = normalize(next.x - prev.x, next.y - prev.y) {
                let in_len = distance(current, prev) * DEFAULT_HANDLE_RATIO;
                let out_len = distance(current, next) * DEFAULT_HANDLE_RATIO;
                (
                    Some(PenPathHandle {
                        x: -tx * in_len,
                        y: -ty * in_len,
                    }),
                    Some(PenPathHandle {
                        x: tx * out_len,
                        y: ty * out_len,
                    }),
                )
            } else {
                // Degenerate tangent — TS falls through to the
                // prev-only branch shape below.
                (
                    Some(PenPathHandle {
                        x: (prev.x - current.x) * DEFAULT_HANDLE_RATIO,
                        y: (prev.y - current.y) * DEFAULT_HANDLE_RATIO,
                    }),
                    None,
                )
            }
        }
        (Some(prev), None) => (
            Some(PenPathHandle {
                x: (prev.x - current.x) * DEFAULT_HANDLE_RATIO,
                y: (prev.y - current.y) * DEFAULT_HANDLE_RATIO,
            }),
            None,
        ),
        (None, Some(next)) => (
            None,
            Some(PenPathHandle {
                x: (next.x - current.x) * DEFAULT_HANDLE_RATIO,
                y: (next.y - current.y) * DEFAULT_HANDLE_RATIO,
            }),
        ),
    }
}

fn handle_length(handle: Option<&PenPathHandle>) -> f64 {
    handle.map(|h| h.x.hypot(h.y)).unwrap_or(0.0)
}

/// TS `applyMirroredPointType` — pick a preferred direction (existing
/// out → inverted in → default out → inverted default in → +x), an
/// averaged mirror length (fallback 40), then emit symmetric handles
/// on whichever sides exist.
fn apply_mirrored_point_type(
    anchor: &mut PenPathAnchor,
    defaults: (Option<PenPathHandle>, Option<PenPathHandle>),
) {
    let (def_in, def_out) = defaults;
    let invert = |d: Option<(f64, f64)>| d.map(|(x, y)| (-x, -y));
    let dir = anchor
        .handle_out
        .as_ref()
        .and_then(|h| normalize(h.x, h.y))
        .or_else(|| invert(anchor.handle_in.as_ref().and_then(|h| normalize(h.x, h.y))))
        .or_else(|| def_out.as_ref().and_then(|h| normalize(h.x, h.y)))
        .or_else(|| invert(def_in.as_ref().and_then(|h| normalize(h.x, h.y))))
        .unwrap_or((1.0, 0.0));
    let lengths: Vec<f64> = [
        handle_length(anchor.handle_in.as_ref()),
        handle_length(anchor.handle_out.as_ref()),
        handle_length(def_in.as_ref()),
        handle_length(def_out.as_ref()),
    ]
    .into_iter()
    .filter(|v| *v > HANDLE_EPSILON)
    .collect();
    let avg = if lengths.is_empty() {
        0.0
    } else {
        lengths.iter().sum::<f64>() / lengths.len() as f64
    };
    let mirror_len = if avg == 0.0 { 40.0 } else { avg };
    let has_in = anchor.handle_in.is_some() || def_in.is_some();
    let has_out = anchor.handle_out.is_some() || def_out.is_some();
    anchor.handle_in = has_in.then(|| PenPathHandle {
        x: -dir.0 * mirror_len,
        y: -dir.1 * mirror_len,
    });
    anchor.handle_out = has_out.then(|| PenPathHandle {
        x: dir.0 * mirror_len,
        y: dir.1 * mirror_len,
    });
    anchor.point_type = Some(PenPathPointType::Mirrored);
}

/// Run `f` against a Path node's anchors at `index`, then re-fit the
/// node bounds + `d`. Shared edit plumbing for the two menu ops.
fn edit_anchor(
    state: &mut EditorState,
    node_id: &NodeId,
    index: usize,
    f: impl FnOnce(&mut Vec<PenPathAnchor>, usize, bool),
) -> bool {
    if !state.is_editable(node_id) {
        return false;
    }
    let Some(node) = find_node_mut(state.active_children_mut(), node_id) else {
        return false;
    };
    {
        let PenNode::Path(path) = &mut *node else {
            return false;
        };
        let closed = path.closed.unwrap_or(false);
        let Some(anchors) = path.anchors.as_mut() else {
            return false;
        };
        if index >= anchors.len() {
            return false;
        }
        f(anchors, index, closed);
    }
    crate::pen::refit_path_bounds(node);
    true
}

impl EditorState {
    /// Set a path anchor's point type with TS `setPathPointType`
    /// semantics (`path-editing.ts:116-146`): `Corner` clears both
    /// handles; `Mirrored` snaps symmetric handles seeded from the
    /// existing geometry or tangent defaults; `Independent` keeps the
    /// existing handles, filling absent ones from the defaults.
    /// History is the caller's responsibility.
    pub fn set_path_anchor_point_type_ts(
        &mut self,
        node_id: &NodeId,
        index: usize,
        point_type: PenPathPointType,
    ) -> bool {
        edit_anchor(self, node_id, index, |anchors, idx, closed| {
            match point_type {
                PenPathPointType::Corner => {
                    let anchor = &mut anchors[idx];
                    anchor.handle_in = None;
                    anchor.handle_out = None;
                    anchor.point_type = Some(PenPathPointType::Corner);
                }
                PenPathPointType::Mirrored => {
                    let defaults = build_default_handles(anchors, idx, closed);
                    apply_mirrored_point_type(&mut anchors[idx], defaults);
                }
                PenPathPointType::Independent => {
                    let defaults = build_default_handles(anchors, idx, closed);
                    let anchor = &mut anchors[idx];
                    if anchor.handle_in.is_none() {
                        anchor.handle_in = defaults.0;
                    }
                    if anchor.handle_out.is_none() {
                        anchor.handle_out = defaults.1;
                    }
                    anchor.point_type = Some(PenPathPointType::Independent);
                }
            };
        })
    }

    /// Reset a path anchor's handles to the tangent defaults — TS
    /// `resetPathPointHandles` (`path-editing.ts:148-170`). The point
    /// type is re-inferred from the new handles. History is the
    /// caller's responsibility.
    pub fn reset_path_anchor_handles(&mut self, node_id: &NodeId, index: usize) -> bool {
        edit_anchor(self, node_id, index, |anchors, idx, closed| {
            let defaults = build_default_handles(anchors, idx, closed);
            let anchor = &mut anchors[idx];
            anchor.handle_in = defaults.0;
            anchor.handle_out = defaults.1;
            anchor.point_type = Some(infer_path_anchor_point_type(anchor));
        })
    }

    /// Drag an EXISTING bezier handle to a new offset — port of the
    /// TS `movePathControl` handle branch (`path-editing.ts:84-113`):
    /// resolve the effective point type from the PRE-move anchor
    /// (stored, else inferred), write the handle, then
    /// - `Mirrored` → negate the opposite handle (only when it
    ///   exists) and store `pointType: 'mirrored'`;
    /// - a STORED `Independent` keeps its stored type;
    /// - anything else leaves `point_type` untouched (an untyped
    ///   anchor stays untyped — TS re-infers on the next read).
    /// History is the caller's responsibility.
    pub fn move_path_anchor_handle_ts(
        &mut self,
        node_id: &NodeId,
        index: usize,
        side: crate::pen::PathHandleSide,
        new_offset: (f64, f64),
    ) -> bool {
        use crate::pen::PathHandleSide;
        edit_anchor(self, node_id, index, |anchors, idx, _closed| {
            let anchor = &mut anchors[idx];
            let resolved = anchor
                .point_type
                .clone()
                .unwrap_or_else(|| infer_path_anchor_point_type(anchor));
            let stored_independent = anchor.point_type == Some(PenPathPointType::Independent);
            let next = PenPathHandle {
                x: new_offset.0,
                y: new_offset.1,
            };
            match side {
                PathHandleSide::In => anchor.handle_in = Some(next.clone()),
                PathHandleSide::Out => anchor.handle_out = Some(next.clone()),
            }
            if resolved == PenPathPointType::Mirrored {
                let opposite = match side {
                    PathHandleSide::In => &mut anchor.handle_out,
                    PathHandleSide::Out => &mut anchor.handle_in,
                };
                if opposite.is_some() {
                    *opposite = Some(PenPathHandle {
                        x: -next.x,
                        y: -next.y,
                    });
                }
                anchor.point_type = Some(PenPathPointType::Mirrored);
            } else if stored_independent {
                anchor.point_type = Some(PenPathPointType::Independent);
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pen_node_ext::PenNodeExt;
    use crate::test_support::state_with;
    use crate::walkers::find_node;

    fn anchor(x: f64, y: f64) -> PenPathAnchor {
        PenPathAnchor {
            x,
            y,
            handle_in: None,
            handle_out: None,
            point_type: None,
        }
    }

    /// A 3-anchor open path node directly in the document.
    fn path_state() -> (EditorState, NodeId) {
        let mut s = state_with(vec![]);
        let mut next_id = 1u64;
        let id = s.start_pen_path(&mut next_id, (0.0, 0.0)).expect("start");
        s.add_pen_point((30.0, 0.0));
        s.add_pen_point((60.0, 30.0));
        s.ui.pen_dragging_handle = false;
        let _ = s.finish_pen_path();
        (s, id)
    }

    fn anchors_of(s: &EditorState, id: &NodeId) -> Vec<PenPathAnchor> {
        match find_node(s.active_children(), id) {
            Some(PenNode::Path(p)) => p.anchors.clone().unwrap_or_default(),
            _ => panic!("path node missing"),
        }
    }

    #[test]
    fn anchors_to_path_data_lines_and_curves() {
        let mut a = anchor(0.0, 0.0);
        a.handle_out = Some(PenPathHandle { x: 10.0, y: 0.0 });
        let b = anchor(30.0, 30.0);
        let c = anchor(60.0, 30.0);
        // a→b is a cubic (a has handleOut), b→c a straight line.
        let d = anchors_to_path_data(&[a, b, c], false);
        assert_eq!(d, "M 0 0 C 10 0 30 30 30 30 L 60 30");
    }

    #[test]
    fn anchors_to_path_data_closed_appends_z() {
        let d = anchors_to_path_data(&[anchor(0.0, 0.0), anchor(10.0, 0.0)], true);
        assert_eq!(d, "M 0 0 L 10 0 L 0 0 Z");
    }

    #[test]
    fn infer_point_type_matches_ts() {
        assert_eq!(
            infer_path_anchor_point_type(&anchor(0.0, 0.0)),
            PenPathPointType::Corner
        );
        let mut mirrored = anchor(0.0, 0.0);
        mirrored.handle_in = Some(PenPathHandle { x: -5.0, y: -5.0 });
        mirrored.handle_out = Some(PenPathHandle { x: 5.0, y: 5.0 });
        assert_eq!(
            infer_path_anchor_point_type(&mirrored),
            PenPathPointType::Mirrored
        );
        let mut free = anchor(0.0, 0.0);
        free.handle_out = Some(PenPathHandle { x: 5.0, y: 5.0 });
        assert_eq!(
            infer_path_anchor_point_type(&free),
            PenPathPointType::Independent
        );
    }

    #[test]
    fn corner_point_type_clears_handles() {
        let (mut s, id) = path_state();
        // Give the middle anchor handles first.
        assert!(s.set_path_anchor_point_type_ts(&id, 1, PenPathPointType::Mirrored));
        assert!(anchors_of(&s, &id)[1].handle_in.is_some());
        assert!(s.set_path_anchor_point_type_ts(&id, 1, PenPathPointType::Corner));
        let a = &anchors_of(&s, &id)[1];
        assert!(a.handle_in.is_none() && a.handle_out.is_none());
        assert_eq!(a.point_type, Some(PenPathPointType::Corner));
    }

    #[test]
    fn mirrored_point_type_mints_symmetric_default_handles() {
        let (mut s, id) = path_state();
        assert!(s.set_path_anchor_point_type_ts(&id, 1, PenPathPointType::Mirrored));
        let a = &anchors_of(&s, &id)[1];
        let hi = a.handle_in.clone().expect("handle_in");
        let ho = a.handle_out.clone().expect("handle_out");
        // Symmetric: in == -out.
        assert!((hi.x + ho.x).abs() < 1e-9 && (hi.y + ho.y).abs() < 1e-9);
        assert_eq!(a.point_type, Some(PenPathPointType::Mirrored));
    }

    #[test]
    fn independent_fills_missing_handles_from_defaults() {
        let (mut s, id) = path_state();
        assert!(s.set_path_anchor_point_type_ts(&id, 1, PenPathPointType::Independent));
        let a = &anchors_of(&s, &id)[1];
        // Middle anchor has both neighbors → both defaults minted at
        // 1/3 of the neighbor distances along the prev→next tangent.
        assert!(a.handle_in.is_some() && a.handle_out.is_some());
        assert_eq!(a.point_type, Some(PenPathPointType::Independent));
    }

    #[test]
    fn reset_handles_reinfers_point_type() {
        let (mut s, id) = path_state();
        assert!(s.reset_path_anchor_handles(&id, 1));
        let a = &anchors_of(&s, &id)[1];
        // Tangent defaults on a middle anchor are mirrored only when
        // equal-length; here prev/next distances differ (30 vs ~42)
        // so the inferred type is Independent.
        assert!(a.handle_in.is_some() && a.handle_out.is_some());
        assert_eq!(a.point_type, Some(PenPathPointType::Independent));
    }

    #[test]
    fn endpoint_reset_gets_one_sided_default() {
        let (mut s, id) = path_state();
        assert!(s.reset_path_anchor_handles(&id, 0));
        let a = &anchors_of(&s, &id)[0];
        assert!(a.handle_in.is_none());
        assert!(a.handle_out.is_some());
    }

    #[test]
    fn move_handle_on_mirrored_anchor_mirrors_the_opposite() {
        // Stored Mirrored — TS movePathControl negates the opposite.
        let (mut s, id) = path_state();
        assert!(s.set_path_anchor_point_type_ts(&id, 1, PenPathPointType::Mirrored));
        assert!(s.move_path_anchor_handle_ts(
            &id,
            1,
            crate::pen::PathHandleSide::Out,
            (12.0, -6.0)
        ));
        let a = &anchors_of(&s, &id)[1];
        let ho = a.handle_out.clone().unwrap();
        let hi = a.handle_in.clone().unwrap();
        assert_eq!((ho.x, ho.y), (12.0, -6.0));
        assert_eq!((hi.x, hi.y), (-12.0, 6.0));
        assert_eq!(a.point_type, Some(PenPathPointType::Mirrored));
    }

    #[test]
    fn move_handle_resolves_inferred_mirrored_without_stored_type() {
        // No stored pointType but symmetric handles — TS resolves via
        // inferPathAnchorPointType and still mirrors the opposite.
        let (mut s, id) = path_state();
        assert!(s.set_path_anchor_point_type_ts(&id, 1, PenPathPointType::Mirrored));
        // Strip the stored type, keep the symmetric handles.
        if let Some(PenNode::Path(p)) = crate::walkers::find_node_mut(s.active_children_mut(), &id)
        {
            p.anchors.as_mut().unwrap()[1].point_type = None;
        }
        assert!(s.move_path_anchor_handle_ts(&id, 1, crate::pen::PathHandleSide::In, (-9.0, 3.0)));
        let a = &anchors_of(&s, &id)[1];
        let hi = a.handle_in.clone().unwrap();
        let ho = a.handle_out.clone().unwrap();
        assert_eq!((hi.x, hi.y), (-9.0, 3.0));
        assert_eq!((ho.x, ho.y), (9.0, -3.0));
        assert_eq!(a.point_type, Some(PenPathPointType::Mirrored));
    }

    #[test]
    fn move_handle_on_independent_anchor_leaves_opposite_alone() {
        let (mut s, id) = path_state();
        assert!(s.set_path_anchor_point_type_ts(&id, 1, PenPathPointType::Independent));
        let before_in = anchors_of(&s, &id)[1].handle_in.clone().unwrap();
        assert!(s.move_path_anchor_handle_ts(
            &id,
            1,
            crate::pen::PathHandleSide::Out,
            (20.0, 20.0)
        ));
        let a = &anchors_of(&s, &id)[1];
        let ho = a.handle_out.clone().unwrap();
        let hi = a.handle_in.clone().unwrap();
        assert_eq!((ho.x, ho.y), (20.0, 20.0));
        assert_eq!((hi.x, hi.y), (before_in.x, before_in.y), "opposite kept");
        assert_eq!(a.point_type, Some(PenPathPointType::Independent));
    }

    #[test]
    fn move_handle_on_untyped_one_sided_anchor_keeps_type_unset() {
        // A lone handle infers Independent (not stored) — TS leaves
        // pointType absent after the move.
        let (mut s, id) = path_state();
        if let Some(PenNode::Path(p)) = crate::walkers::find_node_mut(s.active_children_mut(), &id)
        {
            p.anchors.as_mut().unwrap()[1].handle_out = Some(PenPathHandle { x: 5.0, y: 0.0 });
        }
        assert!(s.move_path_anchor_handle_ts(&id, 1, crate::pen::PathHandleSide::Out, (8.0, 2.0)));
        let a = &anchors_of(&s, &id)[1];
        assert_eq!(a.point_type, None, "untyped stays untyped");
        assert!(a.handle_in.is_none(), "no opposite minted");
        let ho = a.handle_out.clone().unwrap();
        assert_eq!((ho.x, ho.y), (8.0, 2.0));
    }

    #[test]
    fn point_type_edit_refreshes_d_and_bounds() {
        let (mut s, id) = path_state();
        assert!(s.set_path_anchor_point_type_ts(&id, 1, PenPathPointType::Mirrored));
        let node = find_node(s.active_children(), &id).expect("node");
        let PenNode::Path(p) = node else {
            panic!("not a path")
        };
        let d = p.d.clone().expect("d regenerated");
        assert!(d.contains('C'), "curved segment must serialize: {d}");
        // Bounds stay pinned to the handle-aware anchor bounds.
        let (bx, by, _, _) = crate::path_bounds::path_bounds_from_anchors(
            p.anchors.as_deref().unwrap_or(&[]),
            false,
        );
        assert_eq!(node.base().x, Some(bx));
        assert_eq!(node.base().y, Some(by));
    }
}
