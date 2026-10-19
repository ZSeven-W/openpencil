use super::*;
use crate::test_support::VecDocSink;
use serde_json::{json, Value};

fn fixture(name: &str) -> Value {
    let source = match name {
        "workout" => include_str!("fixtures/regress_1019/workout_info.json"),
        "title" => include_str!("fixtures/regress_1019/title_meta.json"),
        "schedule" => include_str!("fixtures/regress_1019/schedule_list.json"),
        "calendar-nav" => include_str!("fixtures/regress_1019/calendar_nav.json"),
        "video-nav" => include_str!("fixtures/regress_1019/video_nav.json"),
        "video" => include_str!("fixtures/regress_1019/video_feed.json"),
        _ => panic!("unknown fixture"),
    };
    serde_json::from_str(source).unwrap()
}

fn task(label: &str, elements: &str) -> Subtask {
    serde_json::from_value(json!({
        "id":"test", "label":label, "elements":elements,
        "region":{"width":375,"height":400}
    }))
    .unwrap()
}

fn inserted(node: Value) -> (VecDocSink, SubtaskOutcome) {
    let id = node["id"].as_str().unwrap().to_owned();
    let sink = VecDocSink {
        state: EditorState::from_document(
            serde_json::from_value(json!({"version":"1.0", "children":[node]})).unwrap(),
        ),
        applied: Vec::new(),
        batch_depth: 0,
    };
    let outcome = SubtaskOutcome {
        id: "test".into(),
        node_count: 1,
        inserted_root_ids: vec![id],
        error: None,
        headline: None,
        subtask: None,
    };
    (sink, outcome)
}

fn remove(node: &mut Value, id: &str) {
    if let Some(children) = node["children"].as_array_mut() {
        children.retain(|child| child["id"] != id);
        for child in children {
            remove(child, id);
        }
    }
}

#[test]
fn app18_workout_and_title_meta_own_summary_values_not_exercise_rows() {
    for name in ["workout", "title"] {
        // Exact visible title-meta prefix from B/app-18/rep-2/smoke.log.
        let task = task(
            "Workout Title & Meta Row",
            "Workout title 'Full Body Strength' (28px bold, one owner of all title copy), meta row of three icon+label items: 45 min duration, 6 exercises, No equipment; no duplicate title",
        );
        let (sink, outcome) = inserted(fixture(name));
        assert_eq!(delivered_item_count(&sink, &outcome.inserted_root_ids), 3);
        assert_eq!(expected_item_count(&task), None);
        assert!(incomplete_attempt(&sink, &task, &outcome).is_none());
    }
}

#[test]
fn exercise_metadata_does_not_hide_a_neighboring_real_list_promise() {
    let task = task("Exercises", "meta row of three icon+label items: 45 min, 12 exercises, No equipment; list of 6 exercises, each with thumbnail, name, sets and reps");
    assert_eq!(expected_item_count(&task), Some(6));
    let (sink, outcome) = inserted(fixture("title"));
    let failure = incomplete_attempt(&sink, &task, &outcome).unwrap();
    assert_eq!((failure.expected, failure.delivered), (6, 3));
}

#[test]
fn app11_schedule_counts_the_card_with_an_ongoing_badge() {
    let task = task("今日日程列表", "4 条日程卡片列表，每条含时间、标题、地点");
    let (sink, outcome) = inserted(fixture("schedule"));
    assert_eq!(delivered_item_count(&sink, &outcome.inserted_root_ids), 3);
    assert_eq!(
        semantic_items::delivered_count(&sink, &outcome.inserted_root_ids, &local_item_text(&task)),
        4
    );
    assert!(incomplete_attempt(&sink, &task, &outcome).is_none());
    let mut missing = fixture("schedule");
    remove(&mut missing, "n254");
    let (sink, outcome) = inserted(missing);
    let failure = incomplete_attempt(&sink, &task, &outcome).unwrap();
    assert_eq!((failure.expected, failure.delivered), (4, 3));
}

#[test]
fn app11_and_app14_navigation_count_badged_tabs_and_publish_button_but_not_fab() {
    for (name, expected, missing_id) in [("calendar-nav", 4, "n705"), ("video-nav", 5, "n258")] {
        let task = task(
            "底部导航栏",
            &format!(
                "{expected} 个图标+文字标签；消息带小圆点，发布入口可以放大；独立悬浮新建按钮"
            ),
        );
        let (sink, outcome) = inserted(fixture(name));
        assert_eq!(delivered_item_count(&sink, &outcome.inserted_root_ids), 3);
        assert_eq!(
            semantic_items::delivered_count(
                &sink,
                &outcome.inserted_root_ids,
                &local_item_text(&task)
            ),
            expected
        );
        assert!(incomplete_attempt(&sink, &task, &outcome).is_none());
        let mut missing = fixture(name);
        remove(&mut missing, missing_id);
        let (sink, outcome) = inserted(missing);
        let failure = incomplete_attempt(&sink, &task, &outcome).unwrap();
        assert_eq!(
            (failure.expected, failure.delivered),
            (expected, expected - 1)
        );
    }
}

#[test]
fn app14_video_feed_counts_lead_and_row_wrapped_tiles_without_avatars() {
    let task = task(
        "推荐视频流",
        "5 张视频卡片；全宽大卡后接两列视频卡片网格，每卡有封面、标题、作者和播放量",
    );
    let (sink, outcome) = inserted(fixture("video"));
    assert_eq!(delivered_item_count(&sink, &outcome.inserted_root_ids), 2);
    // The retained tree has 1 lead + 5 tiles, more than the promised 5.
    assert_eq!(
        semantic_items::delivered_count(&sink, &outcome.inserted_root_ids, &local_item_text(&task)),
        6
    );
    assert!(incomplete_attempt(&sink, &task, &outcome).is_none());

    let mut missing = fixture("video");
    remove(&mut missing, "n181");
    remove(&mut missing, "n232");
    let (sink, outcome) = inserted(missing);
    let failure = incomplete_attempt(&sink, &task, &outcome).unwrap();
    assert_eq!((failure.expected, failure.delivered), (5, 4));
}

#[test]
fn semantic_evidence_excludes_hidden_cards_and_stays_within_inserted_roots() {
    let mut node = fixture("video");
    remove(&mut node, "n181");
    fn hide(node: &mut Value) {
        if node["id"] == "n232" {
            node["opacity"] = json!(0);
        }
        if let Some(children) = node["children"].as_array_mut() {
            for child in children {
                hide(child);
            }
        }
    }
    hide(&mut node);
    let (sink, outcome) = inserted(node);
    assert_eq!(
        semantic_items::delivered_count(&sink, &outcome.inserted_root_ids, "video feed"),
        4
    );
    assert_eq!(
        semantic_items::delivered_count(&sink, &["nonexistent".into()], "video feed"),
        0
    );
    assert_eq!(
        semantic_items::delivered_count(&sink, &outcome.inserted_root_ids, "unrelated list"),
        0
    );
}
