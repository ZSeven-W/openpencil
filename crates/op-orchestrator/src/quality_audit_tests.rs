use super::*;
use serde_json::json;

fn state_from(children: serde_json::Value) -> EditorState {
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(json!({
        "version": "1.0",
        "children": children,
    }))
    .expect("doc");
    EditorState::from_document(doc)
}

fn fixture() -> EditorState {
    state_from(json!([
        {
            "type": "frame", "id": "home", "name": "Home", "layout": "vertical",
            "width": 390, "height": 844,
            "fill": [{"type": "solid", "color": "#FFFFFF"}],
            "children": [
                {"type": "text", "id": "faint", "name": "Caption", "content": "Hello there",
                 "fill": [{"type": "solid", "color": "#F4F4F4"}]},
                {"type": "text", "id": "ok", "name": "Title", "content": "Readable",
                 "fill": [{"type": "solid", "color": "#111111"}]}
            ]
        },
        {
            "type": "frame", "id": "empty", "name": "Saved", "layout": "vertical",
            "width": 390, "height": 844, "children": []
        },
        {
            "type": "frame", "id": "older", "name": "Earlier design", "layout": "vertical",
            "width": 390, "height": 844,
            "fill": [{"type": "solid", "color": "#FFFFFF"}],
            "children": [
                {"type": "text", "id": "old-faint", "content": "Old",
                 "fill": [{"type": "solid", "color": "#FAFAFA"}]}
            ]
        }
    ]))
}

#[test]
fn audit_reports_real_findings_on_the_run_boards_only() {
    let state = fixture();
    let audit = audit_final_quality(&state, &["home".to_string(), "empty".to_string()]);
    let contrast: Vec<_> = audit
        .remaining
        .iter()
        .filter(|item| item.topic == QualityTopic::Contrast)
        .collect();
    assert_eq!(contrast.len(), 1, "{:?}", audit.remaining);
    assert_eq!(contrast[0].node_id.as_deref(), Some("faint"));
    assert_eq!(contrast[0].node_name.as_deref(), Some("Caption"));
    assert_eq!(contrast[0].board_id.as_deref(), Some("home"));
    assert_eq!(contrast[0].source, "text-bg-contrast");
    assert!(
        contrast[0].detail.contains("contrast"),
        "{}",
        contrast[0].detail
    );

    let unfilled: Vec<_> = audit
        .remaining
        .iter()
        .filter(|item| item.topic == QualityTopic::Completeness)
        .collect();
    assert_eq!(unfilled.len(), 1);
    assert_eq!(unfilled[0].node_id.as_deref(), Some("empty"));

    // A board that predates the run is not this run's report.
    assert!(!audit
        .remaining
        .iter()
        .any(|item| item.board_id.as_deref() == Some("older")));
    assert!(audit.audited_topics.contains(&QualityTopic::Contrast));
    assert!(audit.audited_topics.contains(&QualityTopic::Completeness));
}

#[test]
fn audit_without_boards_claims_nothing() {
    let state = fixture();
    let audit = audit_final_quality(&state, &["missing".to_string()]);
    assert!(audit.audited_topics.is_empty());
    assert!(audit.remaining.is_empty());
}

#[test]
fn quality_checked_items_mirror_the_rendered_records() {
    let record = crate::RepairRecord {
        pass: "geometry-validation".into(),
        category: crate::CheckCategory::Overflow,
        node_id: "n7".into(),
        node_name: Some("Hero".into()),
        detail: "width 420 → 327".into(),
    };
    let item = record.quality_item();
    assert_eq!(item.pass, "geometry-validation");
    assert_eq!(item.family, "overflow");
    assert_eq!(item.node_id, "n7");
    assert_eq!(item.node_name.as_deref(), Some("Hero"));
    assert_eq!(item.detail, "width 420 → 327");
}
