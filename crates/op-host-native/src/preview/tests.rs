//! Preview-mode session tests (Phase D5 + preview-render rework).
//!
//! Preview now renders the live document through the design-canvas scene
//! painter, overlaying live widget runtime state onto the `LayoutScene`,
//! and hit-tests by translating scene-space taps into the runtime's
//! root-relative space. These tests cover the invariants testable
//! without a live skia surface: document byte-invariance across
//! enter→input→exit, typed text reaching the runtime, the live overlay
//! reflecting runtime state (typed text, toggles), resolved colours /
//! text tokens surviving into the rendered scene, the scene→runtime tap
//! translation, and layout (hit-test) parity with the design canvas.

#![cfg(test)]

use super::PreviewSession;
use jian_core::gesture::pointer::PointerPhase;
use jian_core::widget_state::WidgetState;
use op_editor_ui::layout_scene::{LayoutScene, SceneNode};

/// The default (empty) active-theme map — every test enters preview with
/// no transient axis selection, so resolution falls back to the
/// document's default theme (first value per axis).
fn default_theme() -> std::collections::BTreeMap<String, String> {
    std::collections::BTreeMap::new()
}

/// Find a node by id on the scene's active page.
fn find<'a>(scene: &'a LayoutScene, id: &str) -> Option<&'a SceneNode> {
    scene.active_page().and_then(|p| p.find(id))
}

/// A minimal `.op` document with a single focusable text_input node.
fn text_input_doc() -> jian_ops_schema::PenDocument {
    let src = r##"{
        "version": "1.1",
        "formatVersion": "1.1",
        "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "children": [
            { "type": "text_input", "id": "field", "width": 200, "height": 40,
              "fill": [{ "type": "solid", "color": "#ffffff" }] }
        ]
    }"##;
    let loaded = jian_ops_schema::load_str(src).expect("parse test doc");
    loaded.value
}

#[test]
fn enter_input_exit_leaves_document_byte_identical() {
    // Spec exit-invariant: an enter → input → exit cycle must leave the
    // saved `PenDocument` byte-identical (the runtime is built from a
    // JSON snapshot, never the live doc).
    let doc = text_input_doc();
    let before = serde_json::to_string(&doc).expect("serialize before");

    {
        let mut session = PreviewSession::enter(&doc, (800.0, 600.0), &default_theme(), 0, false)
            .expect("enter preview");
        session.set_now_ms(0);
        session.focus_next();
        session.dispatch_text("hello");
        // Drop the session here (scope end) — mirrors Stop / Esc exit.
    }

    let after = serde_json::to_string(&doc).expect("serialize after");
    assert_eq!(
        before, after,
        "enter→input→exit must not mutate the document"
    );
}

#[test]
fn dispatched_text_reaches_runtime_state_graph() {
    // Injected text must land in the runtime's widget state, not the doc.
    let doc = text_input_doc();
    let mut session = PreviewSession::enter(&doc, (800.0, 600.0), &default_theme(), 0, false)
        .expect("enter preview");
    session.set_now_ms(0);
    session.focus_next();
    let consumed = session.dispatch_text("hi");
    assert!(consumed, "focused text_input should consume the text");

    let state = session
        .runtime()
        .widget_states
        .get("field")
        .expect("field widget state seeded");
    match state {
        WidgetState::TextInput(st) => assert_eq!(st.text(), "hi"),
        other => panic!("expected TextInput state, got {other:?}"),
    }
}

#[test]
fn overlay_reflects_typed_text() {
    // The live render path: after typing into a focused field, the
    // overlaid scene the painter walks must carry the typed text on the
    // field's `SceneWidget.value_str` (so `paint_text_field` draws it).
    let doc = text_input_doc();
    let mut session = PreviewSession::enter(&doc, (800.0, 600.0), &default_theme(), 0, false)
        .expect("enter preview");
    session.set_now_ms(0);
    session.focus_next();
    session.dispatch_text("hi");

    let scene = session.preview_scene_for_test();
    let field = find(&scene, "field").expect("field node in overlaid scene");
    let widget = field.widget.as_ref().expect("field carries a SceneWidget");
    assert_eq!(
        widget.value_str.as_deref(),
        Some("hi"),
        "typed text should overlay onto the field's scene widget value"
    );
}

#[test]
fn preview_shows_resolved_color_in_scene() {
    // The orange-renders-grey bug, end to end. A fill authored as a
    // `$color-*` ref must paint as the resolved literal in PREVIEW. The
    // design scene resolves it (preview reuses that scene), and the
    // overlay leaves geometry/colour untouched — so the swatch carries
    // the concrete #ff8800, never a dropped/grey fill.
    let src = r##"{
        "version": "1.1",
        "formatVersion": "1.1",
        "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "variables": { "color-brand": { "type": "color", "value": "#ff8800" } },
        "children": [
            {
                "type": "frame", "id": "screen", "width": 200, "height": 200,
                "children": [
                    {
                        "type": "rectangle", "id": "swatch",
                        "x": 0, "y": 0, "width": 100, "height": 100,
                        "fill": [{ "type": "solid", "color": "$color-brand" }]
                    }
                ]
            }
        ]
    }"##;
    let doc = jian_ops_schema::load_str(src)
        .expect("parse var-color doc")
        .value;
    let session = PreviewSession::enter(&doc, (800.0, 600.0), &default_theme(), 0, false)
        .expect("enter preview");

    let scene = session.preview_scene_for_test();
    let swatch = find(&scene, "swatch").expect("swatch node");
    let fill = swatch
        .fill
        .expect("swatch resolved fill (not grey/dropped)");
    assert!(
        (fill.r - 1.0).abs() < 0.02 && (fill.g - 0.533).abs() < 0.02 && fill.b < 0.02,
        "swatch should paint resolved #ff8800, got ({},{},{})",
        fill.r,
        fill.g,
        fill.b
    );
}

#[test]
fn preview_shows_resolved_text_token() {
    // Non-colour token (text content) must resolve in the preview scene
    // too: a text node whose content is the string variable
    // `$label-greeting` renders "Hello", not the raw `$token`.
    let src = r##"{
        "version": "1.1",
        "formatVersion": "1.1",
        "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "variables": { "label-greeting": { "type": "string", "value": "Hello" } },
        "children": [
            {
                "type": "frame", "id": "screen", "width": 200, "height": 200,
                "children": [
                    { "type": "text", "id": "greet", "content": "$label-greeting", "fontSize": 16 }
                ]
            }
        ]
    }"##;
    let doc = jian_ops_schema::load_str(src)
        .expect("parse tokened-text doc")
        .value;
    let session = PreviewSession::enter(&doc, (800.0, 600.0), &default_theme(), 0, false)
        .expect("enter preview");

    let scene = session.preview_scene_for_test();
    let greet = find(&scene, "greet").expect("greet text node");
    assert_eq!(
        greet.text.as_deref(),
        Some("Hello"),
        "text content token should resolve to \"Hello\" in the preview scene"
    );
}

/// A free (non-flex) frame with a single switch at a known absolute
/// position — exercises the full tap→runtime→overlay loop.
fn switch_doc() -> jian_ops_schema::PenDocument {
    let src = r##"{
        "version": "1.1",
        "formatVersion": "1.1",
        "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "children": [
            {
                "type": "frame", "id": "screen", "width": 200, "height": 200,
                "children": [
                    { "type": "switch", "id": "sw", "x": 20, "y": 20, "width": 44, "height": 24 }
                ]
            }
        ]
    }"##;
    let loaded = jian_ops_schema::load_str(src).expect("parse switch doc");
    loaded.value
}

#[test]
fn overlay_reflects_widget_toggle_on_tap() {
    // Full interaction loop: a tap at the switch's painted centre must
    // reach the runtime (via scene→runtime translation), toggle it, and
    // the overlaid scene must then report the switch as checked. Proves
    // taps land where widgets paint AND the live state surfaces in the
    // render.
    let doc = switch_doc();
    let mut session = PreviewSession::enter(&doc, (800.0, 600.0), &default_theme(), 0, false)
        .expect("enter preview");
    session.set_now_ms(0);

    // Switch starts unchecked.
    let scene0 = session.preview_scene_for_test();
    let sw0 = find(&scene0, "sw").expect("sw node");
    assert_ne!(
        sw0.widget.as_ref().and_then(|w| w.checked),
        Some(true),
        "switch should start unchecked"
    );

    // Tap the switch's painted centre (root at origin → scene == runtime).
    let (x, y, w, h) = session.node_rect("sw").expect("sw runtime rect");
    let handled = session.dispatch_tap(x + w / 2.0, y + h / 2.0);
    assert!(handled, "tap on the switch should emit a runtime event");

    let scene1 = session.preview_scene_for_test();
    let sw1 = find(&scene1, "sw").expect("sw node after tap");
    assert_eq!(
        sw1.widget.as_ref().and_then(|w| w.checked),
        Some(true),
        "tapping the switch should flip it on in the overlaid scene"
    );
}

#[test]
fn tap_translates_scene_space_to_runtime_for_offset_root() {
    // A root authored at (100, 50): the design scene offsets the whole
    // subtree by (100, 50), but the runtime lays it at its own origin.
    // A scene-space tap must subtract the root's authored origin so it
    // hits the runtime's root-relative geometry.
    let src = r##"{
        "version": "1.1",
        "formatVersion": "1.1",
        "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "children": [
            {
                "type": "frame", "id": "screen", "x": 100, "y": 50,
                "width": 200, "height": 200,
                "children": [
                    { "type": "switch", "id": "sw", "x": 20, "y": 20, "width": 44, "height": 24 }
                ]
            }
        ]
    }"##;
    let doc = jian_ops_schema::load_str(src)
        .expect("parse offset-root doc")
        .value;
    let session = PreviewSession::enter(&doc, (800.0, 600.0), &default_theme(), 0, false)
        .expect("enter preview");

    // A point inside the root in SCENE space maps to that point minus
    // the root's authored origin in RUNTIME space.
    let (rx, ry) = session.scene_to_runtime_for_test(150.0, 80.0);
    assert!(
        (rx - 50.0).abs() < 0.001 && (ry - 30.0).abs() < 0.001,
        "scene (150,80) under root@(100,50) should map to runtime (50,30), got ({rx},{ry})"
    );

    // A point outside every root falls through unchanged (nothing to hit).
    let (ox, oy) = session.scene_to_runtime_for_test(5.0, 5.0);
    assert!(
        (ox - 5.0).abs() < 0.001 && (oy - 5.0).abs() < 0.001,
        "a point outside all roots should pass through unchanged"
    );
}

/// A fixed-width vertical frame containing a horizontal row of four
/// labelled children plus a standalone text node — a minimal stand-in
/// for the food-app screen whose category labels scattered far to the
/// right in Preview. Root is 390 wide (a phone-screen width).
fn food_app_doc() -> jian_ops_schema::PenDocument {
    let src = r##"{
        "version": "1.1",
        "formatVersion": "1.1",
        "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "children": [
            {
                "type": "frame", "id": "screen",
                "width": 390, "height": 600,
                "layout": "vertical", "gap": 12, "padding": 16,
                "fill": [{ "type": "solid", "color": "#ffffff" }],
                "children": [
                    {
                        "type": "frame", "id": "cats",
                        "width": "fill_container", "height": "fit_content",
                        "layout": "horizontal", "gap": 8,
                        "children": [
                            { "type": "text", "id": "cat-pizza",  "content": "Pizza",  "fontSize": 14 },
                            { "type": "text", "id": "cat-burger", "content": "Burger", "fontSize": 14 },
                            { "type": "text", "id": "cat-sushi",  "content": "Sushi",  "fontSize": 14 },
                            { "type": "text", "id": "cat-seeall", "content": "See All","fontSize": 14 }
                        ]
                    },
                    { "type": "text", "id": "heading", "content": "Popular", "fontSize": 20 }
                ]
            }
        ]
    }"##;
    let loaded = jian_ops_schema::load_str(src).expect("parse food-app doc");
    loaded.value
}

#[test]
fn preview_layout_matches_design_canvas() {
    let _guard = crate::font_registry_test_support::lock();
    // Hit-test parity: the runtime layout (which preview taps hit-test
    // against) must resolve the SAME absolute rects as the static
    // design-canvas path, so a tap lands where the scene paints. For a
    // root authored at the origin, runtime rects == design rects.
    let doc = food_app_doc();

    let loaded = op_pen_loader::pen_document_to_payload(&doc);
    let mut design: std::collections::BTreeMap<String, (f32, f32, f32, f32)> =
        std::collections::BTreeMap::new();
    fn harvest(
        p: &op_pen_loader::NodePayload,
        out: &mut std::collections::BTreeMap<String, (f32, f32, f32, f32)>,
    ) {
        out.insert(p.schema_id.clone(), (p.x, p.y, p.w, p.h));
        for c in &p.children {
            harvest(c, out);
        }
    }
    for page in &loaded.payload.pages {
        for root in &page.children {
            harvest(root, &mut design);
        }
    }

    // A deliberately HUGE canvas region — the old code laid the doc out
    // against this and scattered. The fix ignores it (per-root).
    let session = PreviewSession::enter(&doc, (1600.0, 1200.0), &default_theme(), 0, false)
        .expect("enter preview");

    let (aw, _ah) = session.available();
    assert!(
        (aw - 390.0).abs() < 0.5,
        "preview available width should be the root's authored 390, got {aw}"
    );

    const TOL: f32 = 1.0;
    for id in [
        "screen",
        "cats",
        "cat-pizza",
        "cat-burger",
        "cat-sushi",
        "cat-seeall",
        "heading",
    ] {
        let (dx, dy, dw, dh) = *design
            .get(id)
            .unwrap_or_else(|| panic!("design rect for {id}"));
        let (px, py, pw, ph) = session
            .node_rect(id)
            .unwrap_or_else(|| panic!("preview rect for {id}"));
        assert!(
            (px - dx).abs() <= TOL
                && (py - dy).abs() <= TOL
                && (pw - dw).abs() <= TOL
                && (ph - dh).abs() <= TOL,
            "rect mismatch for {id}: design=({dx},{dy},{dw},{dh}) preview=({px},{py},{pw},{ph})"
        );
    }
}

/// A `fill_container`-rooted screen — laying it against the editor
/// canvas region (the old bug) let it expand to the full canvas.
fn fill_root_doc() -> jian_ops_schema::PenDocument {
    let src = r##"{
        "version": "1.1",
        "formatVersion": "1.1",
        "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "children": [
            {
                "type": "frame", "id": "screen",
                "width": "fill_container", "height": "fill_container",
                "layout": "vertical", "gap": 12, "padding": 16,
                "fill": [{ "type": "solid", "color": "#ffffff" }],
                "children": [
                    {
                        "type": "frame", "id": "cats",
                        "width": "fill_container", "height": "fit_content",
                        "layout": "horizontal", "justifyContent": "space_between",
                        "children": [
                            { "type": "text", "id": "cat-pizza",  "content": "Pizza",   "fontSize": 14 },
                            { "type": "text", "id": "cat-seeall", "content": "See All", "fontSize": 14 }
                        ]
                    }
                ]
            }
        ]
    }"##;
    let loaded = jian_ops_schema::load_str(src).expect("parse fill-root doc");
    loaded.value
}

#[test]
fn preview_children_stay_within_root_width() {
    let _guard = crate::font_registry_test_support::lock();
    // With a `fill_container` root, every node's right edge must fall
    // within the root frame's resolved width (the design canvas's own
    // resolved `screen` width), NOT the editor canvas region.
    let doc = fill_root_doc();

    let loaded = op_pen_loader::pen_document_to_payload(&doc);
    let root_w = loaded
        .payload
        .pages
        .iter()
        .flat_map(|p| p.children.iter())
        .find(|n| n.schema_id == "screen")
        .map(|n| n.w)
        .expect("design root width");

    let session = PreviewSession::enter(&doc, (1600.0, 1200.0), &default_theme(), 0, false)
        .expect("enter preview");

    const BLEED: f32 = 2.0;
    for id in ["screen", "cats", "cat-pizza", "cat-seeall"] {
        let (x, _y, w, _h) = session
            .node_rect(id)
            .unwrap_or_else(|| panic!("preview rect for {id}"));
        assert!(
            x + w <= root_w + BLEED,
            "{id} right edge {} exceeds design root width {root_w} (children scattered)",
            x + w
        );
    }
}

/// Two root frames of DIFFERENT authored widths, each with a
/// `fill_container` child row — proves per-root layout (each root solved
/// against its OWN authored width).
fn two_root_doc() -> jian_ops_schema::PenDocument {
    let src = r##"{
        "version": "1.1",
        "formatVersion": "1.1",
        "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "children": [
            {
                "type": "frame", "id": "root-a",
                "width": 390, "height": 300,
                "layout": "vertical", "padding": 16,
                "fill": [{ "type": "solid", "color": "#ffffff" }],
                "children": [
                    {
                        "type": "frame", "id": "row-a",
                        "width": "fill_container", "height": "fit_content",
                        "layout": "horizontal", "gap": 8,
                        "children": [
                            { "type": "text", "id": "a-x", "content": "X", "fontSize": 14 }
                        ]
                    }
                ]
            },
            {
                "type": "frame", "id": "root-b",
                "width": "fill_container", "height": 300,
                "layout": "vertical", "padding": 16,
                "fill": [{ "type": "solid", "color": "#ffffff" }],
                "children": [
                    {
                        "type": "frame", "id": "row-b",
                        "width": "fill_container", "height": "fit_content",
                        "layout": "horizontal", "gap": 8,
                        "children": [
                            { "type": "text", "id": "b-x", "content": "X", "fontSize": 14 }
                        ]
                    }
                ]
            }
        ]
    }"##;
    let loaded = jian_ops_schema::load_str(src).expect("parse two-root doc");
    loaded.value
}

#[test]
fn preview_lays_each_root_against_its_own_size() {
    let _guard = crate::font_registry_test_support::lock();
    // Per-root layout proof: root-a is numeric 390, root-b is
    // `fill_container` (flex-default 1440). Each must resolve against its
    // OWN size, matching the design canvas.
    let doc = two_root_doc();

    let loaded = op_pen_loader::pen_document_to_payload(&doc);
    let mut design: std::collections::BTreeMap<String, f32> = std::collections::BTreeMap::new();
    fn harvest_w(
        p: &op_pen_loader::NodePayload,
        out: &mut std::collections::BTreeMap<String, f32>,
    ) {
        out.insert(p.schema_id.clone(), p.w);
        for c in &p.children {
            harvest_w(c, out);
        }
    }
    for page in &loaded.payload.pages {
        for root in &page.children {
            harvest_w(root, &mut design);
        }
    }
    let design_row_a = *design.get("row-a").expect("design width for row-a");
    let design_row_b = *design.get("row-b").expect("design width for row-b");
    assert!(
        (design_row_b - design_row_a).abs() > 150.0,
        "design rows should differ widely (a={design_row_a}, b={design_row_b}) — fixture sanity"
    );

    let session = PreviewSession::enter(&doc, (1600.0, 1200.0), &default_theme(), 0, false)
        .expect("enter preview");

    const TOL: f32 = 1.0;
    let (_pax, _pay, pw_a, _pah) = session.node_rect("row-a").expect("preview rect for row-a");
    let (_pbx, _pby, pw_b, _pbh) = session.node_rect("row-b").expect("preview rect for row-b");
    assert!(
        (pw_a - design_row_a).abs() <= TOL,
        "row-a width: design={design_row_a} preview={pw_a}"
    );
    assert!(
        (pw_b - design_row_b).abs() <= TOL,
        "row-b width: design={design_row_b} preview={pw_b}"
    );
}

#[test]
fn legacy_role_promotion_is_recorded_as_warning() {
    // A legacy role-frame should be promoted (promote=true) and the
    // promotion surfaced as a warning for the editor diagnostics.
    let src = r##"{
        "version": "1.1",
        "formatVersion": "1.1",
        "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "children": [
            { "type": "frame", "id": "legacy", "width": 200, "height": 40,
              "meta": { "role": "input" } }
        ]
    }"##;
    let doc = jian_ops_schema::load_str(src)
        .expect("parse legacy doc")
        .value;

    let session = PreviewSession::enter(&doc, (800.0, 600.0), &default_theme(), 0, false)
        .expect("enter preview on legacy doc");
    for w in session.warnings() {
        assert!(!w.is_empty());
    }
}

#[test]
fn focus_seeds_widget_state_for_caret() {
    // CONCERN fix: Tab-focusing a text input (without typing) must seed
    // its runtime state so the caret can paint immediately —
    // `Runtime::focus_next` alone only moves the focus pointer.
    let doc = text_input_doc();
    let mut session = PreviewSession::enter(&doc, (800.0, 600.0), &default_theme(), 0, false)
        .expect("enter preview");
    session.set_now_ms(0);
    session.focus_next();
    assert!(
        session.runtime().widget_states.get("field").is_some(),
        "focusing a text input should seed its widget state for caret paint"
    );
}

#[test]
fn preview_promotes_role_input_frame_to_interactive_field() {
    // The screenshot bug end-to-end: a generated `role=input` mockup
    // frame (box + leading mail icon + muted placeholder) must become a
    // live `text_input` in PREVIEW — rendered as a widget (icon kept) AND
    // clickable + typeable. Before the rework the preview scene showed
    // the un-promoted frame, so typing was invisible.
    let doc = jian_ops_schema::load_str(
        r##"{"version":"1.1","formatVersion":"1.1","id":"x",
        "app":{"name":"x","version":"1","id":"x"},
        "children":[{"type":"frame","id":"screen","width":300,"height":300,"children":[
          {"type":"frame","id":"email","role":"input","x":20,"y":20,"width":260,"height":48,
           "cornerRadius":12,"fill":[{"type":"solid","color":"#f3f4f6"}],"children":[
             {"type":"icon_font","id":"i","iconFontName":"mail","width":20,"height":20},
             {"type":"text","id":"t","content":"you@example.com",
              "fill":[{"type":"solid","color":"#9ca3af"}]}
           ]}]}]}"##,
    )
    .expect("parse role=input doc")
    .value;
    let mut session = PreviewSession::enter(&doc, (800.0, 600.0), &default_theme(), 0, false)
        .expect("enter preview");
    session.set_now_ms(0);

    // The promoted field renders as a text_input widget in preview's scene.
    let scene = session.preview_scene_for_test();
    let field = find(&scene, "email").expect("promoted field node");
    let w = field
        .widget
        .as_ref()
        .expect("role=input frame promoted to a widget in the preview scene");
    assert_eq!(w.kind, "text_input");
    assert_eq!(w.leading_icon.as_deref(), Some("mail"));
    assert_eq!(w.placeholder.as_deref(), Some("you@example.com"));

    // Tap its centre → focus → type → overlaid scene shows the value.
    let (x, y, wd, h) = session
        .node_rect("email")
        .expect("runtime rect for promoted field");
    assert!(
        session.dispatch_tap(x + wd / 2.0, y + h / 2.0),
        "tap should hit the promoted input"
    );
    session.dispatch_text("a@b");
    let after = session.preview_scene_for_test();
    assert_eq!(
        find(&after, "email")
            .unwrap()
            .widget
            .as_ref()
            .unwrap()
            .value_str
            .as_deref(),
        Some("a@b"),
        "typed text must surface in the overlaid preview scene"
    );
}

/// A frame with an onHoverEnter action writing `$app.hovered`.
fn hover_doc() -> jian_ops_schema::PenDocument {
    let src = r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "state": { "hovered": { "type": "bool", "default": false } },
        "children": [
            { "type": "frame", "id": "card", "width": 200, "height": 100,
              "fill": [{ "type": "solid", "color": "#ffffff" }],
              "events": { "onHoverEnter": [ { "set": { "$app.hovered": "true" } } ] } }
        ]
    }"##;
    jian_ops_schema::load_str(src)
        .expect("parse hover doc")
        .value
}

#[test]
fn hover_move_fires_on_hover_enter() {
    let doc = hover_doc();
    let mut session =
        PreviewSession::enter(&doc, (800.0, 600.0), &default_theme(), 0, false).expect("enter");
    session.set_now_ms(0);
    session.dispatch_pointer_phase(20.0, 20.0, PointerPhase::Hover);
    let v = session
        .runtime()
        .state
        .app_get("hovered")
        .expect("hovered seeded from doc state");
    assert_eq!(
        v.0,
        serde_json::json!(true),
        "onHoverEnter must fire on Hover move"
    );
}

/// A slider so a Down→Move→Up drag can be asserted.
fn slider_doc() -> jian_ops_schema::PenDocument {
    let src = r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "children": [
            { "type": "slider", "id": "vol", "width": 200, "height": 24,
              "min": 0, "max": 100, "value": 50 }
        ]
    }"##;
    jian_ops_schema::load_str(src)
        .expect("parse slider doc")
        .value
}

#[test]
fn slider_drag_moves_value() {
    let doc = slider_doc();
    let mut session =
        PreviewSession::enter(&doc, (800.0, 600.0), &default_theme(), 0, false).expect("enter");
    session.set_now_ms(0);
    let (x, y, w, h) = session.node_rect("vol").expect("slider rect");
    let cy = y + h / 2.0;
    session.dispatch_pointer_phase(x + w * 0.5, cy, PointerPhase::Down);
    session.dispatch_pointer_phase(x + w * 0.9, cy, PointerPhase::Move);
    session.dispatch_pointer_phase(x + w * 0.9, cy, PointerPhase::Up);
    match session.runtime().widget_states.get("vol") {
        Some(WidgetState::Slider { value, .. }) => {
            assert!(
                *value > 60.0,
                "drag to 90% must move the value past 60, got {value}"
            );
        }
        other => panic!("expected Slider state, got {other:?}"),
    }
}

#[test]
fn wheel_routes_only_to_on_scroll_handler() {
    // With an onScroll handler → consumed; without → not consumed
    // (the host then falls back to canvas pan/zoom).
    let src = r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "state": { "scrolled": { "type": "bool", "default": false } },
        "children": [
            { "type": "frame", "id": "list", "width": 200, "height": 100,
              "events": { "onScroll": [ { "set": { "$app.scrolled": "true" } } ] } }
        ]
    }"##;
    let doc = jian_ops_schema::load_str(src).expect("parse").value;
    let mut session =
        PreviewSession::enter(&doc, (800.0, 600.0), &default_theme(), 0, false).expect("enter");
    assert!(
        session.dispatch_wheel(20.0, 20.0, 0.0, -12.0),
        "onScroll node must consume"
    );

    let plain = text_input_doc();
    let mut plain_session =
        PreviewSession::enter(&plain, (800.0, 600.0), &default_theme(), 0, false).expect("enter");
    assert!(
        !plain_session.dispatch_wheel(20.0, 20.0, 0.0, -12.0),
        "no handler → not consumed → host may pan/zoom"
    );
}
