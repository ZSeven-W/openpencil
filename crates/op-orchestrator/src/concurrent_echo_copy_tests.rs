use super::*;
use crate::test_support::VecDocSink;
use serde_json::{json, Value};

fn sink(children: Value) -> VecDocSink {
    let doc =
        jian_ops_schema::load_str(&json!({"version":"1.0.0","children":children}).to_string())
            .unwrap()
            .value;
    let mut sink = VecDocSink::new();
    sink.state = EditorState::from_document(doc);
    sink
}

fn board(id: &str, copy: &[&str]) -> Value {
    json!({"type":"frame","id":id,"children":copy.iter().enumerate().map(|(i,text)|
        json!({"type":"text","id":format!("{id}-{i}"),"content":text})).collect::<Vec<_>>()})
}

#[test]
fn exact_copy_allows_reordering_styling_and_plain_to_rich_text() {
    let mut candidate = board("new", &["¥28.50", "周六 14:30\n到店自取"]);
    candidate["children"][1]["content"] =
        json!([{"text":"周六 ","fontWeight":700},{"text":"14:30\n到店自取"}]);
    candidate["children"][0]["fontSize"] = json!(48);
    let sink = sink(json!([
        board("old", &["周六 14:30\n到店自取", "¥28.50"]),
        candidate
    ]));
    let original = CopySnapshot::capture(&sink.state, &["old".into()]).unwrap();
    assert!(original.matches(&sink, &["new".into()]));
    let source = original.feedback();
    assert!(source.contains("COPY LOCK"));
    assert!(source.contains(r"14:30\n到店自取"));
}

#[test]
fn prices_punctuation_spacing_newlines_and_duplicate_counts_are_not_layout() {
    for replacement in [
        vec!["¥29.50", "周六 14:30\n到店自取", "报名", "报名"],
        vec!["¥28.50", "周六14:30\n到店自取", "报名", "报名"],
        vec!["¥28.50", "周六 14:30 到店自取", "报名", "报名"],
        vec!["¥28.50", "周六 14:30\n到店自取。", "报名", "报名"],
        vec!["¥28.50", "周六 14:30\n到店自取", "报名"],
        vec!["¥28.50", "周六 14:30\n到店自取", "报名", "报名", "限时优惠"],
    ] {
        let sink = sink(json!([
            board("old", &["¥28.50", "周六 14:30\n到店自取", "报名", "报名"]),
            board("new", &replacement)
        ]));
        let original = CopySnapshot::capture(&sink.state, &["old".into()]).unwrap();
        assert!(!original.matches(&sink, &["new".into()]), "{replacement:?}");
    }
}

#[test]
fn hidden_or_transparent_copies_cannot_satisfy_the_contract() {
    for hidden in [
        json!({"visible":false}),
        json!({"enabled":false}),
        json!({"opacity":0}),
    ] {
        let mut candidate = board("new", &["原样标题"]);
        candidate
            .as_object_mut()
            .unwrap()
            .extend(hidden.as_object().unwrap().clone());
        let sink = sink(json!([board("old", &["原样标题"]), candidate]));
        let original = CopySnapshot::capture(&sink.state, &["old".into()]).unwrap();
        assert!(!original.matches(&sink, &["new".into()]));
    }
}

#[test]
fn hidden_source_copy_is_not_required_but_visible_chrome_is() {
    let mut original = board("old", &["资料", "9:00 AM", "隐藏草稿"]);
    original["children"][2]["visible"] = json!(false);
    let sink = sink(json!([original, board("new", &["资料", "9:00 AM"])]));
    let original = CopySnapshot::capture(&sink.state, &["old".into()]).unwrap();
    assert!(original.matches(&sink, &["new".into()]));
    assert!(!original.matches(&sink, &["missing-root".into()]));
}

#[test]
fn component_overrides_are_compared_as_rendered_copy() {
    let sink = sink(json!([
        {"type":"frame","id":"master","reusable":true,"children":[{"type":"text","id":"label","content":"原始名称"}]},
        {"type":"frame","id":"old","children":[{"type":"ref","id":"instance","ref":"master","descendants":{"label":{"content":"到店自取"}}}]},
        board("new", &["到店自取"]), board("wrong", &["原始名称"])
    ]));
    let original = CopySnapshot::capture(&sink.state, &["old".into()]).unwrap();
    assert!(original.matches(&sink, &["new".into()]));
    assert!(!original.matches(&sink, &["wrong".into()]));
}

#[test]
fn string_variables_are_compared_to_resolved_content() {
    let mut sink = sink(json!([
        board("old", &["$venue"]),
        board("new", &["静安店"]),
        board("wrong", &["徐汇店"])
    ]));
    sink.state.doc.variables =
        Some(serde_json::from_value(json!({"venue":{"type":"string","value":"静安店"}})).unwrap());
    let original = CopySnapshot::capture(&sink.state, &["old".into()]).unwrap();
    assert!(original.matches(&sink, &["new".into()]));
    assert!(!original.matches(&sink, &["wrong".into()]));
}

#[test]
fn a_candidate_cannot_borrow_copy_from_the_subtree_it_would_delete() {
    let sink = sink(json!([
        board("old", &["不能丢失的文字"]),
        {"type":"frame","id":"new","children":[{"type":"ref","id":"borrowed","ref":"old"}]}
    ]));
    let original = CopySnapshot::capture(&sink.state, &["old".into()]).unwrap();
    assert!(original.matches(&sink, &["new".into()]));
    assert!(!original.matches_after_removal(&sink, &["new".into()], &["old".into()]));
    assert_eq!(sink.state.active_children().len(), 2);
}
