use crate::design_form::DesignForm;
use crate::issue::{FixProperty, Issue, IssueCategory, IssueSeverity};
use crate::node_util::{children, node_id};
use jian_ops_schema::motion::P1_MOTION_PROPERTIES;
use jian_ops_schema::node::PenNode;
use serde_json::json;

pub fn detect_motion_budget(root: &PenNode, _form: DesignForm) -> Vec<Issue> {
    let mut nodes = Vec::new();
    let mut issues = Vec::new();
    collect(root, &mut nodes, &mut issues);
    if nodes.len() > 32 {
        let node_id = nodes[32].to_owned();
        issues.push(issue(
            &node_id,
            nodes.len(),
            format!(
                "this root carries {} nodes with animations; the runtime budget is 32",
                nodes.len()
            ),
        ));
    }
    issues
}

fn collect(node: &PenNode, nodes: &mut Vec<String>, issues: &mut Vec<Issue>) {
    let (transition, animations) = node.motion_declarations();
    if let Some(animations) = animations {
        nodes.push(node_id(node).to_owned());
        if animations
            .iter()
            .any(|animation| animation.duration_ms > 1_500)
        {
            issues.push(issue(
                node_id(node),
                1_500,
                "animation duration exceeds the 1500 ms M3 ceiling".to_owned(),
            ));
        }
        for animation in animations {
            for keyframe in &animation.keyframes {
                if let Some(property) = keyframe
                    .values
                    .keys()
                    .find(|property| !P1_MOTION_PROPERTIES.contains(&property.as_str()))
                {
                    issues.push(issue(
                        node_id(node),
                        json!(property),
                        format!(
                            "motion property `{property}` is not in the P1 compositor whitelist"
                        ),
                    ));
                    break;
                }
            }
        }
    }
    if let Some(transition) = transition {
        if let Some(property) = transition
            .properties
            .as_ref()
            .into_iter()
            .flatten()
            .find(|property| !P1_MOTION_PROPERTIES.contains(&property.as_str()))
        {
            issues.push(issue(
                node_id(node),
                json!(property),
                format!("motion property `{property}` is not in the P1 compositor whitelist"),
            ));
        }
    }
    for child in children(node) {
        collect(child, nodes, issues);
    }
}

fn issue(node_id: &str, current: impl serde::Serialize, reason: String) -> Issue {
    Issue {
        node_id: node_id.to_owned(),
        category: IssueCategory::MotionBudget,
        severity: IssueSeverity::Warning,
        property: FixProperty::None,
        current_value: serde_json::to_value(current).unwrap_or(serde_json::Value::Null),
        suggested_value: serde_json::Value::Null,
        reason,
    }
}

#[cfg(test)]
mod tests;
