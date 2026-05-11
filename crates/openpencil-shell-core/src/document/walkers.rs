//! Free tree-walk helpers for `Document` mutators.
//!
//! All functions are `pub(super)` so the parent `document` module
//! can `use walkers::*` and call them unqualified — keeping the
//! `impl Document` call sites unchanged across the split.

use super::*;

/// Recursive helper for `Document::set_selected_color`.
pub(in crate::document) fn set_color_walk(
    node: &mut Node,
    target: NodeId,
    is_fill: bool,
    color: crate::Color,
) -> bool {
    if node.id == target {
        if is_fill {
            node.fill = Some(color);
        } else {
            // Preserve stroke width if a stroke already exists;
            // otherwise spin one up at 1 px.
            let width = node.stroke.map(|s| s.width).unwrap_or(1.0);
            node.stroke = Some(Stroke { color, width });
        }
        return true;
    }
    for child in &mut node.children {
        if set_color_walk(child, target, is_fill, color) {
            return true;
        }
    }
    false
}

/// Recursive helper for `Document::set_selected_rotation`. Sets
/// the matched node's rotation in radians. Containers carry
/// rotation too — rendering applies the transform around the
/// node's aggregate-bounds center.
pub(in crate::document) fn set_rotation_walk(
    node: &mut Node,
    target: NodeId,
    radians: f32,
) -> bool {
    if node.id == target {
        node.rotation = radians;
        return true;
    }
    for child in &mut node.children {
        if set_rotation_walk(child, target, radians) {
            return true;
        }
    }
    false
}

/// Recursive helper for `Document::set_selected_bounds`. Walks
/// the active page until it finds `target`, then overwrites its
/// bounds. Containers (bounds=ZERO) are no-ops.
pub(in crate::document) fn set_bounds_walk(
    node: &mut Node,
    target: NodeId,
    new_bounds: crate::Rect,
) -> bool {
    if node.id == target {
        if node.bounds.size.x > 0.0 || node.bounds.size.y > 0.0 {
            node.bounds = new_bounds;
        }
        return true;
    }
    for child in &mut node.children {
        if set_bounds_walk(child, target, new_bounds) {
            return true;
        }
    }
    false
}

/// Recursive helper for `Document::node_at_doc_point` — returns
/// the topmost id whose bounds contain `point`. When a node
/// carries rotation, the test point is inverse-rotated about the
/// node's bounds centre BEFORE testing children + self, so the
/// hit area matches what the renderer paints.
pub(in crate::document) fn hit_test_walk(
    node: &Node,
    point: crate::Point2D,
    zoom: f32,
) -> Option<NodeId> {
    // Hidden nodes skip canvas hit-test entirely — subtree
    // inherits (children of a hidden Frame are unclickable too).
    if node.hidden {
        return None;
    }
    let bounds = node.aggregate_bounds();
    // Rotation pivot is kind-aware so a Line node whose
    // `node.bounds` carries a negative dimension (collapsing the
    // aggregate to `Rect::ZERO`) still rotates around the segment
    // midpoint instead of (0, 0).
    let local = if node.rotation.abs() > f32::EPSILON {
        if let Some(pivot) = rotation_pivot(node, bounds) {
            let dx = point.x - pivot.x;
            let dy = point.y - pivot.y;
            let cos_t = (-node.rotation).cos();
            let sin_t = (-node.rotation).sin();
            crate::Point2D::new(
                pivot.x + dx * cos_t - dy * sin_t,
                pivot.y + dx * sin_t + dy * cos_t,
            )
        } else {
            point
        }
    } else {
        point
    };
    for child in node.children.iter().rev() {
        if let Some(hit) = hit_test_walk(child, local, zoom) {
            return Some(hit);
        }
    }
    // Locked nodes themselves can't be selected via canvas hit,
    // but their children still can (TS parity:
    // `spatial-index.ts` filters per-node, not subtree). This
    // check runs AFTER the child walk so descendants of a locked
    // Frame remain hittable; only the Frame body is opt-out.
    if node.locked {
        return None;
    }
    if point_in_node(node, local, bounds, zoom) {
        return Some(node.id);
    }
    None
}

/// Rotation pivot for hit-test. Most kinds rotate around the
/// aggregate-bounds center; Lines need a kind-specific path
/// because a Line with a negative-size dimension collapses its
/// aggregate to `Rect::ZERO` (the segment midpoint is still well
/// defined from `node.bounds`). Returns `None` when no valid
/// pivot exists (e.g. a degenerate Line that's just a point).
pub(in crate::document) fn rotation_pivot(
    node: &Node,
    bounds: crate::Rect,
) -> Option<crate::Point2D> {
    if matches!(node.kind, NodeKind::Line) {
        let raw = node.bounds;
        if raw.size.x.abs() < f32::EPSILON && raw.size.y.abs() < f32::EPSILON {
            return None;
        }
        return Some(crate::Point2D::new(
            raw.origin.x + raw.size.x / 2.0,
            raw.origin.y + raw.size.y / 2.0,
        ));
    }
    if bounds.size.x > 0.0 && bounds.size.y > 0.0 {
        return Some(crate::Point2D::new(
            bounds.origin.x + bounds.size.x / 2.0,
            bounds.origin.y + bounds.size.y / 2.0,
        ));
    }
    None
}

/// Per-NodeKind hit-test. Frames / Groups / Rects / Text / Other
/// use the axis-aligned bounds, matching their fill paint. Ellipse
/// / Polygon / Line use tighter geometry so the click area matches
/// what `canvas_viewport::paint_node` actually paints (codex audit:
/// previously every kind used rect bounds, so users could "click
/// inside the bounding box but outside the painted oval/triangle/
/// stroke" and still select).
pub(in crate::document) fn point_in_node(
    node: &Node,
    local: crate::Point2D,
    bounds: crate::Rect,
    zoom: f32,
) -> bool {
    // Lines get a dedicated path because:
    //   - horizontal / vertical segments have one zero dimension
    //     on the bounds rect; the AABB pre-filter used by other
    //     kinds would reject those even when the click is right
    //     on the stroke.
    //   - negative-size bounds (right-to-left / bottom-to-top)
    //     collapse to `Rect::ZERO` in `Node::aggregate_bounds`,
    //     so the Line path reads `node.bounds` directly. The
    //     distance-to-segment helper is sign-independent.
    if matches!(node.kind, NodeKind::Line) {
        let raw = node.bounds;
        // A true point (both dims 0) is not a hittable segment.
        if raw.size.x.abs() < f32::EPSILON && raw.size.y.abs() < f32::EPSILON {
            return false;
        }
        let from = raw.origin;
        let to = crate::Point2D::new(raw.origin.x + raw.size.x, raw.origin.y + raw.size.y);
        let stroke_half = node.stroke.map(|s| s.width / 2.0).unwrap_or(1.0);
        // Slack target: 4 screen px regardless of zoom. The point
        // is already in doc space, so scale by `1/zoom` to get
        // the doc-space equivalent (low zoom → bigger doc-space
        // slack; high zoom → tighter).
        let screen_slack = 4.0 / zoom.max(0.0001);
        return distance_point_to_segment(local, from, to) <= stroke_half + screen_slack;
    }
    // Non-line kinds need real positive area on both axes — the
    // ellipse / triangle / rect paint paths all expect a positive
    // rect.
    if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 {
        return false;
    }
    // Axis-aligned bounding check is a cheap reject for every
    // kind — the tighter checks below only need to refine the
    // hit inside the bounds.
    let in_bounds = local.x >= bounds.origin.x
        && local.x <= bounds.origin.x + bounds.size.x
        && local.y >= bounds.origin.y
        && local.y <= bounds.origin.y + bounds.size.y;
    if !in_bounds {
        return false;
    }
    let cx = bounds.origin.x + bounds.size.x / 2.0;
    let cy = bounds.origin.y + bounds.size.y / 2.0;
    let rx = (bounds.size.x / 2.0).max(0.0001);
    let ry = (bounds.size.y / 2.0).max(0.0001);
    match node.kind {
        NodeKind::Ellipse => {
            let dx = (local.x - cx) / rx;
            let dy = (local.y - cy) / ry;
            dx * dx + dy * dy <= 1.0
        }
        NodeKind::Polygon => {
            // Triangle vertices: top-center, bottom-left, bottom-
            // right. Mirrors `canvas_viewport.rs::paint_node`'s
            // `fill_polygon` call so hit-test follows paint.
            let top = crate::Point2D::new(cx, bounds.origin.y);
            let bl = crate::Point2D::new(bounds.origin.x, bounds.origin.y + bounds.size.y);
            let br = crate::Point2D::new(
                bounds.origin.x + bounds.size.x,
                bounds.origin.y + bounds.size.y,
            );
            point_in_triangle(local, top, bl, br)
        }
        // Frame, Group, Rect, Text, Other — bounds-only hit.
        _ => true,
    }
}

pub(in crate::document) fn point_in_triangle(
    p: crate::Point2D,
    a: crate::Point2D,
    b: crate::Point2D,
    c: crate::Point2D,
) -> bool {
    // Sign-of-cross-product method. Inside iff all three edge
    // signs match (or any is zero — point on the edge).
    let s1 = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
    let s2 = (c.x - b.x) * (p.y - b.y) - (c.y - b.y) * (p.x - b.x);
    let s3 = (a.x - c.x) * (p.y - c.y) - (a.y - c.y) * (p.x - c.x);
    let has_neg = s1 < 0.0 || s2 < 0.0 || s3 < 0.0;
    let has_pos = s1 > 0.0 || s2 > 0.0 || s3 > 0.0;
    !(has_neg && has_pos)
}

pub(in crate::document) fn distance_point_to_segment(
    p: crate::Point2D,
    a: crate::Point2D,
    b: crate::Point2D,
) -> f32 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq < f32::EPSILON {
        let pdx = p.x - a.x;
        let pdy = p.y - a.y;
        return (pdx * pdx + pdy * pdy).sqrt();
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / len_sq).clamp(0.0, 1.0);
    let cx = a.x + t * dx;
    let cy = a.y + t * dy;
    let pdx = p.x - cx;
    let pdy = p.y - cy;
    (pdx * pdx + pdy * pdy).sqrt()
}

/// Recursive helper for `Document::translate_selected`. Returns
/// `true` once `target` has been translated. Both the matched
/// node AND every descendant are moved — child bounds are stored
/// in document space (not relative to the parent), so dragging a
/// bounded Frame must shift the children too or they'd detach.
pub(in crate::document) fn translate_walk(
    node: &mut Node,
    target: NodeId,
    dx: f32,
    dy: f32,
) -> bool {
    if node.id == target {
        translate_subtree(node, dx, dy);
        return true;
    }
    for child in &mut node.children {
        if translate_walk(child, target, dx, dy) {
            return true;
        }
    }
    false
}

/// Return true if `target` is a descendant (not equal to itself)
/// of any node in `set` within `children`. Used by
/// `translate_selected` to dedupe shifts when both an ancestor
/// and one of its descendants are in the selection set.
pub(in crate::document) fn is_ancestor_in_set(
    children: &[Node],
    target: NodeId,
    set: &[NodeId],
) -> bool {
    for child in children {
        if descendant_contains(child, target) && child.id != target && set.contains(&child.id) {
            return true;
        }
        if is_ancestor_in_set(&child.children, target, set) {
            return true;
        }
    }
    false
}

/// True iff `target` equals `node` or appears in its subtree.
pub(in crate::document) fn descendant_contains(node: &Node, target: NodeId) -> bool {
    if node.id == target {
        return true;
    }
    node.children.iter().any(|c| descendant_contains(c, target))
}

pub(in crate::document) fn translate_subtree(node: &mut Node, dx: f32, dy: f32) {
    if node.bounds.size.x > 0.0 || node.bounds.size.y > 0.0 {
        node.bounds.origin.x += dx;
        node.bounds.origin.y += dy;
    }
    for child in &mut node.children {
        translate_subtree(child, dx, dy);
    }
}

/// Recursive helper for `Document::commit_property_edit`. Returns
/// `true` once the edit lands on the matching node.
pub(in crate::document) fn commit_property_walk(
    node: &mut Node,
    sel: NodeId,
    focus: PropertyFocus,
    value: f32,
) -> bool {
    if node.id == sel {
        match focus {
            PropertyFocus::PositionX => node.bounds.origin.x = value,
            PropertyFocus::PositionY => node.bounds.origin.y = value,
            PropertyFocus::SizeW => node.bounds.size.x = value.max(0.0),
            PropertyFocus::SizeH => node.bounds.size.y = value.max(0.0),
            PropertyFocus::Rotation => {
                // Property panel ships degrees; node stores radians.
                node.rotation = value.to_radians();
            }
            PropertyFocus::StrokeWidth => {
                let color = node.stroke.map(|s| s.color).unwrap_or(crate::Color::BLACK);
                node.stroke = Some(Stroke {
                    color,
                    width: value.max(0.0),
                });
            }
            // Hex inputs go through `Document::set_selected_color`
            // (Color isn't a single f32). Opacity + corner-radius
            // are visual-only until the node schema grows the
            // field — accept the edit but don't persist.
            PropertyFocus::PositionR
            | PropertyFocus::Opacity
            | PropertyFocus::FillHex
            | PropertyFocus::StrokeHex => {}
        }
        return true;
    }
    for child in &mut node.children {
        if commit_property_walk(child, sel, focus, value) {
            return true;
        }
    }
    false
}

/// Direction for `Document::reorder_selected` — picks which
/// neighbour the selected node swaps with in its parent's
/// children vec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReorderDirection {
    /// Towards the front of the paint order (higher index, drawn
    /// on top). Bound to `]`.
    Up,
    /// Towards the back of the paint order (lower index, drawn
    /// underneath). Bound to `[`.
    Down,
}

/// Recursive helper for `Document::delete_selected`. Returns true
/// when the target was found + removed from `children` or any of
/// its descendants' children.
pub(in crate::document) fn remove_from_children(children: &mut Vec<Node>, target: NodeId) -> bool {
    if let Some(idx) = children.iter().position(|n| n.id == target) {
        children.remove(idx);
        return true;
    }
    for child in children.iter_mut() {
        if remove_from_children(&mut child.children, target) {
            return true;
        }
    }
    false
}

/// Deep-clone `node`, allocating a fresh id from `next_id` for
/// every descendant. Bounds and other fields copy verbatim;
/// callers shift `bounds.origin` by an offset if they want the
/// clone to land visually beside the original.
/// True iff `node` and every descendant are editable (none have
/// `hidden` or `locked` set). Backs `Document::is_subtree_editable`.
pub(in crate::document) fn subtree_all_editable(node: &Node) -> bool {
    if node.hidden || node.locked {
        return false;
    }
    node.children.iter().all(subtree_all_editable)
}

pub(in crate::document) fn toggle_hidden_walk(children: &mut Vec<Node>, target: NodeId) -> bool {
    for child in children.iter_mut() {
        if child.id == target {
            child.hidden = !child.hidden;
            return true;
        }
        if toggle_hidden_walk(&mut child.children, target) {
            return true;
        }
    }
    false
}

pub(in crate::document) fn set_fill_type_walk(
    node: &mut Node,
    target: NodeId,
    fill_type: FillType,
) -> bool {
    if node.id == target {
        node.fill_type = fill_type;
        return true;
    }
    for child in &mut node.children {
        if set_fill_type_walk(child, target, fill_type) {
            return true;
        }
    }
    false
}

pub(in crate::document) fn toggle_collapsed_walk(children: &mut Vec<Node>, target: NodeId) -> bool {
    for child in children.iter_mut() {
        if child.id == target {
            child.collapsed = !child.collapsed;
            return true;
        }
        if toggle_collapsed_walk(&mut child.children, target) {
            return true;
        }
    }
    false
}

pub(in crate::document) fn toggle_locked_walk(children: &mut Vec<Node>, target: NodeId) -> bool {
    for child in children.iter_mut() {
        if child.id == target {
            child.locked = !child.locked;
            return true;
        }
        if toggle_locked_walk(&mut child.children, target) {
            return true;
        }
    }
    false
}

pub(in crate::document) fn deep_clone_with_new_ids(node: &Node, next_id: &mut u64) -> Node {
    let id = NodeId::new(*next_id);
    *next_id += 1;
    let children: Vec<Node> = node
        .children
        .iter()
        .map(|c| deep_clone_with_new_ids(c, next_id))
        .collect();
    Node {
        id,
        kind: node.kind.clone(),
        name: node.name.clone(),
        bounds: node.bounds,
        rotation: node.rotation,
        fill: node.fill,
        stroke: node.stroke,
        text: node.text.clone(),
        hidden: node.hidden,
        locked: node.locked,
        collapsed: node.collapsed,
        fill_type: node.fill_type,
        children,
    }
}

/// Recursive helper for `Document::duplicate_selected`. Returns
/// the new node's id when the target was found and a sibling
/// clone was inserted.
///
/// Before allocating any id, verifies the allocator has enough
/// headroom for the entire subtree (one id per node + one
/// trailing increment past the last mint). Returning None when
/// the subtree won't fit guarantees the document isn't left in a
/// half-cloned state on overflow.
pub(in crate::document) fn duplicate_in_children(
    children: &mut Vec<Node>,
    target: NodeId,
    next_id: &mut u64,
    offset: f32,
) -> Option<NodeId> {
    if let Some(idx) = children.iter().position(|n| n.id == target) {
        let size = subtree_size(&children[idx]);
        // The clone walk mints `size` ids and runs one final
        // `*next_id += 1` past the last mint. Both must stay
        // representable, so the required headroom is `size`.
        next_id.checked_add(size)?;
        let mut clone = deep_clone_with_new_ids(&children[idx], next_id);
        // Offset the clone so the user sees it next to the
        // original instead of pixel-perfectly stacked on top.
        // Matches TS `cloneNodesWithNewIds({ offset: 10 })`.
        shift_subtree(&mut clone, offset, offset);
        let new_id = clone.id;
        children.insert(idx + 1, clone);
        return Some(new_id);
    }
    for child in children.iter_mut() {
        if let Some(new_id) = duplicate_in_children(&mut child.children, target, next_id, offset) {
            return Some(new_id);
        }
    }
    None
}

/// Node count in `node`'s subtree (including `node`). Used as
/// `duplicate_in_children`'s headroom check so we reject up-front
/// when the allocator can't fit the entire clone without overflow.
pub(in crate::document) fn subtree_size(node: &Node) -> u64 {
    let mut n = 1u64;
    for child in &node.children {
        n = n.saturating_add(subtree_size(child));
    }
    n
}

pub(in crate::document) fn shift_subtree(node: &mut Node, dx: f32, dy: f32) {
    if node.bounds.size.x > 0.0 || node.bounds.size.y > 0.0 {
        node.bounds.origin.x += dx;
        node.bounds.origin.y += dy;
    }
    for child in &mut node.children {
        shift_subtree(child, dx, dy);
    }
}

/// Recursive helper for `Document::reorder_selected`.
pub(in crate::document) fn reorder_in_children(
    children: &mut Vec<Node>,
    target: NodeId,
    direction: ReorderDirection,
) -> bool {
    if let Some(idx) = children.iter().position(|n| n.id == target) {
        match direction {
            ReorderDirection::Up if idx + 1 < children.len() => {
                children.swap(idx, idx + 1);
                return true;
            }
            ReorderDirection::Down if idx > 0 => {
                children.swap(idx, idx - 1);
                return true;
            }
            _ => return false,
        }
    }
    for child in children.iter_mut() {
        if reorder_in_children(&mut child.children, target, direction) {
            return true;
        }
    }
    false
}

/// Recursive helper for `Document::max_node_id`.
pub(in crate::document) fn max_id_walk(node: &Node) -> u64 {
    let mut max = node.id.raw();
    for child in &node.children {
        max = max.max(max_id_walk(child));
    }
    max
}

pub(in crate::document) fn find_duplicate_walk(
    node: &Node,
    seen: &mut std::collections::HashSet<NodeId>,
) -> Option<NodeId> {
    if !seen.insert(node.id) {
        return Some(node.id);
    }
    for child in &node.children {
        if let Some(dup) = find_duplicate_walk(child, seen) {
            return Some(dup);
        }
    }
    None
}

/// Drain the node identified by `target` out of `children` (or any
/// descendant `children` vec) and return it. Returns `None` when the
/// target isn't found in the subtree. Backs the drag-to-reorder
/// extraction phase — the caller holds the extracted node on the
/// stack and then re-inserts it via `insert_before_in_children` /
/// `insert_after_in_children`.
pub(in crate::document) fn extract_node(children: &mut Vec<Node>, target: NodeId) -> Option<Node> {
    if let Some(idx) = children.iter().position(|n| n.id == target) {
        return Some(children.remove(idx));
    }
    for child in children.iter_mut() {
        if let Some(extracted) = extract_node(&mut child.children, target) {
            return Some(extracted);
        }
    }
    None
}

/// Insert `node` immediately before `anchor` in `children` (or any
/// descendant `children` vec). On success returns `Ok(())`; on miss
/// returns `Err(node)` so the caller can recover the orphaned
/// payload. Pre-check the anchor via `children_contain_descendant`
/// if you want to avoid the Err arm entirely.
pub(in crate::document) fn insert_before_in_children(
    children: &mut Vec<Node>,
    anchor: NodeId,
    node: Node,
) -> Result<(), Node> {
    if let Some(idx) = children.iter().position(|n| n.id == anchor) {
        children.insert(idx, node);
        return Ok(());
    }
    let mut carry = node;
    for child in children.iter_mut() {
        match insert_before_in_children(&mut child.children, anchor, carry) {
            Ok(()) => return Ok(()),
            Err(returned) => carry = returned,
        }
    }
    Err(carry)
}

/// Insert `node` immediately after `anchor` in `children` (or any
/// descendant `children` vec). On success returns `Ok(())`; on miss
/// returns `Err(node)`.
pub(in crate::document) fn insert_after_in_children(
    children: &mut Vec<Node>,
    anchor: NodeId,
    node: Node,
) -> Result<(), Node> {
    if let Some(idx) = children.iter().position(|n| n.id == anchor) {
        children.insert(idx + 1, node);
        return Ok(());
    }
    let mut carry = node;
    for child in children.iter_mut() {
        match insert_after_in_children(&mut child.children, anchor, carry) {
            Ok(()) => return Ok(()),
            Err(returned) => carry = returned,
        }
    }
    Err(carry)
}

/// True iff `anchor_id` is found anywhere in the children forest
/// rooted at `children`. Cheap O(n) scan used as a pre-check before
/// `extract_node` so callers don't have to handle the lost-node case.
pub(in crate::document) fn children_contain_descendant(
    children: &[Node],
    anchor_id: NodeId,
) -> bool {
    for child in children {
        if child.id == anchor_id {
            return true;
        }
        if children_contain_descendant(&child.children, anchor_id) {
            return true;
        }
    }
    false
}
