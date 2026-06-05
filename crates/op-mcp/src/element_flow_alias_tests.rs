//! TS flow/navigation element alias parity checks.

use std::collections::BTreeMap;

use op_editor_core::{EditorCommand, EditorState};
use serde_json::Value;

use super::element_tools::{insert_kit_component_tools, InsertKitComponent};
use super::{McpTool, ToolOutcome};

fn alias_tool(name: &str) -> InsertKitComponent {
    insert_kit_component_tools(&EditorState::new())
        .into_iter()
        .find(|tool| tool.name() == name)
        .unwrap_or_else(|| panic!("missing element alias {name}"))
}

fn semantic_value(tool: &str, args: BTreeMap<String, String>) -> Value {
    match alias_tool(tool).call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::InsertSubtree { nodes, .. }) => {
            serde_json::to_value(&nodes[0]).expect("semantic node json")
        }
        other => panic!("expected semantic {tool} InsertSubtree, got {other:?}"),
    }
}

fn args<const N: usize>(pairs: [(&str, &str); N]) -> BTreeMap<String, String> {
    pairs
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn flow_navigation_aliases_emit_ts_builder_semantic_subtrees() {
    let activity = semantic_value(
        "add_activity_log_v1",
        args([
            ("actor", "Sarah Lee"),
            ("action", "approved deploy"),
            ("timestamp", "2h ago"),
            ("icon", "check"),
            ("tone", "success"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(activity["role"], "activity-log");
    assert_eq!(activity["width"], "fill_container");
    assert_eq!(activity["padding"], serde_json::json!([10.0, 0.0]));
    assert_eq!(activity["children"][0]["role"], "activity-log-icon-dot");
    assert_eq!(
        activity["children"][0]["fill"][0]["color"],
        "$color-success-bg"
    );
    assert_eq!(
        activity["children"][0]["children"][0]["fill"][0]["color"],
        "$color-success-text"
    );
    assert_eq!(activity["children"][1]["role"], "activity-log-body");
    assert_eq!(
        activity["children"][1]["children"][0]["content"][0]["text"],
        "Sarah Lee"
    );
    assert_eq!(
        activity["children"][1]["children"][0]["content"][0]["fill"],
        "$color-text-primary"
    );
    assert_eq!(activity["children"][2]["content"], "2h ago");
    assert_eq!(
        activity["children"][2]["fill"][0]["color"],
        "$color-text-subtle"
    );

    let attachment = semantic_value(
        "add_attachment_row_v1",
        args([
            ("filename", "receipt.pdf"),
            ("size", "240 KB"),
            ("icon", "file-text"),
            ("removable", "false"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(attachment["role"], "attachment-row");
    assert_eq!(attachment["fill"][0]["color"], "$color-bg-deep");
    assert_eq!(attachment["children"][0]["role"], "attachment-icon");
    assert_eq!(
        attachment["children"][0]["fill"][0]["color"],
        "$color-text-muted"
    );
    assert_eq!(attachment["children"][1]["role"], "attachment-meta");
    assert_eq!(
        attachment["children"][1]["children"][0]["fill"][0]["color"],
        "$color-text-primary"
    );
    assert_eq!(
        attachment["children"][1]["children"][1]["fill"][0]["color"],
        "$color-text-muted"
    );
    assert_eq!(
        attachment["children"]
            .as_array()
            .expect("attachment children")
            .len(),
        2
    );

    let avatars = semantic_value(
        "add_avatar_group_v1",
        args([
            (
                "items",
                r##"[{"initial":"A"},{"initial":"B","color":"#111111"},{"initial":"C"}]"##,
            ),
            ("max_visible", "2"),
            ("size", "36"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(avatars["role"], "avatar-group");
    assert_eq!(avatars["children"].as_array().expect("avatars").len(), 3);
    assert_eq!(avatars["children"][0]["role"], "avatar-group-item");
    assert_eq!(
        avatars["children"][0]["stroke"]["fill"][0]["color"],
        "$color-surface"
    );
    assert_eq!(
        avatars["children"][0]["children"][0]["fill"][0]["color"],
        "$color-surface"
    );
    assert_eq!(avatars["children"][1]["fill"][0]["color"], "#111111");
    assert_eq!(avatars["children"][2]["role"], "avatar-group-overflow");
    assert_eq!(
        avatars["children"][2]["fill"][0]["color"],
        "$color-surface-2"
    );
    assert_eq!(avatars["children"][2]["children"][0]["content"], "+1");

    let bottom_nav = semantic_value(
        "add_bottom_nav_v1",
        args([
            (
                "items",
                r#"[{"title":"Home","icon":"house","active":true},{"title":"Cart","icon":"shopping-bag"}]"#,
            ),
            ("height", "70"),
            ("theme", "dark"),
        ]),
    );
    assert_eq!(bottom_nav["role"], "bottom-tab-bar");
    assert_eq!(bottom_nav["height"].as_f64(), Some(70.0));
    assert_eq!(bottom_nav["children"][0]["role"], "nav-item-active");
    assert_eq!(bottom_nav["children"][0]["children"][1]["fontWeight"], 600);
    assert_eq!(bottom_nav["children"][1]["role"], "nav-item");
    assert_eq!(
        bottom_nav["children"][1]["children"][0]["iconFontName"],
        "shopping-cart"
    );

    let breadcrumb = semantic_value(
        "add_breadcrumb_v1",
        args([
            (
                "items",
                r#"[{"label":"Home"},{"label":"Settings"},{"label":"Billing"}]"#,
            ),
            ("theme", "system"),
        ]),
    );
    assert_eq!(breadcrumb["role"], "breadcrumb");
    assert_eq!(breadcrumb["children"].as_array().expect("crumbs").len(), 5);
    assert_eq!(breadcrumb["children"][0]["role"], "breadcrumb-item");
    assert_eq!(breadcrumb["children"][1]["role"], "breadcrumb-separator");
    assert_eq!(breadcrumb["children"][4]["role"], "breadcrumb-item-active");
    assert_eq!(breadcrumb["children"][4]["fontWeight"], 600);

    let calendar = semantic_value(
        "add_calendar_grid_v1",
        args([
            ("days_in_month", "10"),
            ("start_day_offset", "2"),
            ("today", "4"),
            ("selected_day", "6"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(calendar["role"], "calendar-grid");
    assert_eq!(calendar["children"][0]["role"], "calendar-header-row");
    assert_eq!(
        calendar["children"][0]["children"][0]["children"][0]["fill"][0]["color"],
        "$color-text-muted"
    );
    assert_eq!(
        calendar["children"][1]["children"][0]["role"],
        "calendar-day-empty"
    );
    assert_eq!(
        calendar["children"][1]["children"][2]["role"],
        "calendar-day"
    );
    assert_eq!(
        calendar["children"][1]["children"][5]["role"],
        "calendar-day-today"
    );
    assert_eq!(
        calendar["children"][1]["children"][5]["fill"][0]["color"],
        "$color-info-bg"
    );
    assert_eq!(
        calendar["children"][2]["children"][0]["role"],
        "calendar-day-selected"
    );
    assert_eq!(
        calendar["children"][2]["children"][0]["fill"][0]["color"],
        "$color-accent"
    );
    assert_eq!(
        calendar["children"][2]["children"][0]["children"][0]["fill"][0]["color"],
        "$color-surface"
    );
}
