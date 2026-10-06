use super::*;
use crate::design_form::DesignForm;
use crate::issue::{FixProperty, IssueSeverity};

fn root(children: serde_json::Value) -> PenNode {
    serde_json::from_value(serde_json::json!({
        "type":"frame","id":"root","width":390,"height":844,"children":children
    }))
    .expect("lint fixture")
}

#[test]
fn warns_for_more_than_32_animated_nodes() {
    let children = (0..33)
        .map(|i| serde_json::json!({"type":"rectangle","id":format!("n{i}"),"animations":[
            {"trigger":"mount","keyframes":[{"offset":0,"values":{"opacity":0}},{"offset":1,"values":{"opacity":1}}],"durationMs":200}
        ]}))
        .collect();
    let issues = detect_motion_budget(&root(children), DesignForm::MobileScreen);
    assert!(issues.iter().any(|issue| issue.reason.contains("32")));
    assert!(issues
        .iter()
        .all(|issue| issue.severity == IssueSeverity::Warning));
    assert!(issues
        .iter()
        .all(|issue| issue.property == FixProperty::None));
}

#[test]
fn warns_for_long_duration_and_non_whitelisted_property() {
    let node = serde_json::json!({
        "type":"rectangle","id":"hero",
        "animations":[{"trigger":"mount","keyframes":[
            {"offset":0,"values":{"opacity":0,"width":10}},
            {"offset":1,"values":{"opacity":1,"width":100}}
        ],"durationMs":1501}],
        "transition":{"properties":["x"]}
    });
    let issues = detect_motion_budget(
        &root(serde_json::Value::Array(vec![node])),
        DesignForm::Page,
    );
    assert!(issues.iter().any(|issue| issue.reason.contains("1500")));
    assert!(issues.iter().any(|issue| issue.reason.contains("`x`")));
}

#[test]
fn clean_motion_document_has_no_motion_budget_warning() {
    let node = serde_json::json!({
        "type":"rectangle","id":"hero",
        "animations":[{"trigger":"mount","keyframes":[
            {"offset":0,"values":{"opacity":0}},
            {"offset":1,"values":{"opacity":1}}
        ],"durationMs":400}],
        "transition":{"durationMs":200,"properties":["opacity","fill"]}
    });
    assert!(detect_motion_budget(
        &root(serde_json::Value::Array(vec![node])),
        DesignForm::Page
    )
    .is_empty());
}
