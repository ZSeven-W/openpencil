use super::{test_measure, PreviewSession};
use std::collections::BTreeMap;

fn enter() -> PreviewSession {
    let doc=jian_ops_schema::load_str(r##"{
        "version":"1.1","state":{"count":{"type":"int","default":0}},
        "children":[{"type":"frame","id":"screen","width":375,"height":812,"layout":"none","children":[
            {"type":"frame","id":"hidden","x":16,"y":100,"width":80,"height":48,"visible":false,"events":{"onTap":[{"set":{"$app.count":"99"}}]}},
            {"type":"frame","id":"button","x":16,"y":100,"width":80,"height":48,"layout":"horizontal","events":{"onTap":[{"set":{"$app.count":"$app.count + 1"}}]},"children":[
                {"type":"text","id":"label","width":80,"height":48,"content":"0","bindings":{"content":"'' + $app.count"}}
            ]},
            {"type":"frame","id":"map","width":375,"height":812,"layout":"none","children":[
                {"type":"frame","id":"streets","width":375,"height":812,"layout":"none","children":[
                    {"type":"rectangle","id":"block","width":375,"height":812,"fill":[{"type":"solid","color":"#102030"}]}
                ]}
            ]}
        ]}]
    }"##).unwrap().value;
    PreviewSession::enter(
        &doc,
        (800.0, 1000.0),
        &BTreeMap::new(),
        0,
        false,
        false,
        test_measure(),
        0,
    )
    .unwrap()
}

#[test]
fn painted_foreground_wins_over_deeper_background_and_invisible_overlay() {
    let mut session = enter();
    assert_eq!(session.deepest_mapped_hit(40.0, 120.0).unwrap().2, "label");
    session.dispatch_tap(40.0, 120.0);
    let scene = session.preview_scene_for_test();
    assert_eq!(
        scene
            .active_page()
            .unwrap()
            .find("label")
            .unwrap()
            .text
            .as_deref(),
        Some("1")
    );
}

#[test]
fn canonical_pointer_path_uses_the_same_visible_foreground() {
    use crate::{PreviewInput, PreviewInputEnvelope};
    use jian_core::gesture::{pointer::PointerEvent, PointerPhase};
    let mut session = enter();
    for (phase, time) in [(PointerPhase::Down, 100), (PointerPhase::Up, 150)] {
        session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Pointer(
            PointerEvent::simple_at(1, phase, jian_core::geometry::point(40.0, 120.0), time),
        )));
    }
    let scene = session.preview_scene_for_test();
    assert_eq!(
        scene
            .active_page()
            .unwrap()
            .find("label")
            .unwrap()
            .text
            .as_deref(),
        Some("1")
    );
}
