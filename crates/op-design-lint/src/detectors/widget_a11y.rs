//! Widget accessibility detector (Phase E5). Flags writable text inputs
//! (`text_input` / `text_area`) that carry NEITHER a `placeholder` NOR a
//! `semantics.label` — an input with no visible or programmatic label is an
//! a11y problem (screen readers announce it as an unnamed edit field).

use jian_ops_schema::node::PenNode;
use jian_ops_schema::semantics::SemanticsMeta;

use crate::issue::{FixProperty, Issue, IssueCategory, IssueSeverity};
use crate::node_util::{children, node_id};

/// Flags every `text_input` / `text_area` whose `placeholder` is missing or
/// empty AND whose `semantics.label` is missing or empty. A non-empty value
/// in either field clears the warning — one accessible name is enough.
pub fn detect_unlabeled_inputs(root: &PenNode) -> Vec<Issue> {
    let mut issues = Vec::new();
    walk_unlabeled_inputs(root, &mut issues);
    issues
}

fn walk_unlabeled_inputs(node: &PenNode, issues: &mut Vec<Issue>) {
    // Only the two writable text-entry kinds carry both `placeholder` and
    // `semantics` in a way that makes "unlabeled" meaningful.
    let fields = match node {
        PenNode::TextInput(n) => Some((n.placeholder.as_deref(), n.semantics.as_ref())),
        PenNode::TextArea(n) => Some((n.placeholder.as_deref(), n.semantics.as_ref())),
        _ => None,
    };
    if let Some((placeholder, semantics)) = fields {
        if !has_placeholder(placeholder) && !has_label(semantics) {
            issues.push(Issue {
                node_id: node_id(node).to_string(),
                category: IssueCategory::WidgetA11y,
                severity: IssueSeverity::Warning,
                property: FixProperty::Label,
                current_value: serde_json::Value::Null,
                suggested_value: serde_json::Value::Null,
                reason: "input has no placeholder or accessible label".to_string(),
            });
        }
    }
    for child in children(node) {
        walk_unlabeled_inputs(child, issues);
    }
}

/// True when a placeholder string is present and not empty (whitespace-only
/// counts as empty — it provides no accessible name).
fn has_placeholder(placeholder: Option<&str>) -> bool {
    placeholder.is_some_and(|p| !p.trim().is_empty())
}

/// True when `semantics.label` is present and not empty.
fn has_label(semantics: Option<&SemanticsMeta>) -> bool {
    semantics.is_some_and(|s| s.label.as_deref().is_some_and(|l| !l.trim().is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Deserialize a JSON value into a `PenNode` — doubles as a schema
    /// round-trip check for every fixture.
    fn node(value: serde_json::Value) -> PenNode {
        serde_json::from_value(value).expect("fixture must deserialize as PenNode")
    }

    #[test]
    fn flags_text_input_with_neither_placeholder_nor_label() {
        let root = node(json!({"type": "text_input", "id": "i1"}));
        let issues = detect_unlabeled_inputs(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "i1");
        assert_eq!(issues[0].category, IssueCategory::WidgetA11y);
        assert_eq!(issues[0].severity, IssueSeverity::Warning);
        assert_eq!(issues[0].property, FixProperty::Label);
        assert_eq!(
            issues[0].reason,
            "input has no placeholder or accessible label"
        );
    }

    #[test]
    fn ignores_text_input_with_placeholder() {
        let root = node(json!({
            "type": "text_input", "id": "i1", "placeholder": "Search…"
        }));
        assert!(detect_unlabeled_inputs(&root).is_empty());
    }

    #[test]
    fn ignores_text_input_with_semantics_label() {
        let root = node(json!({
            "type": "text_input", "id": "i1",
            "semantics": {"label": "Email address"}
        }));
        assert!(detect_unlabeled_inputs(&root).is_empty());
    }

    #[test]
    fn flags_text_area_with_neither() {
        let root = node(json!({"type": "text_area", "id": "ta1"}));
        let issues = detect_unlabeled_inputs(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "ta1");
        assert_eq!(issues[0].category, IssueCategory::WidgetA11y);
    }

    #[test]
    fn treats_empty_and_whitespace_placeholder_as_unlabeled() {
        let empty = node(json!({"type": "text_input", "id": "i1", "placeholder": ""}));
        assert_eq!(detect_unlabeled_inputs(&empty).len(), 1);

        let blank = node(json!({"type": "text_input", "id": "i2", "placeholder": "   "}));
        assert_eq!(detect_unlabeled_inputs(&blank).len(), 1);
    }

    #[test]
    fn ignores_non_input_kinds() {
        let text = node(json!({"type": "text", "id": "t1", "content": "hi"}));
        assert!(detect_unlabeled_inputs(&text).is_empty());

        let frame = node(json!({"type": "frame", "id": "f1"}));
        assert!(detect_unlabeled_inputs(&frame).is_empty());
    }

    #[test]
    fn walks_nested_inputs() {
        let root = node(json!({
            "type": "frame", "id": "root",
            "children": [
                {"type": "frame", "id": "inner", "children": [
                    {"type": "text_input", "id": "deep"}
                ]}
            ]
        }));
        let issues = detect_unlabeled_inputs(&root);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].node_id, "deep");
    }
}
