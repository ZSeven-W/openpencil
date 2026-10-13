use super::*;
use serde_json::json;

fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
    Rect { x, y, w, h }
}

fn viewport_pair(clipped: bool) -> (Value, HashMap<String, Rect>) {
    let root = json!({"type":"frame","id":"root","children":[
        {"type":"frame","id":"viewport","clipContent":clipped,"children":[
            {"type":"frame","id":"content","children":[
                {"type":"text","id":"body","content":"Body copy"}
            ]}
        ]},
        {"type":"frame","id":"nav","children":[
            {"type":"text","id":"label","content":"Fixed navigation"}
        ]}
    ]});
    let rects = HashMap::from([
        ("viewport".into(), rect(0.0, 0.0, 100.0, 100.0)),
        ("body".into(), rect(10.0, 110.0, 80.0, 20.0)),
        ("label".into(), rect(10.0, 110.0, 80.0, 20.0)),
    ]);
    (root, rects)
}

#[test]
fn text_fully_outside_an_ancestor_clip_does_not_collide_with_fixed_chrome() {
    let (root, rects) = viewport_pair(true);
    assert!(collect_text_collisions(&root, &rects).is_empty());
}

#[test]
fn unclipped_overflow_still_collides() {
    let (root, rects) = viewport_pair(false);
    assert_eq!(collect_text_collisions(&root, &rects).len(), 1);
}

#[test]
fn partially_visible_text_still_reports_its_visible_overlap() {
    let (root, mut rects) = viewport_pair(true);
    rects.insert("body".into(), rect(10.0, 90.0, 80.0, 20.0));
    rects.insert("label".into(), rect(10.0, 92.0, 80.0, 20.0));
    let collisions = collect_text_collisions(&root, &rects);
    assert_eq!(collisions.len(), 1);
    assert_eq!(collisions[0].a_rect.h, 10.0);
    assert_eq!(collisions[0].overlap_y, 8.0);
    assert_eq!(collisions[0].overlap_area_ratio, 0.8);
}

#[test]
fn overlap_only_in_the_clipped_part_is_not_reported() {
    let (root, mut rects) = viewport_pair(true);
    rects.insert("body".into(), rect(10.0, 90.0, 80.0, 30.0));
    assert!(collect_text_collisions(&root, &rects).is_empty());
}

#[test]
fn nested_clips_intersect_instead_of_replacing_the_outer_clip() {
    let (mut root, mut rects) = viewport_pair(true);
    root["children"][0]["children"][0]["clipContent"] = json!(true);
    rects.insert("content".into(), rect(50.0, 80.0, 100.0, 100.0));
    rects.insert("body".into(), rect(40.0, 90.0, 80.0, 40.0));
    rects.insert("label".into(), rect(40.0, 90.0, 80.0, 40.0));
    let collisions = collect_text_collisions(&root, &rects);
    assert_eq!(collisions.len(), 1);
    assert_eq!(collisions[0].a_rect.x, 50.0);
    assert_eq!(collisions[0].a_rect.y, 90.0);
    assert_eq!(collisions[0].a_rect.w, 50.0);
    assert_eq!(collisions[0].a_rect.h, 10.0);
}

#[test]
fn disjoint_nested_clips_hide_the_entire_subtree() {
    let (mut root, mut rects) = viewport_pair(true);
    root["children"][0]["children"][0]["clipContent"] = json!(true);
    rects.insert("content".into(), rect(0.0, 110.0, 100.0, 100.0));
    assert!(collect_text_collisions(&root, &rects).is_empty());
}

#[test]
fn missing_or_degenerate_clip_bounds_do_not_hide_overflow() {
    let (root, mut rects) = viewport_pair(true);
    rects.remove("viewport");
    assert_eq!(collect_text_collisions(&root, &rects).len(), 1);
    rects.insert("viewport".into(), rect(0.0, 0.0, 100.0, 0.0));
    assert_eq!(collect_text_collisions(&root, &rects).len(), 1);
}

#[test]
fn measured_fitness_scroll_body_and_fixed_nav_have_no_visible_collisions() {
    // The 1013 fitness result had two apparent bar-day/navigation collisions.
    // Both day labels are below the scroll viewport, so neither paints there.
    let mut root = json!({"type":"frame","id":"root","children":[
        {"type":"frame","id":"viewport","clipContent":false,"children":[
            {"type":"text","id":"day1","content":"三"},
            {"type":"text","id":"day2","content":"六"}
        ]},
        {"type":"text","id":"nav1","content":"课程"},
        {"type":"text","id":"nav2","content":"数据"}
    ]});
    let rects = HashMap::from([
        ("viewport".into(), rect(80.0, 102.0, 375.0, 677.0)),
        ("day1".into(), rect(218.0, 830.0, 11.0, 17.0)),
        ("day2".into(), rect(307.0, 830.0, 11.0, 17.0)),
        ("nav1".into(), rect(219.0, 821.0, 23.0, 16.0)),
        ("nav2".into(), rect(293.0, 821.0, 23.0, 16.0)),
    ]);
    assert_eq!(collect_text_collisions(&root, &rects).len(), 2);
    root["children"][0]["clipContent"] = json!(true);
    assert!(collect_text_collisions(&root, &rects).is_empty());
}

/// Read-only native-layout audit of the original document, without embedding
/// its image assets into the repository.
#[test]
#[ignore = "requires OP_TEXT_CLIP_SAMPLE pointing to the original fitness .op"]
fn original_fitness_document_has_no_geometry_issues() {
    let path = std::env::var("OP_TEXT_CLIP_SAMPLE").expect("OP_TEXT_CLIP_SAMPLE");
    let source = std::fs::read_to_string(path).expect("read original .op");
    let doc = jian_ops_schema::load_str(&source)
        .expect("parse original .op")
        .value;
    let state = op_editor_core::EditorState::from_document(doc);
    let rects = super::super::resolved_rects(&state);
    let without_clips: usize = state
        .active_children()
        .iter()
        .map(|root| {
            let mut value = serde_json::to_value(root).expect("serialize root");
            remove_clips(&mut value);
            collect_text_collisions(&value, &rects).len()
        })
        .sum();
    assert_eq!(without_clips, 2, "original unclipped detector baseline");
    let issues = crate::geometry_validation::geometry_diagnostics(&state);
    assert!(issues.is_empty(), "original document issues: {issues:?}");
}

fn remove_clips(node: &mut Value) {
    node.as_object_mut()
        .expect("node object")
        .remove("clipContent");
    if let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) {
        for child in children {
            remove_clips(child);
        }
    }
}
