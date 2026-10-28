//! Page assembly: per-page payload build, root-frame clip marking and
//! the taffy layout pass that harvests computed rects.

use super::*;

/// Resolve one page into the paint payload used by the editor scene.
///
/// The full-document adapters above remain available to export/validation
/// callers. Interactive hosts use this focused entry point so switching pages
/// does not lay out every inactive page or retain their duplicate payloads.
pub(crate) fn pen_roots_to_page_payload(
    id: &str,
    name: &str,
    roots: &[PenNode],
    page_idx: usize,
    preserve_authored_geometry: bool,
) -> PagePayload {
    if preserve_authored_geometry {
        build_page_preserving_geometry(id, name, roots)
    } else {
        build_page(id, name, roots, page_idx)
    }
}

pub(super) fn build_page(id: &str, name: &str, roots: &[PenNode], page_idx: usize) -> PagePayload {
    let _ = page_idx;
    build_page_with_measure(id, name, roots, layout_measure_backend())
}

pub(super) fn build_page_with_measure(
    id: &str,
    name: &str,
    roots: &[PenNode],
    measure: Rc<dyn MeasureBackend>,
) -> PagePayload {
    let mut layout_rects: BTreeMap<String, [f32; 4]> = BTreeMap::new();
    for root in roots {
        compute_layout_with_measure(root, &mut layout_rects, Rc::clone(&measure));
    }
    let mut children: Vec<NodePayload> = roots
        .iter()
        .map(|n| node_to_payload(n, &layout_rects))
        .collect();
    mark_root_frame_clips(roots, &mut children);
    PagePayload {
        id: id.to_string(),
        name: name.to_string(),
        children,
    }
}

pub(super) fn build_page_preserving_geometry(
    id: &str,
    name: &str,
    roots: &[PenNode],
) -> PagePayload {
    let rects = crate::authored_geometry::rects_for_roots(roots);
    let mut children: Vec<NodePayload> = roots.iter().map(|n| node_to_payload(n, &rects)).collect();
    mark_root_frame_clips(roots, &mut children);
    PagePayload {
        id: id.to_string(),
        name: name.to_string(),
        children,
    }
}

/// Legacy TS flattener parity (`document-flattener.ts`): ROOT frames whose
/// canonical `clipContent` field is absent clip like artboards. An explicit
/// `clipContent: false` is authored geometry and must remain open. Pair the
/// source roots with their payloads so the DTO can keep its compact `bool`.
pub(super) fn mark_root_frame_clips(roots: &[PenNode], children: &mut [NodePayload]) {
    for (root, child) in roots.iter().zip(children) {
        if matches!(
            root,
            PenNode::Frame(frame) if frame.container.clip_content.is_none()
        ) && child.kind == "frame"
        {
            child.clip_content = true;
        }
    }
}

/// Run jian-core's `LayoutEngine` on `root` and harvest absolute
/// rects per schema id into `out`. Each page root gets its own
/// `LayoutEngine` instance — OpenPencil's canvas is infinite, so
/// roots don't share a coordinate frame.
///
/// Merge note (responsive-m1a into main): `LayoutEngine::node_rect`
/// now bakes a root's own authored `(base.x, base.y)` into its
/// returned absolute rect itself (`root_origins` / `is_origin_normalized`
/// in jian-core's `layout/mod.rs`, added for the responsive runtime's
/// multi-root canvas) — this used to be this function's OWN job (see
/// git history for the prior manual `+ root_ox + root_oy` add here).
/// Doing both doubled every harvested rect's origin (measured: a root
/// authored at doc (400, 60) resolved to scene (800, 120) — root_ox/
/// root_oy added on top of jian-core's own addition). `root_authored_
/// origin` stays exported for `op-host-native`'s Canvas Preview tap
/// translation, which still needs the authored origin as a standalone
/// value (not baked into a rect).
pub(super) fn compute_layout_with_measure(
    root: &PenNode,
    out: &mut BTreeMap<String, [f32; 4]>,
    measure: Rc<dyn MeasureBackend>,
) {
    let (root_w, root_h) = root_available_size(root);
    let mut tree = NodeTree::new();
    let mut layout_root = root.clone();
    crate::layout_repair::normalize_fill_row_coordinates(&mut layout_root);
    tree.insert_subtree(layout_root, None);
    // Preview must use its injected runtime backend for paint geometry too.
    let mut engine = LayoutEngine::with_backend(measure);
    let Ok(taffy_roots) = engine.build(&tree) else {
        return;
    };
    let Some(root_id) = taffy_roots.first() else {
        return;
    };
    if engine.compute(*root_id, (root_w, root_h)).is_err() {
        return;
    }
    // Walk every node by SlotMap iteration, looking up its absolute
    // rect via `node_rect`. Keyed back by the schema id we stashed
    // in `tree.by_id` during insertion. `node_rect` already carries
    // the root's authored canvas offset (see the merge note above),
    // so each design sits where the file placed it without any
    // further adjustment here.
    for (id_str, node_key) in tree.by_id.iter() {
        if let Some(rect) = engine.node_rect(*node_key) {
            out.insert(
                id_str.clone(),
                [
                    rect.origin.x,
                    rect.origin.y,
                    rect.size.width,
                    rect.size.height,
                ],
            );
        }
    }
    crate::layout_repair::repair_fit_content_layout(root, out);
}

/// The canvas's selected backend, also used by runtimes that share its geometry.
pub fn layout_measure_backend() -> Rc<dyn MeasureBackend> {
    LAYOUT_MEASURE_BACKEND.with(Rc::clone)
}
