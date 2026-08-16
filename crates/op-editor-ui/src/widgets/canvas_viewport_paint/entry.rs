//! Public / crate-level entry points into the canvas painter, split out
//! of `canvas_viewport_paint.rs` to keep that spine under the
//! repository's 800-line cap. Every function is re-exported from the
//! spine so existing `canvas_viewport_paint::…` paths keep resolving.

use super::mask::paint_child_siblings;
use super::{paint_node_inner, PaintNodeHits, PaintNodeOptions, RevealSchedule};
use crate::layout_scene::SceneNode;
use crate::widgets::canvas_viewport::EditCaret;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};
use std::collections::HashSet;
/// Off-canvas public entry for painters outside this crate (raster/PDF
/// export, `debug_screenshot` in `op-host-desktop`). Paints the base
/// scene only — no editor overlays (reveal animation, hover outline,
/// selection highlight, pen preview) and no caret — forwarding to
/// [`paint_node_with_options`]. Keeps the cross-crate surface to types
/// that are already `pub` (`PaintCx` / `SceneNode` / `Point2D` / `Rect`)
/// so the overlay-only helper types stay crate-private.
pub fn paint_node(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    viewport_origin: Point2D,
    zoom: f32,
    cull: Rect,
) {
    let _ = paint_node_with_options(
        cx,
        node,
        viewport_origin,
        zoom,
        None,
        cull,
        None,
        None,
        None,
        None,
    );
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_node_with_options<'a>(
    cx: &mut PaintCx<'_>,
    node: &'a SceneNode,
    viewport_origin: Point2D,
    zoom: f32,
    edit_caret: Option<EditCaret>,
    cull: Rect,
    reveals: Option<RevealSchedule<'a>>,
    hovered: Option<&'a str>,
    selected: Option<&'a str>,
    pen: Option<&'a str>,
) -> PaintNodeHits<'a> {
    paint_node_with_options_hiding(
        cx,
        node,
        viewport_origin,
        zoom,
        edit_caret,
        cull,
        reveals,
        hovered,
        selected,
        pen,
        None,
        0,
        None,
        None,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_node_with_options_hiding<'a>(
    cx: &mut PaintCx<'_>,
    node: &'a SceneNode,
    viewport_origin: Point2D,
    zoom: f32,
    edit_caret: Option<EditCaret>,
    cull: Rect,
    reveals: Option<RevealSchedule<'a>>,
    hovered: Option<&'a str>,
    selected: Option<&'a str>,
    pen: Option<&'a str>,
    hidden: Option<&'a str>,
    now_ms: u64,
    generating_descendant_ids: Option<&HashSet<String>>,
    generation_accent: Option<Color>,
    queued_shell_ids: Option<&HashSet<String>>,
) -> PaintNodeHits<'a> {
    let options = PaintNodeOptions {
        viewport_origin,
        zoom,
        edit_caret,
        cull,
        reveals,
        hovered,
        selected,
        pen,
        hidden,
        now_ms,
        generating_descendant_ids,
        generation_accent,
        queued_shell_ids,
        mask_source: false,
        suppress_node_composite_id: None,
        fast_interaction: false,
    };
    paint_node_inner(cx, node, &options, &mut Vec::new(), false)
}

/// Paint a topmost-first sibling list with the same mask semantics used by
/// nested containers. Keeping page roots on this path is essential because a
/// Figma mask may legally be a direct child of the canvas.
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_scene_nodes_with_options_hiding<'a>(
    cx: &mut PaintCx<'_>,
    nodes: &'a [SceneNode],
    viewport_origin: Point2D,
    zoom: f32,
    edit_caret: Option<EditCaret>,
    cull: Rect,
    reveals: Option<RevealSchedule<'a>>,
    hovered: Option<&'a str>,
    selected: Option<&'a str>,
    pen: Option<&'a str>,
    hidden: Option<&'a str>,
    now_ms: u64,
    generating_descendant_ids: Option<&HashSet<String>>,
    generation_accent: Option<Color>,
    queued_shell_ids: Option<&HashSet<String>>,
    fast_interaction: bool,
) -> PaintNodeHits<'a> {
    let options = PaintNodeOptions {
        viewport_origin,
        zoom,
        edit_caret,
        cull,
        reveals,
        hovered,
        selected,
        pen,
        hidden,
        now_ms,
        generating_descendant_ids,
        generation_accent,
        queued_shell_ids,
        mask_source: false,
        suppress_node_composite_id: None,
        fast_interaction,
    };
    let mut hits = PaintNodeHits::default();
    paint_child_siblings(cx, nodes, &options, &mut Vec::new(), false, &mut hits);
    hits
}

/// Paint a resolved scene page's node tree with the editor viewport
/// transform applied, WITHOUT any editor chrome (no selection outline /
/// handles / hover / grid / reveal animation / text-edit caret).
///
/// The Canvas Preview (Play) path uses this to render the live document
/// through the SAME mature painter the design canvas uses — so preview
/// is pixel-identical to the design surface (root offsets, images,
/// gradients, shadows, real text metrics) instead of jian's separate
/// MVP scene walker. The host (`op-host-native::preview`) overlays live
/// widget runtime state into the scene before calling this, and paints
/// its own focus caret on top.
///
/// `viewport_origin` is `canvas_rect.origin + (pan_x, pan_y)`; `cull`
/// is the canvas rect grown by the standard margin. Children paint
/// back-to-front (`.rev()`), matching [`super::canvas_viewport::CanvasViewport`]'s
/// own walk so z-order is identical.
pub fn paint_scene_page(
    cx: &mut PaintCx<'_>,
    page: &crate::layout_scene::ScenePage,
    viewport_origin: Point2D,
    zoom: f32,
    cull: Rect,
) {
    paint_scene_page_with_options(cx, page, viewport_origin, zoom, cull, None);
}

/// Like [`paint_scene_page`], but with an optional inline text-edit
/// caret: the painter then renders the edited node's draft text +
/// composition + caret (the mobile players' text-edit path).
pub fn paint_scene_page_with_options(
    cx: &mut PaintCx<'_>,
    page: &crate::layout_scene::ScenePage,
    viewport_origin: Point2D,
    zoom: f32,
    cull: Rect,
    edit_caret: Option<EditCaret>,
) {
    let _ = paint_scene_nodes_with_options_hiding(
        cx,
        &page.children,
        viewport_origin,
        zoom,
        edit_caret,
        cull,
        None,
        None,
        None,
        None,
        None,
        0,
        None,
        None,
        None,
        false,
    );
}
