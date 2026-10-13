//! Reject motion that the canonical document loader cannot reopen.
use super::{SelfCheckIssue, SelfCheckReport, SelfCheckSeverity};
use serde_json::{json, Value};

pub(super) fn check(node: &Value, report: &mut SelfCheckReport) {
    let Some(animations) = node.get("animations") else {
        return;
    };
    // Isolate the declaration: do not reload images, promote widgets, or
    // validate children twice. The loader owns keyframe/property validation.
    let document = json!({"version":"1.0", "children":[{
        "type":"frame", "id":"motion-check", "animations":animations
    }]});
    if let Err(error) = jian_ops_schema::load_str(&document.to_string()) {
        report.issues.push(SelfCheckIssue {
            code: "unloadable-motion",
            node_id: node["id"].as_str().map(str::to_owned),
            message: format!(
                "{error}. Keep the design and correct the animation declaration: keyframes must start at offset 0, end at 1, and be sorted. Supported properties: {}. Use scaleX/scaleY for scaling; value is not an animation property. Preserve readable static text and valid motion.",
                jian_ops_schema::motion::ANIMATION_REGISTRY_PROPERTIES.join(", ")
            ),
            severity: SelfCheckSeverity::Fatal,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration_self_check::{check_value_forest, IntentCheckMode};

    fn node(property: &str, offsets: [f32; 2]) -> Value {
        json!({"type":"frame","id":"metric","animations":[{
            "trigger":"mount", "durationMs":800,
            "keyframes":[
                {"offset":offsets[0],"values":{property:0}},
                {"offset":offsets[1],"values":{property:1}}
            ]
        }]})
    }

    #[test]
    fn rejects_actual_unloadable_properties_even_on_final_attempt() {
        for property in ["value", "scale"] {
            let report = check_value_forest(&node(property, [0.0, 1.0]), 375.0);
            assert!(report.rejects(IntentCheckMode::Advisory));
            assert!(report
                .failure_message()
                .contains("unknown animation property"));
            assert_eq!(report.issues[0].node_id.as_deref(), Some("metric"));
        }
    }

    #[test]
    fn uses_loader_offset_contract_and_accepts_valid_motion_without_changes() {
        assert!(check_value_forest(&node("opacity", [0.2, 1.0]), 375.0).has_fatal());
        for property in jian_ops_schema::motion::ANIMATION_REGISTRY_PROPERTIES {
            let authored = node(property, [0.0, 1.0]);
            assert!(
                !check_value_forest(&authored, 375.0).has_fatal(),
                "{property}"
            );
        }
        let nested = json!({"type":"frame","id":"root","children":[node("scale", [0.0,1.0])]});
        assert!(check_value_forest(&nested, 375.0).has_fatal());
    }
}
