//! Runtime bindings do not supply the editor canvas's static paint.
use super::{SelfCheckIssue, SelfCheckReport, SelfCheckSeverity};
use serde_json::Value;

fn runtime_expression(value: &str) -> bool {
    let value = value.trim();
    value.contains('$')
        && (value.contains('?')
            || value.contains("==")
            || value.contains("!=")
            || value.starts_with("$app.")
            || value.starts_with("$state."))
}

fn invalid_color(value: &Value) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(key, value)| {
            (key == "color" && value.as_str().is_some_and(runtime_expression))
                || invalid_color(value)
        }),
        Value::Array(values) => values.iter().any(invalid_color),
        _ => false,
    }
}

pub(super) fn check(node: &Value, report: &mut SelfCheckReport) {
    if ["fill", "stroke"]
        .iter()
        .any(|key| invalid_color(&node[key]))
    {
        report.issues.push(SelfCheckIssue {
            code: "runtime-expression-in-static-paint",
            node_id: node["id"].as_str().map(str::to_owned),
            message: "fill/stroke color contains a runtime expression, which renders black in the design canvas. Use a literal initial color (or a design-variable reference) in static fill/stroke; put the state expression only in bindings. Keep all labels and controls.".into(),
            severity: SelfCheckSeverity::Fatal,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration_self_check::{check_value_forest, IntentCheckMode};
    use serde_json::json;

    #[test]
    fn rejects_dynamic_colors_even_on_last_attempt() {
        for paint in [
            json!({"fill":[{"type":"solid","color":"$activeScene == 'home' ? '#22C55E' : '#1A1A1A'"}]}),
            json!({"stroke":{"fill":[{"type":"solid","color":"$app.color"}]}}),
            json!({"fill":[{"type":"linear_gradient","stops":[{"offset":0,"color":"$state.color"}]}]}),
        ] {
            let mut node = json!({"type":"frame","id":"chip"});
            node.as_object_mut()
                .unwrap()
                .extend(paint.as_object().unwrap().clone());
            let report = check_value_forest(&node, 375.0);
            assert!(report.rejects(IntentCheckMode::Advisory));
            assert!(report
                .failure_message()
                .contains("runtime-expression-in-static-paint"));
        }
    }

    #[test]
    fn preserves_valid_initial_paint_and_runtime_bindings() {
        let mut report = SelfCheckReport::default();
        check(
            &json!({"id":"chip","fill":[{"type":"solid","color":"$--card"}],
                "stroke":{"fill":[{"type":"solid","color":"#222222"}]},
                "bindings":{"fill":"$app.active ? '#22C55E' : '#222222'"},
                "children":[{"type":"text","content":"$state.count ? 'On' : 'Off'"}]
            }),
            &mut report,
        );
        assert!(report.issues.is_empty());
    }
}
