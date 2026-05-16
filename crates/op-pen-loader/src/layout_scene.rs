//! `EditorState` → [`LayoutScene`] builder.
//!
//! Produces the paint-only, layout-resolved render scene that a
//! future `CanvasViewport` painter walks instead of the editor's
//! `Document`.
//!
//! The flex layout pass is NOT re-implemented here. `EditorState.doc`
//! is a `PenDocument`; [`pen_document_to_document`] already runs each
//! page-root through jian-core's taffy `LayoutEngine` +
//! `jian_skia::SkiaMeasure` (see `adapter.rs`) and bakes the resolved
//! absolute AABBs into a shell-core `Document`. This builder reuses
//! that exact resolved `Document` and re-shapes its `Node` tree into
//! [`SceneNode`]s, dropping all editor state (selection / chat /
//! history / ui) and resolving variable `$ref` fills against the
//! editor's variables + active theme.
//!
//! So the resolved geometry a `LayoutScene` carries is bit-identical
//! to what `pen_document_to_document` bakes today — there is one
//! layout pass and one set of resolved rects.

use openpencil_shell_core::document::{Document, Node, VariableTable};
use openpencil_shell_core::layout_scene::{
    LayoutScene, SceneFillType, SceneNode, SceneStroke, ScenePage,
};

use crate::editor_state_var_table;
use crate::payload::pen_document_to_document;

/// Build a paint-only [`LayoutScene`] from an editor state.
///
/// Runs the same jian `LayoutEngine` + `SkiaMeasure` flex pass that
/// [`pen_document_to_document`] uses (by delegating to it), resolves
/// variable `$ref` fills / strokes against the editor's variables +
/// active theme (via [`editor_state_var_table`]), and re-shapes the
/// resolved node tree into a render scene that carries NO editor
/// state.
pub fn editor_state_to_layout_scene(state: &op_editor_core::EditorState) -> LayoutScene {
    // The resolved `Document` — flex layout already baked into AABBs.
    let doc: Document = pen_document_to_document(&state.doc);
    // Variables + active theme + the `fill_refs` / `stroke_refs`
    // caches the editor holds. `pen_document_to_document` returns a
    // `Document` whose `var_table` only carries the persisted
    // definitions; the transient `fill_refs` / `stroke_refs` live on
    // `EditorState.ui` — so resolve `$ref`s against the editor-state
    // table, which folds both halves together.
    let var_table: VariableTable = editor_state_var_table(state);

    LayoutScene {
        pages: doc
            .pages
            .iter()
            .map(|page| ScenePage {
                id: page.id.as_str().to_string(),
                name: page.name.clone(),
                children: page
                    .children
                    .iter()
                    .map(|n| node_to_scene(n, &var_table))
                    .collect(),
            })
            .collect(),
        active_page_index: doc.active_page_index,
    }
}

/// Convert one resolved shell-core [`Node`] into a [`SceneNode`].
///
/// Geometry is copied straight through — `pen_document_to_document`
/// already resolved it. Variable `$ref` fills / strokes are resolved
/// here so the scene carries only concrete colours; a registered ref
/// wins over the node's authored colour, mirroring the canvas
/// painter's `var_table.fill_for(id).or(node.fill)`.
fn node_to_scene(node: &Node, var_table: &VariableTable) -> SceneNode {
    SceneNode {
        id: node.id.as_str().to_string(),
        kind: node.kind.clone(),
        bounds: node.bounds,
        rotation: node.rotation,
        corner_radius: node.corner_radius,
        // Paint-time `$ref` resolution: a registered fill ref wins,
        // else the node's own fill. Same precedence as the canvas
        // painter's `node_fill` helper.
        fill: var_table.fill_for(&node.id).or(node.fill),
        fill_type: fill_type_to_scene(node.fill_type),
        stroke: node.stroke.map(|s| SceneStroke {
            // Stroke `$ref` resolution parallels the fill path.
            color: var_table
                .stroke_color_for(&node.id)
                .unwrap_or(s.color),
            width: s.width,
        }),
        text: node.text.clone(),
        font_size: node.font_size,
        font_weight: node.font_weight,
        text_wrap: node.text_wrap,
        points: node.points.clone(),
        effects: node.effects.clone(),
        hidden: node.hidden,
        children: node
            .children
            .iter()
            .map(|c| node_to_scene(c, var_table))
            .collect(),
    }
}

/// Map shell-core's editor-model `FillType` onto the scene's own
/// `SceneFillType`. A dedicated enum keeps `LayoutScene` from
/// re-exporting an editor-model type that may diverge.
fn fill_type_to_scene(
    ft: openpencil_shell_core::document::FillType,
) -> SceneFillType {
    use openpencil_shell_core::document::FillType;
    match ft {
        FillType::Solid => SceneFillType::Solid,
        FillType::LinearGradient => SceneFillType::LinearGradient,
        FillType::RadialGradient => SceneFillType::RadialGradient,
        FillType::Image => SceneFillType::Image,
    }
}

#[cfg(test)]
#[path = "layout_scene_tests.rs"]
mod tests;
