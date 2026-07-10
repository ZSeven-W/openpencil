//! Preview ↔ design-canvas GEOMETRY parity tests.
//!
//! The user-visible bug: elements misalign when entering Preview. The
//! design canvas and the preview session build their `LayoutScene`s
//! through different paths, so any divergence between those paths
//! paints elements at different positions in the two modes. These
//! tests pin the invariant: for the SAME document, node bounds in the
//! preview scene must equal node bounds in the design scene.
//!
//! Covered divergences:
//! - Preserve-geometry documents (Figma imports set
//!   `preserve_authored_geometry`): the design canvas paints authored
//!   rects; preview must not silently re-run the flex solver.
//! - Legacy role-frame promotion: preview promotes `role=input` frames
//!   to widget leaves before layout; the promoted tree must not shift
//!   sibling geometry relative to the design canvas.
//! - Plain free-layout documents: control group — identical today,
//!   must stay identical.

#![cfg(test)]

use super::PreviewSession;
use op_editor_ui::layout_scene::{LayoutScene, SceneNode};

fn default_theme() -> std::collections::BTreeMap<String, String> {
    std::collections::BTreeMap::new()
}

fn load(src: &str) -> jian_ops_schema::PenDocument {
    jian_ops_schema::load_str(src)
        .expect("parse test doc")
        .value
}

fn find<'a>(scene: &'a LayoutScene, id: &str) -> Option<&'a SceneNode> {
    scene.active_page().and_then(|p| p.find(id))
}

/// The design-canvas scene for `doc`, exactly as the editor paints it.
fn design_scene(
    doc: &jian_ops_schema::PenDocument,
    preserve_authored_geometry: bool,
) -> LayoutScene {
    let mut state = op_editor_core::EditorState::from_document(doc.clone());
    state.editor_ui.preserve_authored_geometry = preserve_authored_geometry;
    op_pen_loader::editor_state_to_layout_scene(&state)
}

/// Assert `id` occupies the same rect in both scenes (±0.5 px).
fn assert_same_bounds(design: &LayoutScene, preview: &LayoutScene, id: &str) {
    let d = find(design, id)
        .unwrap_or_else(|| panic!("node {id} missing from design scene"))
        .bounds;
    let p = find(preview, id)
        .unwrap_or_else(|| panic!("node {id} missing from preview scene"))
        .bounds;
    let close = |a: f32, b: f32| (a - b).abs() <= 0.5;
    assert!(
        close(d.origin.x, p.origin.x)
            && close(d.origin.y, p.origin.y)
            && close(d.size.x, p.size.x)
            && close(d.size.y, p.size.y),
        "node {id} misaligned in preview:\n  design  ({}, {}) {}x{}\n  preview ({}, {}) {}x{}",
        d.origin.x,
        d.origin.y,
        d.size.x,
        d.size.y,
        p.origin.x,
        p.origin.y,
        p.size.x,
        p.size.y,
    );
}

/// A Figma-import-shaped document: authored parent-local geometry that
/// deliberately DISAGREES with what the flex solver would compute (a
/// vertical auto-layout root whose second child is authored far from
/// the stacked position — exactly what Preserve-mode imports carry).
fn preserve_geometry_doc() -> jian_ops_schema::PenDocument {
    load(
        r##"{
        "version": "0.8.0",
        "children": [
            { "type": "frame", "id": "root", "x": 0, "y": 0,
              "width": 400, "height": 300,
              "layout": "vertical", "gap": 8, "padding": 16,
              "fill": [{"type":"solid","color":"#ffffff"}],
              "children": [
                { "type": "rectangle", "id": "a", "x": 16, "y": 16,
                  "width": 60, "height": 20,
                  "fill": [{"type":"solid","color":"#ff0000"}] },
                { "type": "rectangle", "id": "b", "x": 120, "y": 210,
                  "width": 60, "height": 20,
                  "fill": [{"type":"solid","color":"#00ff00"}] }
              ] }
        ]
    }"##,
    )
}

#[test]
fn preview_matches_design_for_preserve_geometry_doc() {
    // Text measurement must not race a concurrent font-registry
    // mutation between the two scene builds (the import tests register
    // real faces process-globally).
    let _guard = crate::font_registry_test_support::lock();
    // Figma Preserve import: the design canvas honors authored rects
    // (`preserve_authored_geometry = true` skips the flex pass). The
    // preview must paint child `b` at the same authored spot — not at
    // the flex-solved stacked position.
    let doc = preserve_geometry_doc();
    let design = design_scene(&doc, true);
    let session = PreviewSession::enter(&doc, (800.0, 600.0), &default_theme(), 0, true)
        .expect("enter preview");
    let preview = session.preview_scene_for_test();
    assert_same_bounds(&design, &preview, "a");
    assert_same_bounds(&design, &preview, "b");
}

/// An AI-generation-shaped document: a vertical auto-layout root where a
/// legacy `role=input` frame (hug height, sized by its text child) is
/// followed by a sibling. Preview promotes the role frame to a leaf
/// `text_input` widget before layout; if the promoted leaf measures
/// differently than the frame+child, the sibling below shifts.
fn role_frame_doc() -> jian_ops_schema::PenDocument {
    load(
        r##"{
        "version": "0.8.0",
        "children": [
            { "type": "frame", "id": "root", "x": 0, "y": 0,
              "width": 400, "height": 600,
              "layout": "vertical", "gap": 12, "padding": 16,
              "fill": [{"type":"solid","color":"#ffffff"}],
              "children": [
                { "type": "frame", "id": "emailField", "role": "input",
                  "width": 320, "padding": 12,
                  "layout": "horizontal", "gap": 8,
                  "stroke": {"color": "#d0d0d0", "thickness": 1},
                  "cornerRadius": 8,
                  "children": [
                    { "type": "text", "id": "ph", "content": "you@example.com",
                      "fontSize": 14,
                      "fill": [{"type":"solid","color":"#999999"}] }
                  ] },
                { "type": "text", "id": "below", "content": "Below the field",
                  "fontSize": 14,
                  "fill": [{"type":"solid","color":"#000000"}] }
              ] }
        ]
    }"##,
    )
}

#[test]
fn preview_matches_design_for_promoted_role_frame_doc() {
    // Text measurement must not race a concurrent font-registry
    // mutation between the two scene builds (the import tests register
    // real faces process-globally).
    let _guard = crate::font_registry_test_support::lock();
    // The design canvas lays out the UNPROMOTED tree (role frame with a
    // text child); preview lays out the PROMOTED tree (leaf text_input).
    // The field's own box and the sibling below it must not move.
    let doc = role_frame_doc();
    let design = design_scene(&doc, false);
    let session = PreviewSession::enter(&doc, (800.0, 600.0), &default_theme(), 0, false)
        .expect("enter preview");
    let preview = session.preview_scene_for_test();
    assert_same_bounds(&design, &preview, "emailField");
    assert_same_bounds(&design, &preview, "below");
}

#[test]
fn held_drag_stays_anchored_to_the_down_node() {
    // Pointer capture across the scene→runtime remap: once a gesture
    // anchors on a node whose scene and runtime rects diverge (e.g. a
    // promoted widget's hug drift), every held Move and the Up must map
    // through THAT rect pair — re-resolving per event would remap a
    // drag through whatever node the pointer crosses, teleporting the
    // drag and potentially activating a widget the pointer isn't
    // visually over. The anchor is installed synthetically because the
    // divergence is engine-dependent; the capture SEMANTICS are what
    // this test pins.
    use jian_core::gesture::pointer::PointerPhase;
    use op_editor_ui::{Point2D, Rect};
    let doc = preserve_geometry_doc();
    let mut session = PreviewSession::enter(&doc, (800.0, 600.0), &default_theme(), 0, true)
        .expect("enter preview");

    // Simulate a Down that anchored on a node painted at (120,210)
    // 60x20 whose runtime copy sits at (16,44) 60x20.
    let scene_rect = Rect {
        origin: Point2D::new(120.0, 210.0),
        size: Point2D::new(60.0, 20.0),
    };
    let runtime_rect = Rect {
        origin: Point2D::new(16.0, 44.0),
        size: Point2D::new(60.0, 20.0),
    };
    session.set_gesture_mapping_for_test(scene_rect, runtime_rect);

    // Held Move at the node's scene center maps to its runtime center.
    let (mx, my) = session.resolve_runtime_point_for_test(150.0, 220.0, PointerPhase::Move);
    assert!(
        (mx - 46.0).abs() <= 0.5 && (my - 54.0).abs() <= 0.5,
        "held Move should map through the anchored pair, got ({mx}, {my})"
    );

    // Held Move 20px PAST the node's right edge extrapolates through
    // the same pair (x = 16 + (200-120)/60 * 60 = 96) — it must NOT
    // re-resolve through the node under the new point (the root's
    // identity map would yield 200).
    let (px, py) = session.resolve_runtime_point_for_test(200.0, 220.0, PointerPhase::Move);
    assert!(
        (px - 96.0).abs() <= 0.5 && (py - 54.0).abs() <= 0.5,
        "held Move past the edge must stay in the anchor's space, got ({px}, {py})"
    );

    // Up consumes the anchor; the next unpressed resolve at the same
    // point goes back to fresh mapping (identity here: this fixture's
    // runtime honors the authored rects, so scene == runtime).
    session.resolve_runtime_point_for_test(200.0, 220.0, PointerPhase::Up);
    let (hx, hy) = session.resolve_runtime_point_for_test(200.0, 220.0, PointerPhase::Hover);
    assert!(
        (hx - 200.0).abs() <= 0.5 && (hy - 220.0).abs() <= 0.5,
        "after Up the anchor must be released (fresh mapping), got ({hx}, {hy})"
    );
}

/// Control group: a plain hand-drawn free-layout document (absolute
/// rects, no auto-layout, no roles, no tokens) must render identically
/// in both modes.
#[test]
fn preview_matches_design_for_free_layout_doc() {
    // Text measurement must not race a concurrent font-registry
    // mutation between the two scene builds (the import tests register
    // real faces process-globally).
    let _guard = crate::font_registry_test_support::lock();
    let doc = load(
        r##"{
        "version": "0.8.0",
        "children": [
            { "type": "frame", "id": "root", "x": 40, "y": 40,
              "width": 300, "height": 200,
              "fill": [{"type":"solid","color":"#ffffff"}],
              "children": [
                { "type": "rectangle", "id": "r1", "x": 60, "y": 60,
                  "width": 80, "height": 30,
                  "fill": [{"type":"solid","color":"#3366ff"}] },
                { "type": "text", "id": "t1", "x": 60, "y": 120,
                  "content": "Hand drawn", "fontSize": 16,
                  "fill": [{"type":"solid","color":"#000000"}] }
              ] }
        ]
    }"##,
    );
    let design = design_scene(&doc, false);
    let session = PreviewSession::enter(&doc, (800.0, 600.0), &default_theme(), 0, false)
        .expect("enter preview");
    let preview = session.preview_scene_for_test();
    assert_same_bounds(&design, &preview, "root");
    assert_same_bounds(&design, &preview, "r1");
    assert_same_bounds(&design, &preview, "t1");
}
