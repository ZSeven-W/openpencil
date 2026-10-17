use super::*;
use crate::test_support::VecDocSink;
use jian_ops_schema::node::PenNode;

fn section(damaged: bool) -> Value {
    // Retained n200 subtree from the 1018 full-pipeline workout run-2.
    let mut section: Value = serde_json::from_str(include_str!(
        "test_fixtures/workout_difficulty_badges_1018.json"
    ))
    .unwrap();
    if !damaged {
        let row = &mut section["children"][0];
        row.as_object_mut().unwrap().remove("clipContent");
        let last = &mut row["children"][2];
        last["width"] = json!("fit_content");
        last.as_object_mut().unwrap().remove("clipContent");
    }
    section
}

fn sink(section: Value, width: f64) -> VecDocSink {
    let doc = serde_json::from_value(json!({"version":"1.0", "children":[{
        "type":"frame", "id":"root", "width":width, "height":"fit_content",
        "layout":"vertical", "children":[section]
    }]}))
    .unwrap();
    VecDocSink {
        state: EditorState::from_document(doc),
        applied: Vec::new(),
        batch_depth: 0,
    }
}

fn tree(sink: &VecDocSink) -> Value {
    serde_json::to_value(&sink.state.doc).unwrap()
}

fn by_id<'a>(v: &'a Value, id: &str) -> Option<&'a Value> {
    if v.get("id").and_then(Value::as_str) == Some(id) {
        return Some(v);
    }
    match v {
        Value::Object(obj) => obj.values().find_map(|c| by_id(c, id)),
        Value::Array(items) => items.iter().find_map(|c| by_id(c, id)),
        _ => None,
    }
}

fn assert_complete(sink: &VecDocSink, before: &Value) {
    let after = tree(sink);
    let row = by_id(&after, "n201").unwrap();
    assert_eq!(row["layout"], "vertical");
    assert_eq!(
        children(row)
            .iter()
            .map(|line| children(line).len())
            .collect::<Vec<_>>(),
        vec![2, 1]
    );
    let rects = resolved_rects(sink.state());
    let rail = rects["n201"];
    assert_eq!(rail.w, 327.0);
    assert!(
        rail.h >= 72.0 && rail.h <= 74.0,
        "native reflow height: {rail:?}"
    );
    for (chip, text) in [("n202", "n204"), ("n205", "n207"), ("n208", "n210")] {
        let badge = by_id(&after, chip).unwrap();
        assert_eq!(badge["width"], "fit_content");
        assert_eq!(
            by_id(&after, text),
            by_id(before, text),
            "label styling and content survive: {text}"
        );
        let b = rects[chip];
        let t = rects[text];
        assert!(
            b.x >= rail.x && b.x + b.w <= rail.x + rail.w + 1.0,
            "badge fits: {b:?}"
        );
        assert!(
            t.x + t.w <= b.x + b.w - 12.0 + 1.0,
            "complete label and right inset: {t:?} {b:?}"
        );
        assert!(t.h <= 18.0 + 1.0, "label never wraps or shrinks");
        assert_ne!(badge.get("clipContent"), Some(&json!(true)));
    }
    assert!(rects["n208"].y >= rects["n202"].y + rects["n202"].h + 8.0);
}

#[test]
fn retained_metadata_badges_wrap_before_generic_clip_and_preserve_text() {
    for damaged in [false, true] {
        let mut sink = sink(section(damaged), 375.0);
        let before = tree(&sink);
        geometry_validate_and_fix(&mut sink, "root");
        assert_complete(&sink, &before);
        let settled = tree(&sink);
        geometry_validate_and_fix(&mut sink, "root");
        assert_eq!(
            tree(&sink),
            settled,
            "the complete cleanup loop is idempotent"
        );
    }
}

#[test]
fn metadata_badges_keep_fitting_rows_and_preserve_interactive_or_authored_layouts() {
    for kind in [
        "fits",
        "filter",
        "category",
        "tabs",
        "navigation",
        "selector",
        "scroll",
        "events",
        "badge-events",
        "fixed-height",
        "fixed-badge",
        "positioned",
        "unknown",
    ] {
        let mut input = section(false);
        let width = if kind == "fits" { 480.0 } else { 375.0 };
        match kind {
            "fits" => (),
            "filter" | "category" | "tabs" | "navigation" | "selector" | "scroll" => {
                input["children"][0]["name"] = json!(format!("Difficulty {kind} row"));
            }
            "events" => {
                input["children"][0]["events"] =
                    json!({"onClick":[{"set":{"$state.selected":"true"}}]});
            }
            "badge-events" => {
                input["children"][0]["children"][0]["events"] =
                    json!({"onClick":[{"set":{"$state.selected":"true"}}]});
            }
            "fixed-height" => input["children"][0]["height"] = json!(32),
            "fixed-badge" => input["children"][0]["children"][0]["width"] = json!(122),
            "positioned" => input["children"][0]["x"] = json!(12),
            "unknown" => input["name"] = json!("Ordinary content"),
            _ => unreachable!(),
        }
        let mut sink = sink(input, width);
        let before = tree(&sink);
        assert!(!repair_metadata_badges(&mut sink, "root"), "{kind}");
        assert_eq!(tree(&sink), before, "scope exclusion: {kind}");
    }
}

#[test]
fn metadata_badges_inside_vertical_scroll_content_recover_saved_damage() {
    let input = json!({"type":"frame", "id":"viewport", "name":"Scrollable content",
        "width":"fill_container", "height":500, "layout":"vertical", "clipContent":true,
        "role":"scroll-area", "state":{"scrolled":{"type":"bool", "default":false}},
        "events":{"onScroll":[{"set":{"$state.scrolled":"true"}}]},
        "children":[section(true)]});
    let mut sink = sink(input, 375.0);
    let before = tree(&sink);
    assert!(repair_metadata_badges(&mut sink, "root"));
    assert_complete(&sink, &before);
    assert_eq!(
        by_id(&tree(&sink), "viewport").unwrap()["height"].as_f64(),
        Some(500.0)
    );
}

#[test]
fn metadata_badges_do_not_rewrite_tables_absolute_stacks_or_oversized_individual_labels() {
    for kind in ["table", "absolute", "one-too-wide"] {
        let mut input = section(false);
        match kind {
            "table" => {
                input["children"] = json!((0..2).map(|i| json!({
                    "type":"frame", "id":format!("table-row-{i}"), "layout":"horizontal", "children":[
                        {"type":"text", "id":format!("table-name-{i}"), "width":120, "content":"Name"},
                        {"type":"text", "id":format!("table-value-{i}"), "width":120, "content":"Value"},
                        {"type":"text", "id":format!("table-status-{i}"), "width":120, "content":"Status"}
                    ]
                })).collect::<Vec<_>>());
            }
            "absolute" => input["layout"] = json!("none"),
            "one-too-wide" => {
                input["children"][0]["children"][0]["children"][1]["content"] =
                    json!("WWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWW");
            }
            _ => unreachable!(),
        }
        let mut sink = sink(input, 375.0);
        let before = tree(&sink);
        assert!(!repair_metadata_badges(&mut sink, "root"), "{kind}");
        assert_eq!(tree(&sink), before);
    }
}

#[test]
fn metadata_badge_line_height_participates_in_the_existing_native_section_budget() {
    let plan = crate::plan::parse_plan(
        &json!({
            "rootFrame":{"id":"page", "name":"Workout Detail", "width":375, "height":812},
            "subtasks":[{"id":"summary", "label":"Difficulty Badges",
                "elements":"OP DETAIL BUDGET: retain all metadata",
                "region":{"width":375,"height":50}}]
        })
        .to_string(),
    )
    .unwrap();
    let nodes: Vec<PenNode> = vec![serde_json::from_value(section(false)).unwrap()];
    let context = EditorState::new();
    let reason = crate::first_viewport::rejection(&plan.subtasks[0], &nodes, &context);
    assert!(
        reason
            .as_deref()
            .is_some_and(|s| s.contains("measures 72px")),
        "{reason:?}"
    );
    let mut task = plan.subtasks[0].clone();
    task.region.height = 80.0;
    assert!(crate::first_viewport::rejection(&task, &nodes, &context).is_none());
}
