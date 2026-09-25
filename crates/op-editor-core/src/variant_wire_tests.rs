//! Round-trip and compatibility tests for the variants wire DTO.

use super::*;
use serde_json::json;

fn landed() -> WorkspaceVariant {
    let mut variables = BTreeMap::new();
    variables.insert(
        "accent".to_string(),
        serde_json::from_value(json!({"type": "color", "value": "#FF5500"})).unwrap(),
    );
    let mut themes = BTreeMap::new();
    themes.insert("mode".to_string(), vec!["light".to_string()]);
    WorkspaceVariant {
        index: 1,
        name: "方案 B".into(),
        style_guide: "zen-paper-light".into(),
        style_label: "Zen Paper Light".into(),
        name_prefix: "方案 B · Zen Paper Light · ".into(),
        root_ids: vec!["r1".into(), "r2".into()],
        variables: Some(variables),
        themes: Some(themes),
    }
}

#[test]
fn a_ready_report_round_trips_to_the_same_workspace_variant() {
    let wire = VariantEventWire::ready(&landed());
    let encoded = serde_json::to_value(&wire).unwrap();
    assert_eq!(encoded["v"], 1);
    assert_eq!(encoded["phase"], "ready");
    assert_eq!(encoded["index"], 1);
    assert_eq!(encoded["styleLabel"], "Zen Paper Light");
    assert_eq!(encoded["rootIds"], json!(["r1", "r2"]));

    let decoded = VariantEventWire::decode(&encoded).expect("decodes");
    assert_eq!(decoded, wire);
    assert_eq!(decoded.to_workspace_variant(), Some(landed()));
}

#[test]
fn a_failed_report_round_trips_and_is_no_workspace_variant() {
    let wire = VariantEventWire::failed(2, "Direction C", "every section failed");
    let encoded = serde_json::to_value(&wire).unwrap();
    assert_eq!(
        encoded,
        json!({"v": 1, "index": 2, "name": "Direction C", "phase": "failed",
               "error": "every section failed"})
    );
    let decoded = VariantEventWire::decode(&encoded).expect("decodes");
    assert_eq!(decoded, wire);
    assert_eq!(decoded.to_workspace_variant(), None);
}

#[test]
fn unknown_fields_are_ignored_but_other_versions_and_phases_are_dropped() {
    let additive = json!({"v": 1, "index": 0, "name": "A", "phase": "failed",
                          "error": "x", "futureField": true});
    assert!(VariantEventWire::decode(&additive).is_some());

    let newer = json!({"v": 2, "index": 0, "name": "A", "phase": "failed", "error": "x"});
    assert_eq!(VariantEventWire::decode(&newer), None);

    let unknown_phase = json!({"v": 1, "index": 0, "name": "A", "phase": "started"});
    assert_eq!(VariantEventWire::decode(&unknown_phase), None);

    assert_eq!(VariantEventWire::decode(&json!(42)), None);
}
