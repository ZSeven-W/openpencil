//! Bound text must remeasure the painted scene, not only the engine tree.

use super::{test_measure, PreviewSession};
use std::collections::BTreeMap;

fn enter(width: &str, layout: &str) -> PreviewSession {
    enter_with_state(width, layout, false)
}

fn enter_with_state(width: &str, layout: &str, ended: bool) -> PreviewSession {
    let growth = if width == "80" { "fixed-width" } else { "auto" };
    let source = format!(
        r##"{{"version":"1.1","state":{{"ended":{{"type":"bool","default":{ended}}}}},
        "children":[{{"type":"frame","id":"screen","x":80,"y":60,"width":375,"height":180,"layout":"{layout}","gap":16,"padding":16,"children":[
            {{"type":"text","id":"status","width":{width},"height":"fit_content","textGrowth":"{growth}","fontSize":13,"content":"预计12分钟","bindings":{{"content":"$app.ended ? '可随时恢复导航' : '预计12分钟'"}}}},
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
        test_measure(),
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
    let mut session = enter("\"fit_content\"", "horizontal");
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

#[test]
fn fixed_width_bound_text_wraps_and_moves_the_row_below() {
    let mut session = enter("80", "vertical");
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

#[test]
fn initial_bound_content_uses_its_actual_width_before_any_input() {
    let short = enter_with_state("\"fit_content\"", "horizontal", false);
    let long = enter_with_state("\"fit_content\"", "horizontal", true);
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
