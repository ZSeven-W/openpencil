//! R6 full binding overlay, invalidation, scroll namespace, and sticky nodes.

#![cfg(test)]

use super::input_event::{PreviewInput, PreviewInputEnvelope, ScrollPhase};
use super::{test_measure, InvalidationKind, PreviewSession};
use op_editor_ui::layout_scene::{LayoutScene, SceneNode};

pub(super) fn theme() -> std::collections::BTreeMap<String, String> {
    std::collections::BTreeMap::new()
}

pub(super) fn find<'a>(scene: &'a LayoutScene, id: &str) -> &'a SceneNode {
    scene
        .active_page()
        .and_then(|page| page.find(id))
        .unwrap_or_else(|| panic!("missing scene node {id}"))
}

fn binding_doc() -> jian_ops_schema::PenDocument {
    let source = r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "bindings",
        "app": { "name": "bindings", "version": "1", "id": "bindings" },
        "state": {
            "visible": { "type": "bool", "default": true },
            "opacity": { "type": "number", "default": 1 },
            "color": { "type": "string", "default": "#0000ff" },
            "stroke": { "type": "string", "default": "#000000" },
            "content": { "type": "string", "default": "Authored" },
            "textColor": { "type": "string", "default": "#000000" },
            "checked": { "type": "bool", "default": false },
            "selected": { "type": "string", "default": "a" },
            "value": { "type": "number", "default": 0 },
            "x": { "type": "number", "default": 10 },
            "y": { "type": "number", "default": 20 },
            "width": { "type": "number", "default": 100 },
            "height": { "type": "number", "default": 40 },
            "rotation": { "type": "number", "default": 0 },
            "scaleX": { "type": "number", "default": 1 },
            "scaleY": { "type": "number", "default": 1 },
            "variant": { "type": "string", "default": "variant-a" },
            "active": { "type": "string", "default": "tab-a" }
        },
        "children": [
            { "type": "rectangle", "id": "card", "x": 10, "y": 20,
              "width": 100, "height": 40,
              "fill": [{ "type": "solid", "color": "#0000ff" }],
              "events": {
                  "onTap": [{ "set": { "$app.width": "240" } }]
              },
              "bindings": {
                  "visible": "$app.visible",
                  "opacity": "$app.opacity",
                  "fill": "$app.color",
                  "stroke": "$app.stroke",
                  "x": "$app.x",
                  "y": "$app.y",
                  "width": "$app.width",
                  "height": "$app.height",
                  "rotation": "$app.rotation",
                  "scaleX": "$app.scaleX",
                  "scaleY": "$app.scaleY"
              } },
            { "type": "text", "id": "label", "x": 0, "y": 80,
              "width": 200, "height": 24, "content": "Authored",
              "fill": [{ "type": "solid", "color": "#000000" }],
              "bindings": {
                  "content": "$app.content",
                  "textColor": "$app.textColor"
              } },
            { "type": "switch", "id": "switch", "x": 0, "y": 120,
              "width": 44, "height": 24,
              "bindings": { "checked": "$app.checked" } },
            { "type": "select", "id": "select", "x": 60, "y": 120,
              "width": 100, "height": 32, "value": "a",
              "options": [
                  { "value": "a", "label": "A" },
                  { "value": "b", "label": "B" }
              ],
              "bindings": { "selectedValue": "$app.selected" } },
            { "type": "slider", "id": "slider", "x": 0, "y": 165,
              "width": 160, "height": 24, "min": 0, "max": 100, "value": 0,
              "bindings": { "value": "$app.value" } },
            { "type": "frame", "id": "variant-host", "x": 220, "y": 0,
              "width": 100, "height": 80,
              "bindings": { "variant": "$app.variant" },
              "children": [
                  { "type": "rectangle", "id": "variant-a",
                    "width": 100, "height": 80 },
                  { "type": "rectangle", "id": "variant-b",
                    "width": 100, "height": 80 }
              ] },
            { "type": "tabs", "id": "tabs", "x": 220, "y": 100,
              "width": 160, "height": 100, "value": "tab-a",
              "tabs": [
                  { "value": "tab-a", "label": "A" },
                  { "value": "tab-b", "label": "B" }
              ],
              "bindings": { "activeState": "$app.active" },
              "children": [
                  { "type": "rectangle", "id": "tab-a",
                    "width": 160, "height": 100 },
                  { "type": "rectangle", "id": "tab-b",
                    "width": 160, "height": 100 }
              ] }
        ]
    }"##;
    jian_ops_schema::load_str(source)
        .expect("parse binding document")
        .value
}

#[test]
fn authored_action_state_change_runs_the_same_relayout_pipeline() {
    let document = binding_doc();
    let mut session = enter(&document);
    let scene = session.preview_scene_for_test();
    let card = find(&scene, "card");
    let tap = jian_core::geometry::point(
        card.bounds.origin.x + card.bounds.size.x / 2.0,
        card.bounds.origin.y + card.bounds.size.y / 2.0,
    );
    let mut down = jian_core::gesture::PointerEvent::simple_at(
        1,
        jian_core::gesture::pointer::PointerPhase::Down,
        tap,
        0,
    );
    down.kind = jian_core::gesture::pointer::PointerKind::Touch;
    let _ = session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Pointer(down)));
    let mut up = jian_core::gesture::PointerEvent::simple_at(
        1,
        jian_core::gesture::pointer::PointerPhase::Up,
        tap,
        10,
    );
    up.kind = jian_core::gesture::pointer::PointerKind::Touch;
    let outcome = session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Pointer(up)));
    let state_width = session
        .runtime
        .state
        .app_get("width")
        .and_then(|value| value.as_i64());
    assert!(
        outcome.needs_redraw,
        "tap must run the set action; state width is {state_width:?}"
    );
    assert_eq!(
        session.runtime_rect("card").map(|rect| rect.size.x),
        Some(240.0),
        "authored set action materializes the Relayout overlay"
    );
}

#[test]
fn every_typed_target_changes_its_runtime_projection() {
    let document = binding_doc();
    let session = enter(&document);
    for (key, value) in [
        ("stroke", serde_json::json!("#ff00ff")),
        ("content", serde_json::json!("Bound")),
        ("textColor", serde_json::json!("#00ff00")),
        ("checked", serde_json::json!(true)),
        ("selected", serde_json::json!("b")),
        ("value", serde_json::json!(42)),
        ("y", serde_json::json!(30)),
        ("height", serde_json::json!(50)),
        ("rotation", serde_json::json!(90)),
        ("scaleX", serde_json::json!(2)),
        ("scaleY", serde_json::json!(0.5)),
        ("variant", serde_json::json!("variant-b")),
        ("active", serde_json::json!("tab-b")),
    ] {
        session.runtime.state.app_set(key, value);
    }

    let scene = session.preview_scene_for_test();
    let card = find(&scene, "card");
    assert_eq!(card.bounds.origin, op_editor_ui::Point2D::new(-40.0, 42.5));
    assert_eq!(card.bounds.size, op_editor_ui::Point2D::new(200.0, 25.0));
    assert!((card.rotation - std::f32::consts::FRAC_PI_2).abs() < 0.001);
    let stroke = card.stroke.expect("bound stroke");
    assert_eq!(
        (stroke.color.r, stroke.color.g, stroke.color.b),
        (1.0, 0.0, 1.0)
    );

    let label = find(&scene, "label");
    assert_eq!(label.text.as_deref(), Some("Bound"));
    let text_color = label.fill.expect("bound text color");
    assert_eq!((text_color.r, text_color.g, text_color.b), (0.0, 1.0, 0.0));
    assert_eq!(
        find(&scene, "switch")
            .widget
            .as_ref()
            .and_then(|widget| widget.checked),
        Some(true)
    );
    assert_eq!(
        find(&scene, "select")
            .widget
            .as_ref()
            .and_then(|widget| widget.value_str.as_deref()),
        Some("b")
    );
    assert_eq!(
        find(&scene, "slider")
            .widget
            .as_ref()
            .and_then(|widget| widget.value_num),
        Some(42.0)
    );
    assert!(find(&scene, "variant-a").hidden);
    assert!(!find(&scene, "variant-b").hidden);
    let tabs = find(&scene, "tabs");
    assert_eq!(
        tabs.widget
            .as_ref()
            .and_then(|widget| widget.value_str.as_deref()),
        Some("tab-b")
    );
    assert_eq!(
        tabs.visible_children()
            .first()
            .map(|child| child.id.as_str()),
        Some("tab-b")
    );
}

pub(super) fn enter(document: &jian_ops_schema::PenDocument) -> PreviewSession {
    PreviewSession::enter(
        document,
        (800.0, 600.0),
        &theme(),
        0,
        false,
        false,
        test_measure(),
    )
    .expect("enter preview")
}

#[test]
fn non_content_bindings_apply_to_the_preview_scene() {
    let document = binding_doc();
    let session = enter(&document);
    session
        .runtime
        .state
        .app_set("visible", serde_json::json!(false));
    session
        .runtime
        .state
        .app_set("opacity", serde_json::json!(0.5));
    session
        .runtime
        .state
        .app_set("color", serde_json::json!("#ff0000"));
    session.runtime.state.app_set("x", serde_json::json!(24));
    session
        .runtime
        .state
        .app_set("width", serde_json::json!(240));

    let scene = session.preview_scene_for_test();
    let card = find(&scene, "card");
    assert!(card.hidden, "visible=false removes the node from hit/paint");
    assert_eq!(card.opacity, 0.5);
    let fill = card.fill.expect("bound fill");
    assert_eq!((fill.r, fill.g, fill.b), (1.0, 0.0, 0.0));
    assert_eq!(card.bounds.origin.x, 24.0);
    assert_eq!(card.bounds.size.x, 240.0);
}

#[test]
fn binding_target_classification_covers_the_complete_contract() {
    use jian_core::binding::{classify_binding, BindingTarget};

    for (property, target, invalidation) in [
        (
            "content",
            BindingTarget::Content,
            InvalidationKind::Relayout,
        ),
        ("value", BindingTarget::Value, InvalidationKind::PaintOnly),
        (
            "checked",
            BindingTarget::Checked,
            InvalidationKind::PaintOnly,
        ),
        (
            "selectedValue",
            BindingTarget::SelectedValue,
            InvalidationKind::PaintOnly,
        ),
        ("visible", BindingTarget::Visible, InvalidationKind::HitTest),
        (
            "opacity",
            BindingTarget::Opacity,
            InvalidationKind::PaintOnly,
        ),
        ("fill", BindingTarget::Fill, InvalidationKind::PaintOnly),
        ("stroke", BindingTarget::Stroke, InvalidationKind::PaintOnly),
        (
            "textColor",
            BindingTarget::TextColor,
            InvalidationKind::PaintOnly,
        ),
        ("x", BindingTarget::X, InvalidationKind::HitTest),
        ("y", BindingTarget::Y, InvalidationKind::HitTest),
        ("width", BindingTarget::Width, InvalidationKind::Relayout),
        ("height", BindingTarget::Height, InvalidationKind::Relayout),
        (
            "rotation",
            BindingTarget::Rotation,
            InvalidationKind::HitTest,
        ),
        ("scaleX", BindingTarget::ScaleX, InvalidationKind::HitTest),
        ("scaleY", BindingTarget::ScaleY, InvalidationKind::HitTest),
        (
            "variant",
            BindingTarget::Variant,
            InvalidationKind::Relayout,
        ),
        (
            "activeState",
            BindingTarget::ActiveState,
            InvalidationKind::Relayout,
        ),
    ] {
        assert_eq!(BindingTarget::parse(property), Some(target), "{property}");
        assert_eq!(classify_binding(property), invalidation, "{property}");
    }
    assert_eq!(
        InvalidationKind::None.merge(InvalidationKind::PaintOnly),
        InvalidationKind::PaintOnly
    );
    assert_eq!(
        InvalidationKind::PaintOnly.merge(InvalidationKind::HitTest),
        InvalidationKind::HitTest
    );
    assert_eq!(
        InvalidationKind::HitTest.merge(InvalidationKind::Relayout),
        InvalidationKind::Relayout
    );
    assert_eq!(
        InvalidationKind::Relayout.merge(InvalidationKind::Navigation),
        InvalidationKind::Navigation
    );
    assert_eq!(
        serde_json::to_string(&InvalidationKind::Navigation).unwrap(),
        "\"navigation\""
    );
}

#[test]
fn set_state_reports_invalidation_and_refreshes_hit_geometry() {
    let document = binding_doc();
    let before = serde_json::to_string(&document).expect("serialize source");
    let mut session = enter(&document);

    assert_eq!(
        session.set_state("color", serde_json::json!("#ff0000")),
        InvalidationKind::PaintOnly
    );
    assert_eq!(
        session.set_state("visible", serde_json::json!(false)),
        InvalidationKind::HitTest
    );
    assert_eq!(
        session.set_state("visible", serde_json::json!(true)),
        InvalidationKind::HitTest
    );
    assert_eq!(
        session.set_state("x", serde_json::json!(24)),
        InvalidationKind::HitTest
    );
    let before_rotation = session.preview_scene_for_test();
    let card_before_rotation = find(&before_rotation, "card");
    let rotated_point = (
        card_before_rotation.bounds.origin.x + card_before_rotation.bounds.size.x / 2.0,
        card_before_rotation.bounds.origin.y - 10.0,
    );
    assert_eq!(
        session.set_state("rotation", serde_json::json!(90)),
        InvalidationKind::HitTest
    );
    assert_eq!(
        session
            .deepest_mapped_hit(rotated_point.0, rotated_point.1)
            .map(|(_, _, id)| id)
            .as_deref(),
        Some("card"),
        "rotation invalidation refreshes transformed hit geometry"
    );
    assert_eq!(
        session.set_state("rotation", serde_json::json!(0)),
        InvalidationKind::HitTest
    );
    assert_eq!(
        session.set_state("width", serde_json::json!(240)),
        InvalidationKind::Relayout
    );

    assert_eq!(
        session
            .deepest_mapped_hit(200.0, 30.0)
            .map(|(_, _, id)| id)
            .as_deref(),
        Some("card"),
        "expanded bound width is immediately hittable"
    );
    assert_eq!(
        session.runtime_rect("card").map(|rect| rect.size.x),
        Some(240.0),
        "Relayout installs the materialized runtime overlay document"
    );
    assert!(
        session.deepest_mapped_hit(12.0, 30.0).is_none(),
        "old x position is no longer hittable"
    );
    assert_eq!(
        serde_json::to_string(&document).expect("serialize source after"),
        before,
        "runtime overlay never mutates the authored document"
    );
}

fn scroll_doc() -> jian_ops_schema::PenDocument {
    let source = r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "scroll",
        "app": { "name": "scroll", "version": "1", "id": "scroll" },
        "state": { "seen": { "type": "bool", "default": false } },
        "children": [
            { "type": "frame", "id": "scroller", "x": 0, "y": 0,
              "width": 160, "height": 100, "clipContent": true,
              "stickyChildren": ["sticky"],
              "events": { "onScroll": [
                  { "set": { "$app.seen": "true" } }
              ] },
              "children": [
                  { "type": "rectangle", "id": "sticky", "pin": true,
                    "x": 0, "y": 0, "width": 160, "height": 20,
                    "fill": [{ "type": "solid", "color": "#111111" }] },
                  { "type": "rectangle", "id": "body",
                    "x": 0, "y": 40, "width": 160, "height": 240,
                    "opacity": 1,
                    "fill": [{ "type": "solid", "color": "#eeeeee" }],
                    "bindings": {
                        "opacity": "$scroll.direction == \"none\" ? 0 : $scroll.progress"
                    } }
              ] }
        ]
    }"##;
    jian_ops_schema::load_str(source)
        .expect("parse scroll document")
        .value
}

#[test]
fn scroll_phase_updates_namespace_and_moves_only_unpinned_content() {
    let document = scroll_doc();
    let mut session = enter(&document);
    let before = session.preview_scene_for_test();
    let sticky_y = find(&before, "sticky").bounds.origin.y;
    let body_y = find(&before, "body").bounds.origin.y;

    let wheel = jian_core::gesture::pointer::WheelEvent::simple(
        jian_core::geometry::point(80.0, 50.0),
        jian_core::geometry::point(0.0, -60.0),
    );
    let outcome = session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Wheel {
        event: wheel,
        phase: ScrollPhase::Changed,
    }));
    assert!(outcome.needs_redraw);
    assert_eq!(
        session
            .runtime
            .state
            .app_get("seen")
            .and_then(|value| value.as_bool()),
        Some(true),
        "the same scroll trace produces the authored state diff"
    );

    let after = session.preview_scene_for_test();
    let sticky = find(&after, "sticky");
    let body = find(&after, "body");
    assert_eq!(sticky.bounds.origin.y, sticky_y, "pinned child stays fixed");
    assert!(
        body.bounds.origin.y < body_y,
        "ordinary content moves with the scroll offset"
    );
    assert!(
        body.opacity > 0.0 && body.opacity < 1.0,
        "$scroll.progress drives the PaintOnly binding"
    );
    assert!(
        (body.opacity - (1.0 / 3.0)).abs() < 0.01,
        "offset=60 and maxOffset=180 produce progress=1/3, got {}",
        body.opacity
    );
    assert_eq!(
        session
            .deepest_mapped_hit(80.0, sticky.bounds.origin.y + 10.0)
            .map(|(_, _, id)| id)
            .as_deref(),
        Some("sticky"),
        "sticky visual and hit-test geometry stay aligned"
    );

    let ended = jian_core::gesture::pointer::WheelEvent::simple(
        jian_core::geometry::point(80.0, 50.0),
        jian_core::geometry::point(0.0, 0.0),
    );
    let ended_outcome = session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Wheel {
        event: ended,
        phase: ScrollPhase::Ended,
    }));
    assert!(
        ended_outcome.needs_redraw,
        "Ended changes direction to none"
    );
    assert_eq!(
        find(&session.preview_scene_for_test(), "body").opacity,
        0.0,
        "ScrollPhase::Ended is consumed by the $scroll producer"
    );
}

#[test]
fn scroll_binding_rejects_relayout_and_warns_once_without_ancestor() {
    let invalid = r##"{
        "version":"1.1","formatVersion":"1.1","id":"invalid",
        "app":{"name":"invalid","version":"1","id":"invalid"},
        "children":[
            {"type":"rectangle","id":"bad","width":100,"height":20,
             "bindings":{"width":"$scroll.progress * 100"}},
            {"type":"rectangle","id":"orphan","width":100,"height":20,
             "opacity":1,"bindings":{"opacity":"$scroll.progress"}}
        ]
    }"##;
    let document = jian_ops_schema::load_str(invalid).expect("parse").value;
    let session = enter(&document);
    assert_eq!(
        session.binding_sites_len_for_test(),
        1,
        "the Relayout scroll binding is rejected, PaintOnly orphan remains"
    );
    let restricted: Vec<_> = session
        .warnings()
        .iter()
        .filter(|warning| warning.contains("ScrollBindingRequiresPaintOnly"))
        .collect();
    assert_eq!(restricted.len(), 1, "one structured restriction diagnostic");
    assert!(
        !session
            .warnings()
            .iter()
            .any(|warning| warning.contains("MissingScrollAncestor")),
        "a top-level binding is never orphaned: it binds to the page scroll"
    );
    assert_eq!(
        find(&session.preview_scene_for_test(), "orphan").opacity,
        0.0,
        "an unscrolled page evaluates $scroll.progress to 0"
    );
}

#[test]
fn routed_navigation_is_a_real_navigation_invalidation_producer() {
    let source = r##"{
        "version":"1.1","formatVersion":"1.1","id":"routes",
        "app":{"name":"routes","version":"1","id":"routes"},
        "children":[
            {"type":"frame","id":"home","name":"Home","screen":"/",
             "width":200,"height":200},
            {"type":"frame","id":"detail","name":"Detail","screen":"/detail",
             "width":200,"height":200}
        ]
    }"##;
    let document = jian_ops_schema::load_str(source).expect("parse").value;
    let mut session = enter(&document);
    assert_eq!(
        session.navigate_to_screen("/detail"),
        InvalidationKind::Navigation
    );
    assert_eq!(
        session.reconcile(0).invalidation(),
        InvalidationKind::Navigation,
        "the committed page mount remains a Navigation producer"
    );
}

#[test]
fn ui_mutation_work_is_consumed_as_hit_test_invalidation() {
    use jian_core::action::services::{UiMutationOutcome, UiMutationRequest, UiMutationWork};

    let document = binding_doc();
    let session = enter(&document);
    assert_eq!(
        session
            .runtime
            .ui_mutation_sink
            .apply(&UiMutationRequest::SetVisibility {
                node_id: "card".to_owned(),
                visible: false,
            }),
        UiMutationOutcome::Applied(UiMutationWork::REDRAW_AND_HIT_TEST)
    );
    assert!(
        find(&session.preview_scene_for_test(), "card").hidden,
        "R5 visibility mutations are folded into the unified R6 overlay"
    );
    assert_eq!(
        session.take_ui_action_invalidation(),
        InvalidationKind::HitTest
    );
    assert_eq!(
        session.take_ui_action_invalidation(),
        InvalidationKind::None
    );
}

#[test]
fn scroll_to_request_is_consumed_by_the_unified_overlay() {
    use jian_core::action::services::{
        ScrollAlignment, UiMutationOutcome, UiMutationRequest, UiMutationWork,
    };

    let document = scroll_doc();
    let session = enter(&document);
    assert_eq!(
        session
            .runtime
            .ui_mutation_sink
            .apply(&UiMutationRequest::ScrollTo {
                target_id: "body".to_owned(),
                alignment: ScrollAlignment::Center,
            }),
        UiMutationOutcome::Applied(UiMutationWork::REDRAW_AND_HIT_TEST)
    );
    let scene = session.preview_scene_for_test();
    let scroller = find(&scene, "scroller");
    let body = find(&scene, "body");
    assert!(
        ((body.bounds.origin.y + body.bounds.size.y / 2.0)
            - (scroller.bounds.origin.y + scroller.bounds.size.y / 2.0))
            .abs()
            < 0.01,
        "scroll_to center aligns target and viewport centers"
    );
}
