//! `EditorState` → [`LayoutScene`] builder.
//!
//! Produces the paint-only, layout-resolved render scene that the
//! `CanvasViewport` painter walks.
//!
//! The flex layout pass is NOT re-implemented here. `EditorState.doc`
//! is a `PenDocument`; [`pen_document_to_payload`] runs each page-root
//! through jian-core's taffy `LayoutEngine` + `jian_skia::SkiaMeasure`
//! (see `adapter.rs`) and bakes the resolved absolute AABBs — plus
//! every paint field — into a layout-resolved [`DocPayload`]. This
//! builder reuses that resolved payload and re-shapes its `NodePayload`
//! tree into [`SceneNode`]s, dropping all editor state (selection /
//! chat / history / ui) and resolving variable `$ref` fills against
//! the editor's variables + active theme.
//!
//! So the resolved geometry a `LayoutScene` carries is bit-identical
//! to what `pen_document_to_payload` bakes — there is one layout pass
//! and one set of resolved rects. The builder no longer routes through
//! the shell-core `Document` model: `DocPayload` already carries the
//! resolved geometry + paint fields, and `apply_payload`'s only
//! transforms on them are lossless format conversions (colour array →
//! struct, kind / fill-type string → enum).

use openpencil_shell_core::document::{NodeKind, VariableTable};
use openpencil_shell_core::layout_scene::{
    LayoutScene, SceneFillType, SceneNode, SceneStroke, ScenePage,
};
use openpencil_shell_core::Color;

use crate::editor_state_var_table;
use crate::payload::{DocPayload, NodePayload, StrokePayload};

/// Build a paint-only [`LayoutScene`] from an editor state.
///
/// Runs the same jian `LayoutEngine` + `SkiaMeasure` flex pass that
/// the canonical `.op` loader uses (via [`pen_document_to_payload`]),
/// resolves variable `$ref` fills / strokes against the editor's
/// variables + active theme (via [`editor_state_var_table`]), and
/// re-shapes the resolved node tree into a render scene that carries
/// NO editor state.
pub fn editor_state_to_layout_scene(state: &op_editor_core::EditorState) -> LayoutScene {
    // The layout-resolved payload — flex layout already baked into
    // every `NodePayload`'s AABB by jian-core's `LayoutEngine`. This
    // is the reusable layout-resolution core; it never touches the
    // shell-core `Document` model.
    let payload: DocPayload = crate::adapter::pen_document_to_payload(&state.doc).payload;
    // Variables + active theme + the `fill_refs` / `stroke_refs`
    // caches the editor holds. `editor_state_var_table` folds the
    // persisted definitions and the transient `EditorState.ui`
    // selection / caches together.
    let var_table: VariableTable = editor_state_var_table(state);

    LayoutScene {
        pages: payload
            .pages
            .iter()
            .map(|page| ScenePage {
                id: page.id.clone(),
                name: page.name.clone(),
                children: page
                    .children
                    .iter()
                    .map(|n| node_payload_to_scene(n, &var_table))
                    .collect(),
            })
            .collect(),
        // The loader always opens on page 0 (`pen_document_to_payload`
        // hardcodes `active_page_index: 0`, then `apply_payload` clamps
        // it into range — same result for a fresh load).
        active_page_index: payload
            .active_page_index
            .min(payload.pages.len().saturating_sub(1)),
    }
}

/// Convert one resolved [`NodePayload`] into a [`SceneNode`].
///
/// Geometry is copied straight through — `pen_document_to_payload`
/// already resolved it. Variable `$ref` fills / strokes are resolved
/// here so the scene carries only concrete colours; a registered ref
/// wins over the node's authored colour, mirroring the canvas
/// painter's `var_table.fill_for(id).or(node.fill)`.
fn node_payload_to_scene(node: &NodePayload, var_table: &VariableTable) -> SceneNode {
    use openpencil_shell_core::{Point2D, Rect};
    let node_id = openpencil_shell_core::document::NodeId::new(node.id.clone());
    SceneNode {
        id: node.id.clone(),
        kind: str_to_kind(&node.kind),
        bounds: Rect {
            origin: Point2D::new(node.x, node.y),
            size: Point2D::new(node.w, node.h),
        },
        rotation: node.rotation,
        corner_radius: node.corner_radius,
        // Paint-time `$ref` resolution: a registered fill ref wins,
        // else the node's own fill. Same precedence as the canvas
        // painter's `node_fill` helper.
        fill: var_table
            .fill_for(&node_id)
            .or_else(|| node.fill.map(array_to_color)),
        fill_type: str_to_scene_fill_type(&node.fill_type),
        stroke: node.stroke.as_ref().map(|s| scene_stroke(s, &node_id, var_table)),
        text: node.text.clone(),
        font_size: node.font_size,
        font_weight: node.font_weight,
        text_wrap: node.text_wrap,
        points: node
            .points
            .iter()
            .map(|p| Point2D::new(p[0], p[1]))
            .collect(),
        effects: crate::effects::effects_from_payload_ref(&node.effects),
        hidden: node.hidden,
        locked: node.locked,
        children: node
            .children
            .iter()
            .map(|c| node_payload_to_scene(c, var_table))
            .collect(),
    }
}

/// Resolve a payload stroke into a scene stroke. The `$ref` stroke
/// resolution parallels the fill path.
fn scene_stroke(
    s: &StrokePayload,
    node_id: &openpencil_shell_core::document::NodeId,
    var_table: &VariableTable,
) -> SceneStroke {
    SceneStroke {
        color: var_table
            .stroke_color_for(node_id)
            .unwrap_or_else(|| array_to_color(s.color)),
        width: s.width,
    }
}

/// `[r, g, b, a]` payload colour → shell-core `Color`. Lossless;
/// the same conversion `apply_payload` runs on the `Document` path.
fn array_to_color(a: [f32; 4]) -> Color {
    Color {
        r: a[0],
        g: a[1],
        b: a[2],
        a: a[3],
    }
}

/// `NodePayload.kind` string → shell-core `NodeKind`. Mirrors
/// `payload::str_to_kind` so the scene's per-kind paint dispatch
/// matches the `Document` path exactly.
fn str_to_kind(s: &str) -> NodeKind {
    match s {
        "frame" => NodeKind::Frame,
        "group" => NodeKind::Group,
        "rect" => NodeKind::Rect,
        "ellipse" => NodeKind::Ellipse,
        "polygon" => NodeKind::Polygon,
        "line" => NodeKind::Line,
        "text" => NodeKind::Text,
        "path" => NodeKind::Path,
        other => NodeKind::Other(other.to_string()),
    }
}

/// `NodePayload.fill_type` string → scene `SceneFillType`. Mirrors
/// `payload::str_to_fill_type` followed by `fill_type_to_scene`.
fn str_to_scene_fill_type(s: &str) -> SceneFillType {
    match s {
        "linear" => SceneFillType::LinearGradient,
        "radial" => SceneFillType::RadialGradient,
        "image" => SceneFillType::Image,
        _ => SceneFillType::Solid,
    }
}

#[cfg(test)]
#[path = "layout_scene_tests.rs"]
mod tests;
