//! B2 read-only `$pointer` binding namespace coverage.

#![cfg(test)]

use super::input_event::{PreviewInput, PreviewInputEnvelope};
use super::{test_measure, PreviewSession};
use jian_core::geometry::point;
use jian_core::gesture::pointer::{PointerKind, PointerPhase};
use jian_core::gesture::PointerEvent;
use op_editor_ui::layout_scene::{LayoutScene, SceneNode};

fn enter(source: &str) -> PreviewSession {
    let document = jian_ops_schema::load_str(source)
        .expect("parse pointer binding fixture")
        .value;
    PreviewSession::enter(
        &document,
        (800.0, 600.0),
        &std::collections::BTreeMap::new(),
        0,
        false,
        false,
        test_measure(),
    )
    .expect("enter pointer binding fixture")
}

fn pointer(
    id: u32,
    kind: PointerKind,
    phase: PointerPhase,
    x: f32,
    y: f32,
) -> PreviewInputEnvelope {
    let mut event = PointerEvent::simple_at(id, phase, point(x, y), 0);
    event.kind = kind;
    PreviewInputEnvelope::new(PreviewInput::Pointer(event))
}

fn find<'a>(scene: &'a LayoutScene, id: &str) -> &'a SceneNode {
    scene
        .active_page()
        .and_then(|page| page.find(id))
        .unwrap_or_else(|| panic!("missing scene node {id}"))
}

fn opacity(session: &PreviewSession, id: &str) -> f32 {
    find(&session.preview_scene_for_test(), id).opacity
}

fn pointer_fixture() -> &'static str {
    r##"{
        "version":"1.1","formatVersion":"1.1","id":"pointer-bindings",
        "app":{"name":"pointer-bindings","version":"1","id":"pointer-bindings"},
        "children":[
            {"type":"frame","id":"screen","x":100,"y":50,"width":200,"height":100,
             "layout":"none",
             "children":[
                {"type":"rectangle","id":"x","width":10,"height":10,"opacity":0,
                 "bindings":{"opacity":"$pointer.x == 150 ? 0.5 : 0"}},
                {"type":"rectangle","id":"y","width":10,"height":10,"opacity":0,
                 "bindings":{"opacity":"$pointer.y == 75 ? 0.6 : 0"}},
                {"type":"rectangle","id":"nx","width":10,"height":10,"opacity":0,
                 "bindings":{"opacity":"$pointer.nx"}},
                {"type":"rectangle","id":"ny","width":10,"height":10,"opacity":0,
                 "bindings":{"opacity":"$pointer.ny"}},
                {"type":"rectangle","id":"inside","width":10,"height":10,"opacity":0,
                 "bindings":{"opacity":"$pointer.inside ? 1 : 0"}}
             ]}
        ]
    }"##
}

#[test]
fn pointer_fields_normalize_clamp_and_retain_the_last_coordinates() {
    let mut session = enter(pointer_fixture());
    for id in ["x", "y", "nx", "ny", "inside"] {
        assert_eq!(opacity(&session, id), 0.0, "initial $pointer defaults");
    }

    let hover = session.dispatch_input(pointer(
        7,
        PointerKind::Mouse,
        PointerPhase::Hover,
        150.0,
        75.0,
    ));
    assert!(hover.needs_redraw);
    assert_eq!(opacity(&session, "x"), 0.5);
    assert_eq!(opacity(&session, "y"), 0.6);
    assert_eq!(opacity(&session, "nx"), 0.25);
    assert_eq!(opacity(&session, "ny"), 0.25);
    assert_eq!(opacity(&session, "inside"), 1.0);

    let _ = session.dispatch_input(pointer(
        7,
        PointerKind::Mouse,
        PointerPhase::Cancel,
        0.0,
        0.0,
    ));
    assert_eq!(opacity(&session, "x"), 0.5, "x retains the last value");
    assert_eq!(opacity(&session, "y"), 0.6, "y retains the last value");
    assert_eq!(opacity(&session, "nx"), 0.25);
    assert_eq!(opacity(&session, "ny"), 0.25);
    assert_eq!(
        opacity(&session, "inside"),
        0.0,
        "Cancel removes the hover without publishing compatibility 0,0"
    );

    let _ = session.dispatch_input(pointer(
        9,
        PointerKind::Touch,
        PointerPhase::Down,
        350.0,
        175.0,
    ));
    assert_eq!(opacity(&session, "nx"), 1.0, "normalized x clamps");
    assert_eq!(opacity(&session, "ny"), 1.0, "normalized y clamps");
    assert_eq!(opacity(&session, "inside"), 0.0, "outside the screen");
}

#[test]
fn smallest_active_pointer_id_is_primary_until_it_ends() {
    let mut session = enter(pointer_fixture());
    let _ = session.dispatch_input(pointer(
        7,
        PointerKind::Touch,
        PointerPhase::Down,
        250.0,
        125.0,
    ));
    assert_eq!(opacity(&session, "nx"), 0.75);

    let _ = session.dispatch_input(pointer(
        3,
        PointerKind::Touch,
        PointerPhase::Down,
        150.0,
        75.0,
    ));
    assert_eq!(opacity(&session, "nx"), 0.25, "id 3 outranks id 7");

    let _ = session.dispatch_input(pointer(
        7,
        PointerKind::Touch,
        PointerPhase::Move,
        280.0,
        125.0,
    ));
    assert_eq!(opacity(&session, "nx"), 0.25, "secondary motion is inert");

    let _ = session.dispatch_input(pointer(
        3,
        PointerKind::Touch,
        PointerPhase::Up,
        150.0,
        75.0,
    ));
    assert_eq!(opacity(&session, "nx"), 0.9, "id 7 becomes primary");
}

#[test]
fn legacy_hover_reports_pointer_binding_redraw_work() {
    let mut session = enter(pointer_fixture());
    let changed = session.dispatch_pointer_for_id_at(
        1,
        PointerKind::Mouse,
        150.0,
        75.0,
        PointerPhase::Hover,
        0,
    );
    assert!(changed, "legacy hosts must schedule the PaintOnly redraw");
    assert_eq!(opacity(&session, "nx"), 0.25);
}

#[test]
fn pointer_bindings_reject_non_paint_targets_with_one_diagnostic() {
    let source = r##"{
        "version":"1.1","formatVersion":"1.1","id":"pointer-restriction",
        "app":{"name":"pointer-restriction","version":"1","id":"pointer-restriction"},
        "children":[
            {"type":"rectangle","id":"bad","width":100,"height":20,
             "bindings":{"width":"$pointer.nx * 100"}},
            {"type":"rectangle","id":"good","width":100,"height":20,"opacity":1,
             "bindings":{"opacity":"$pointer.nx"}}
        ]
    }"##;
    let session = enter(source);
    assert_eq!(
        session.binding_sites_len_for_test(),
        1,
        "Relayout is rejected while PaintOnly remains"
    );
    let diagnostics: Vec<_> = session
        .warnings()
        .iter()
        .filter(|warning| warning.contains("PointerBindingRequiresPaintOnly"))
        .collect();
    assert_eq!(diagnostics.len(), 1, "one explicit rejection diagnostic");
}
