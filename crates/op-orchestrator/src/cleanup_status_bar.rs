//! Status bar identification helpers shared by cleanup and tree-heuristics passes.

use jian_ops_schema::node::PenNode;
use op_editor_core::PenNodeExt;

/// The alias set both entry points match against. Kept in one place because
/// the two callers read from different representations (typed tree vs. raw
/// JSON) and a second copy of this list would drift the moment a new alias
/// is added on one side only.
fn matches_status_bar_identity(role: Option<&str>, id: &str, name: &str) -> bool {
    if role.is_some_and(|role| role.eq_ignore_ascii_case("status-bar")) {
        return true;
    }
    let hay = format!("{} {}", id.to_lowercase(), name.to_lowercase());
    hay.contains("status bar")
        || hay.contains("status-bar")
        || hay.contains("statusbar")
        || hay.contains("状态栏")
        || hay.contains("系统栏")
}

/// Explicit status-bar role or an English/Chinese name/id match identifies
/// status-bar chrome. The role path keeps generated/custom-named bars stable;
/// Chinese aliases cover direct local edits such as "顶部状态栏".
pub(crate) fn is_status_bar(node: &PenNode) -> bool {
    matches_status_bar_identity(
        node.base().role.as_deref(),
        node.id_str(),
        node.base().name.as_deref().unwrap_or(""),
    )
}

/// JSON variant for tree-heuristics passes, which run on `serde_json` values
/// before deserialization.
pub(crate) fn is_status_bar_from_json(node: &serde_json::Value) -> bool {
    let field = |key: &str| node.get(key).and_then(serde_json::Value::as_str);
    matches_status_bar_identity(
        field("role"),
        field("id").unwrap_or(""),
        field("name").unwrap_or(""),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The two entry points must agree on every alias — that agreement is the
    /// reason the matcher is shared, so it is asserted rather than assumed.
    #[test]
    fn typed_and_json_predicates_agree_on_every_alias() {
        let cases = [
            (
                json!({"type":"frame","role":"status-bar","id":"n1","name":"Bar"}),
                true,
            ),
            (json!({"type":"frame","id":"n2","name":"Status Bar"}), true),
            (
                json!({"type":"frame","id":"status-bar-1","name":"Chrome"}),
                true,
            ),
            (json!({"type":"frame","id":"n3","name":"statusbar"}), true),
            (json!({"type":"frame","id":"n4","name":"顶部状态栏"}), true),
            (json!({"type":"frame","id":"n5","name":"系统栏"}), true),
            (json!({"type":"frame","id":"n6","name":"Header"}), false),
            (
                json!({"type":"frame","role":"bottom-tab-bar","id":"n7","name":"Nav"}),
                false,
            ),
        ];
        for (value, expected) in cases {
            assert_eq!(
                is_status_bar_from_json(&value),
                expected,
                "json predicate disagreed on {value}"
            );
            let node: PenNode =
                serde_json::from_value(value.clone()).expect("fixture parses as a node");
            assert_eq!(
                is_status_bar(&node),
                expected,
                "typed predicate disagreed on {value}"
            );
        }
    }
}
