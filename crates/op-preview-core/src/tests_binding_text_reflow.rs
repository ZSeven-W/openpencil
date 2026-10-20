//! Bound text must remeasure the painted scene, not only the engine tree.

use super::PreviewSession;
use crate::tests::measure_backends;
use jian_core::layout::measure::{FontStyleKind, MeasureBackend, MeasureRequest, StyledRun};
use std::collections::BTreeMap;
use std::rc::Rc;

const SHORT: &str = "预计12分钟";
const LONG: &str = "导航已结束，可随时恢复导航";

fn assert_fixture_preconditions(backend: &str, measure: &dyn MeasureBackend) {
    let measured = |text: &str, max_width| {
        let runs = [StyledRun {
            text,
            font_family: None,
            font_size: 13.0,
            font_weight: 400,
            font_style: FontStyleKind::Normal,
            letter_spacing: 0.0,
        }];
        measure.measure(&MeasureRequest {
            runs: &runs,
            line_height: 0.0,
            max_width,
        })
    };
    let short = measured(SHORT, None);
    let long = measured(LONG, None);
    assert!(
        long.width > short.width + 15.0,
        "{backend}: short={short:?}, long={long:?}"
    );
    assert!(
        short.width < 80.0 && long.width > 80.0,
        "{backend}: short={short:?}, long={long:?}"
    );
    let wrapped = measured(LONG, Some(80.0));
    assert!(
        wrapped.height > measured(SHORT, Some(80.0)).height + 5.0,
        "{backend}: wrapped={wrapped:?}"
    );
}

fn enter(
    width: &str,
    layout: &str,
    ended: bool,
    measure: Rc<dyn MeasureBackend>,
) -> PreviewSession {
    let growth = if width == "80" { "fixed-width" } else { "auto" };
    let source = format!(
        r##"{{"version":"1.1","state":{{"ended":{{"type":"bool","default":{ended}}}}},
        "children":[{{"type":"frame","id":"screen","x":80,"y":60,"width":375,"height":180,"layout":"{layout}","gap":16,"padding":16,"children":[
            {{"type":"text","id":"status","width":{width},"height":"fit_content","textGrowth":"{growth}","fontSize":13,"content":"{SHORT}","bindings":{{"content":"$app.ended ? '{LONG}' : '{SHORT}'"}}}},
            {{"type":"frame","id":"button","width":92,"height":44,"events":{{"onTap":[{{"toggle":"$app.ended"}}]}},"children":[{{"type":"text","id":"action-label","content":"结束导航","fontSize":14}}]}}
        ]}}]}}"##
    );
    let doc = jian_ops_schema::load_str(&source).unwrap().value;
    PreviewSession::enter(
        &doc,
        (800.0, 600.0),
        &BTreeMap::new(),
        0,
        false,
        false,
        measure,
        0,
    )
    .unwrap()
}

fn tap(session: &mut PreviewSession, now: u64) {
    let scene = session.preview_scene_for_test();
    let rect = scene.active_page().unwrap().find("button").unwrap().bounds;
    session.set_now_ms(now);
    assert!(session.dispatch_tap(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0
    ));
}

#[test]
fn auto_width_bound_text_reflows_its_following_sibling_and_restores() {
    for (backend, measure) in measure_backends() {
        assert_fixture_preconditions(backend, measure.as_ref());
        let mut session = enter("\"fit_content\"", "horizontal", false, measure.clone());
        let before = session.preview_scene_for_test();
        let before = before.active_page().unwrap();
        let width = before.find("status").unwrap().bounds.size.x;
        let button_x = before.find("button").unwrap().bounds.origin.x;
        tap(&mut session, 100);
        let after = session.preview_scene_for_test();
        let after = after.active_page().unwrap();
        assert!(after.find("status").unwrap().bounds.size.x > width + 15.0);
        assert!(after.find("button").unwrap().bounds.origin.x > button_x + 15.0);
        tap(&mut session, 700);
        let restored = session.preview_scene_for_test();
        let restored = restored.active_page().unwrap();
        assert!((restored.find("status").unwrap().bounds.size.x - width).abs() < 0.1);
        assert!((restored.find("button").unwrap().bounds.origin.x - button_x).abs() < 0.1);
    }
}

#[test]
fn fixed_width_bound_text_wraps_and_moves_the_row_below() {
    for (backend, measure) in measure_backends() {
        assert_fixture_preconditions(backend, measure.as_ref());
        let mut session = enter("80", "vertical", false, measure.clone());
        let before = session.preview_scene_for_test();
        let before = before.active_page().unwrap();
        let height = before.find("status").unwrap().bounds.size.y;
        let button_y = before.find("button").unwrap().bounds.origin.y;
        tap(&mut session, 100);
        let after = session.preview_scene_for_test();
        let after = after.active_page().unwrap();
        assert_eq!(after.find("status").unwrap().bounds.size.x, 80.0);
        assert!(after.find("status").unwrap().bounds.size.y > height + 5.0);
        assert!(after.find("button").unwrap().bounds.origin.y > button_y + 5.0);
    }
}

#[test]
fn initial_bound_content_uses_its_actual_width_before_any_input() {
    for (backend, measure) in measure_backends() {
        assert_fixture_preconditions(backend, measure.as_ref());
        let short = enter("\"fit_content\"", "horizontal", false, measure.clone());
        let long = enter("\"fit_content\"", "horizontal", true, measure.clone());
        let short = short.preview_scene_for_test();
        let long = long.preview_scene_for_test();
        let short = short.active_page().unwrap();
        let long = long.active_page().unwrap();
        assert!(
            long.find("status").unwrap().bounds.size.x
                > short.find("status").unwrap().bounds.size.x + 15.0
        );
        assert!(
            long.find("button").unwrap().bounds.origin.x
                > short.find("button").unwrap().bounds.origin.x + 15.0
        );
    }
}
