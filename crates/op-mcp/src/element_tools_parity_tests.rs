//! TS element-tool parity checks that compare Rust aliases against
//! concrete `pen-mcp` schema/semantic contracts.

use std::collections::BTreeMap;

use op_editor_core::{EditorCommand, EditorState};
use serde_json::Value;

use super::element_alias_builders::semantic_alias_node;
use super::element_tools::{
    element_tool_name, element_tool_schemas, insert_kit_component_tools, InsertKitComponent,
    TS_ELEMENT_ALIASES,
};
use super::{McpTool, ToolErrorCode, ToolOutcome};

fn alias_tool(name: &str) -> InsertKitComponent {
    insert_kit_component_tools(&EditorState::new())
        .into_iter()
        .find(|tool| tool.name() == name)
        .unwrap_or_else(|| panic!("missing element alias {name}"))
}

fn schema_value(name: &str) -> Value {
    let schema = element_tool_schemas(&EditorState::new())
        .into_iter()
        .find(|schema| schema.contains(&format!(r#""name":"{name}""#)))
        .unwrap_or_else(|| panic!("missing schema for {name}"));
    serde_json::from_str(&schema).expect("schema json")
}

fn assert_props(value: &Value, keys: &[&str]) {
    let props = value["inputSchema"]["properties"]
        .as_object()
        .expect("schema properties");
    for key in keys {
        assert!(props.contains_key(*key), "schema missing {key}: {value}");
    }
}

fn assert_exact_props(value: &Value, keys: &[&str]) {
    let props = value["inputSchema"]["properties"]
        .as_object()
        .expect("schema properties");
    let actual: std::collections::BTreeSet<_> = props.keys().map(String::as_str).collect();
    let expected: std::collections::BTreeSet<_> = keys.iter().copied().collect();
    assert_eq!(actual, expected, "schema property mismatch: {value}");
}

fn ts_production_element_tool_names() -> Vec<String> {
    let routes_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages/pen-mcp/src/routes");
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&routes_dir).expect("read TS routes dir") {
        let path = entry.expect("route entry").path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !file_name.starts_with("element-tool-defs") || !file_name.ends_with(".ts") {
            continue;
        }
        let src = std::fs::read_to_string(&path).expect("read TS element defs");
        let mut rest = src.as_str();
        while let Some(start) = rest.find("name: '") {
            rest = &rest[start + "name: '".len()..];
            let Some(end) = rest.find('\'') else {
                break;
            };
            let name = &rest[..end];
            if name.starts_with("add_") {
                out.push(name.to_string());
            }
            rest = &rest[end + 1..];
        }
    }
    out
}

#[test]
fn element_tool_names_sanitize_dashes() {
    assert_eq!(element_tool_name("btn-primary"), "insert_btn_primary");
    assert_eq!(element_tool_name("nav-bar"), "insert_nav_bar");
}

#[test]
fn canonical_insert_tool_emits_instantiate_command() {
    let tool = InsertKitComponent::new("openpencil-starter", "btn-primary");
    assert_eq!(tool.name(), "insert_btn_primary");
    let mut args = BTreeMap::new();
    args.insert("x".to_string(), "120".to_string());
    args.insert("y".to_string(), "80".to_string());

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            _,
            EditorCommand::InstantiateKitComponent { doc_x, doc_y, .. },
        ) => {
            assert_eq!(doc_x, Some(120.0));
            assert_eq!(doc_y, Some(80.0));
        }
        other => panic!("expected InstantiateKitComponent, got {other:?}"),
    }
}

#[test]
fn canonical_insert_tool_defaults_missing_position_and_rejects_bad_x() {
    let tool = InsertKitComponent::new("openpencil-starter", "badge");
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::OkWithCommand(
            _,
            EditorCommand::InstantiateKitComponent { doc_x, doc_y, .. },
        ) => {
            assert_eq!(doc_x, None);
            assert_eq!(doc_y, None);
        }
        other => panic!("expected InstantiateKitComponent, got {other:?}"),
    }

    let mut args = BTreeMap::new();
    args.insert("x".to_string(), "not-a-number".to_string());
    match tool.call(&args) {
        ToolOutcome::Err(ToolErrorCode::InvalidArgument, _) => {}
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn registry_covers_starter_kit_and_ts_production_aliases() {
    let state = EditorState::new();
    let tools = insert_kit_component_tools(&state);
    let schemas = element_tool_schemas(&state);
    assert_eq!(
        tools
            .iter()
            .filter(|tool| tool.name().starts_with("insert_"))
            .count(),
        37,
        "builtin kits ship 6 starter + 31 shadcn components"
    );
    assert_eq!(schemas.len(), tools.len(), "schema + tool counts agree");
    for tool in &tools {
        assert!(
            schemas
                .iter()
                .any(|schema| schema.contains(&format!("\"name\":\"{}\"", tool.name()))),
            "schema set must include {}",
            tool.name(),
        );
    }

    let mut ts_names = ts_production_element_tool_names();
    ts_names.sort();
    ts_names.dedup();
    assert!(
        ts_names.len() > 100,
        "TS production element catalog should remain broad"
    );
    let rust_names: std::collections::BTreeSet<_> =
        tools.iter().map(|tool| tool.name().to_string()).collect();
    let missing: Vec<_> = ts_names
        .into_iter()
        .filter(|name| !rust_names.contains(name))
        .collect();
    assert!(
        missing.is_empty(),
        "Rust MCP element registry is missing TS production tools: {missing:?}",
    );
}

/// Full porting parity: every TS element alias must now dispatch to a
/// real semantic builder, never the legacy kit-component placeholder.
/// `semantic_alias_node(alias, {})` returning `Ok(None)` would mean the
/// alias has no builder and `InsertKitComponent::call` would fall back to
/// `InstantiateKitComponent` — which TS `pen-mcp` never does (every TS
/// tool builds a real ElementTree). `Ok(Some)` (built with defaults) and
/// `Err` (a builder demanded a required arg) both prove the alias is
/// wired to a builder; only `Ok(None)` is a parity gap.
#[test]
fn every_ts_alias_dispatches_to_a_real_builder_not_kit_placeholder() {
    let unported: Vec<&str> = TS_ELEMENT_ALIASES
        .iter()
        .copied()
        .filter(|alias| matches!(semantic_alias_node(alias, &BTreeMap::new()), Ok(None)))
        .collect();
    assert!(
        unported.is_empty(),
        "these TS aliases still fall back to the kit placeholder (no real builder): {unported:?}",
    );
}

#[test]
fn text_aliases_emit_semantic_subtrees() {
    let mut heading_args = BTreeMap::new();
    heading_args.insert("content".to_string(), "Welcome".to_string());
    heading_args.insert("level".to_string(), "h1".to_string());
    match alias_tool("add_heading_v0").call(&heading_args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::InsertSubtree { nodes, .. }) => {
            let value = serde_json::to_value(&nodes[0]).expect("heading json");
            assert_eq!(value["type"], "text");
            assert_eq!(value["role"], "heading");
            assert_eq!(value["fontSize"].as_f64(), Some(32.0));
            assert_eq!(value["fontWeight"].as_f64(), Some(700.0));
        }
        other => panic!("expected semantic heading InsertSubtree, got {other:?}"),
    }

    let mut button_args = BTreeMap::new();
    button_args.insert("label".to_string(), "Continue".to_string());
    match alias_tool("add_text_button_v0").call(&button_args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::InsertSubtree { nodes, .. }) => {
            let value = serde_json::to_value(&nodes[0]).expect("button json");
            assert_eq!(value["role"], "button");
            assert_eq!(value["padding"], serde_json::json!([12.0, 20.0]));
            assert_eq!(value["children"][0]["content"], "Continue");
        }
        other => panic!("expected semantic text button InsertSubtree, got {other:?}"),
    }
}

#[test]
fn checkbox_radio_and_switch_aliases_emit_semantic_subtrees() {
    let mut checkbox_args = BTreeMap::new();
    checkbox_args.insert("label".to_string(), "Accept terms".to_string());
    checkbox_args.insert("checked".to_string(), "true".to_string());
    checkbox_args.insert("theme".to_string(), "system".to_string());
    match alias_tool("add_checkbox_v1").call(&checkbox_args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::InsertSubtree { nodes, .. }) => {
            let value = serde_json::to_value(&nodes[0]).expect("checkbox json");
            assert_eq!(value["role"], "checkbox-row");
            assert_eq!(value["children"][0]["role"], "checkbox-checked");
            assert_eq!(value["children"][0]["fill"][0]["color"], "$color-accent");
        }
        other => panic!("expected semantic checkbox InsertSubtree, got {other:?}"),
    }

    let mut radio_args = BTreeMap::new();
    radio_args.insert("label".to_string(), "Option A".to_string());
    radio_args.insert("theme".to_string(), "dark".to_string());
    match alias_tool("add_radio_v1").call(&radio_args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::InsertSubtree { nodes, .. }) => {
            let value = serde_json::to_value(&nodes[0]).expect("radio json");
            assert_eq!(value["role"], "radio-row");
            assert_eq!(value["children"][0]["role"], "radio");
            assert_eq!(
                value["children"][0]["stroke"]["fill"][0]["color"],
                "#334155"
            );
        }
        other => panic!("expected semantic radio InsertSubtree, got {other:?}"),
    }

    let mut switch_args = BTreeMap::new();
    switch_args.insert("active".to_string(), "true".to_string());
    match alias_tool("add_switch_v1").call(&switch_args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::InsertSubtree { nodes, .. }) => {
            let value = serde_json::to_value(&nodes[0]).expect("switch json");
            assert_eq!(value["role"], "switch");
            assert_eq!(value["justifyContent"], "end");
            assert_eq!(value["padding"].as_f64(), Some(2.0));
            assert_eq!(value["children"][0]["role"], "switch-thumb");
        }
        other => panic!("expected semantic switch InsertSubtree, got {other:?}"),
    }
}

#[test]
fn text_button_alias_schemas_advertise_ts_args() {
    let v0 = schema_value("add_text_button_v0");
    assert_props(
        &v0,
        &[
            "schemaVersion",
            "filePath",
            "label",
            "leading_icon",
            "parent_id",
            "pageId",
        ],
    );
    assert_eq!(v0["inputSchema"]["required"], serde_json::json!(["label"]));

    let v1 = schema_value("add_text_button_v1");
    assert_props(
        &v1,
        &[
            "schemaVersion",
            "filePath",
            "label",
            "leading_icon",
            "theme",
            "parent_id",
            "pageId",
        ],
    );
    assert_eq!(v1["inputSchema"]["required"], serde_json::json!(["label"]));
}

#[test]
fn form_field_alias_schemas_advertise_ts_args() {
    let v0 = schema_value("add_form_field_v0");
    assert_props(
        &v0,
        &[
            "schemaVersion",
            "filePath",
            "label",
            "placeholder",
            "leading_icon",
            "trailing_icon",
            "required",
            "parent_id",
            "pageId",
        ],
    );
    assert_eq!(v0["inputSchema"]["required"], serde_json::json!(["label"]));

    let v1 = schema_value("add_form_field_v1");
    assert_props(
        &v1,
        &[
            "schemaVersion",
            "filePath",
            "label",
            "placeholder",
            "leading_icon",
            "trailing_icon",
            "required",
            "theme",
            "parent_id",
            "pageId",
        ],
    );
    assert_eq!(v1["inputSchema"]["required"], serde_json::json!(["label"]));
}

#[test]
fn segmented_control_alias_schemas_advertise_ts_args() {
    let v0 = schema_value("add_segmented_control_v0");
    assert_props(
        &v0,
        &["schemaVersion", "filePath", "items", "parent_id", "pageId"],
    );
    assert_eq!(v0["inputSchema"]["required"], serde_json::json!(["items"]));

    let v1 = schema_value("add_segmented_control_v1");
    assert_props(
        &v1,
        &[
            "schemaVersion",
            "filePath",
            "items",
            "theme",
            "parent_id",
            "pageId",
        ],
    );
    assert_eq!(v1["inputSchema"]["required"], serde_json::json!(["items"]));
}

#[test]
fn checkbox_radio_and_switch_alias_schemas_advertise_ts_args() {
    let checkbox = schema_value("add_checkbox_v1");
    assert_props(
        &checkbox,
        &[
            "schemaVersion",
            "filePath",
            "label",
            "checked",
            "theme",
            "parent_id",
            "pageId",
        ],
    );
    assert_eq!(
        checkbox["inputSchema"]["required"],
        serde_json::json!(["label"])
    );

    let radio = schema_value("add_radio_v1");
    assert_props(
        &radio,
        &[
            "schemaVersion",
            "filePath",
            "label",
            "selected",
            "theme",
            "parent_id",
            "pageId",
        ],
    );
    assert_eq!(
        radio["inputSchema"]["required"],
        serde_json::json!(["label"])
    );

    let switch = schema_value("add_switch_v1");
    assert_props(
        &switch,
        &[
            "schemaVersion",
            "filePath",
            "active",
            "theme",
            "parent_id",
            "pageId",
        ],
    );
    assert_eq!(switch["inputSchema"]["required"], serde_json::json!([]));
}

#[test]
fn generic_alias_schemas_advertise_ts_business_args_instead_of_xy_fallback() {
    let pricing = schema_value("add_pricing_card_v1");
    assert_exact_props(
        &pricing,
        &[
            "schemaVersion",
            "filePath",
            "tier",
            "price",
            "currency",
            "period",
            "features",
            "description",
            "badge",
            "cta",
            "emphasis",
            "width",
            "corner_radius",
            "theme",
            "parent_id",
            "pageId",
        ],
    );
    assert_eq!(
        pricing["inputSchema"]["required"],
        serde_json::json!(["tier", "price"])
    );

    let action_menu = schema_value("add_action_menu_v0");
    assert_exact_props(
        &action_menu,
        &[
            "schemaVersion",
            "filePath",
            "items",
            "width",
            "parent_id",
            "pageId",
        ],
    );
    assert_eq!(
        action_menu["inputSchema"]["required"],
        serde_json::json!(["items"])
    );

    let chart = schema_value("add_chart_line_v1");
    assert_exact_props(
        &chart,
        &[
            "schemaVersion",
            "filePath",
            "values",
            "point_spacing",
            "chart_height",
            "dots",
            "stroke_color",
            "theme",
            "parent_id",
            "pageId",
        ],
    );
    assert_eq!(
        chart["inputSchema"]["required"],
        serde_json::json!(["values"])
    );
}

#[test]
fn segmented_control_v1_alias_emits_theme_aware_semantic_subtree() {
    let mut args = BTreeMap::new();
    args.insert(
        "items".to_string(),
        r#"[{"label":"Day"},{"label":"Week","active":true},{"label":"Month"}]"#.to_string(),
    );
    args.insert("theme".to_string(), "dark".to_string());

    match alias_tool("add_segmented_control_v1").call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::InsertSubtree { nodes, .. }) => {
            assert_eq!(nodes.len(), 1);
            let value = serde_json::to_value(&nodes[0]).expect("segmented control json");
            assert_eq!(value["role"], "segmented-control");
            assert_eq!(value["width"], "fill_container");
            assert_eq!(value["height"].as_f64(), Some(32.0));
            assert_eq!(value["cornerRadius"].as_f64(), Some(8.0));
            assert_eq!(value["fill"][0]["color"], "#334155");
            assert_eq!(value["padding"].as_f64(), Some(4.0));

            let segments = value["children"].as_array().expect("segments");
            assert_eq!(segments.len(), 3);
            assert_eq!(segments[0]["role"], "segment");
            assert_eq!(segments[1]["role"], "segment-active");
            assert_eq!(segments[2]["role"], "segment");
            for segment in segments {
                assert_eq!(segment["width"], "fill_container");
                assert_eq!(segment["height"], "fill_container");
            }
            assert_eq!(segments[0]["fill"], serde_json::json!([]));
            assert_eq!(segments[1]["fill"][0]["color"], "#1E293B");
            assert_eq!(segments[1]["children"][0]["content"], "Week");
            assert_eq!(
                segments[1]["children"][0]["fontWeight"].as_f64(),
                Some(600.0)
            );
            assert_eq!(segments[1]["children"][0]["fill"][0]["color"], "#F1F5F9");
            assert_eq!(segments[2]["children"][0]["fill"][0]["color"], "#94A3B8");
        }
        other => panic!("expected semantic segmented control InsertSubtree, got {other:?}"),
    }
}

#[test]
fn action_menu_chart_line_and_pricing_card_aliases_emit_semantic_subtrees() {
    let mut menu_args = BTreeMap::new();
    menu_args.insert(
        "items".to_string(),
        r#"[{"label":"Archive","icon":"archive"},{"label":"Delete","icon":"trash","destructive":true,"divider_before":true}]"#
            .to_string(),
    );
    menu_args.insert("width".to_string(), "180".to_string());
    menu_args.insert("theme".to_string(), "dark".to_string());
    match alias_tool("add_action_menu_v1").call(&menu_args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::InsertSubtree { nodes, .. }) => {
            let value = serde_json::to_value(&nodes[0]).expect("action menu json");
            assert_eq!(value["role"], "action-menu");
            assert_eq!(value["width"].as_f64(), Some(180.0));
            assert_eq!(value["fill"][0]["color"], "#1E293B");
            assert_eq!(value["children"][1]["role"], "action-menu-divider");
            assert_eq!(value["children"][2]["role"], "action-menu-item-destructive");
            assert_eq!(
                value["children"][2]["children"][1]["fill"][0]["color"],
                "#F87171"
            );
        }
        other => panic!("expected semantic action menu InsertSubtree, got {other:?}"),
    }

    let mut chart_args = BTreeMap::new();
    chart_args.insert("values".to_string(), "[10,20,0]".to_string());
    chart_args.insert("point_spacing".to_string(), "16".to_string());
    chart_args.insert("chart_height".to_string(), "80".to_string());
    chart_args.insert("theme".to_string(), "system".to_string());
    match alias_tool("add_chart_line_v1").call(&chart_args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::InsertSubtree { nodes, .. }) => {
            let value = serde_json::to_value(&nodes[0]).expect("chart line json");
            assert_eq!(value["role"], "chart-line");
            assert_eq!(value["width"].as_f64(), Some(48.0));
            assert_eq!(value["height"].as_f64(), Some(80.0));
            assert_eq!(value["children"][0]["role"], "chart-line-path");
            assert_eq!(
                value["children"][0]["stroke"]["fill"][0]["color"],
                "$color-chart-1"
            );
            assert_eq!(value["children"][1]["role"], "chart-line-dot");
        }
        other => panic!("expected semantic chart line InsertSubtree, got {other:?}"),
    }

    let mut pricing_args = BTreeMap::new();
    pricing_args.insert("tier".to_string(), "Pro".to_string());
    pricing_args.insert("price".to_string(), "29".to_string());
    pricing_args.insert("period".to_string(), "/mo".to_string());
    pricing_args.insert(
        "features".to_string(),
        r#"["Projects","Exports"]"#.to_string(),
    );
    pricing_args.insert("badge".to_string(), "Popular".to_string());
    pricing_args.insert("emphasis".to_string(), "featured".to_string());
    pricing_args.insert("theme".to_string(), "dark".to_string());
    match alias_tool("add_pricing_card_v1").call(&pricing_args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::InsertSubtree { nodes, .. }) => {
            let value = serde_json::to_value(&nodes[0]).expect("pricing card json");
            assert_eq!(value["role"], "pricing-card");
            assert_eq!(value["fill"][0]["color"], "#1E293B");
            assert_eq!(value["stroke"]["fill"][0]["color"], "#2563EB");
            assert_eq!(value["children"][0]["role"], "pricing-badge");
            assert_eq!(value["children"][1]["role"], "pricing-header");
            assert_eq!(value["children"][2]["role"], "pricing-price-row");
            assert_eq!(value["children"][3]["role"], "pricing-features");
            assert_eq!(value["children"][4]["role"], "pricing-cta");
            assert_eq!(
                value["children"][4]["children"][0]["content"],
                "Get started"
            );
        }
        other => panic!("expected semantic pricing card InsertSubtree, got {other:?}"),
    }
}

#[test]
fn basic_visual_aliases_emit_semantic_subtrees() {
    let mut badge_args = BTreeMap::new();
    badge_args.insert("label".to_string(), "BETA".to_string());
    match alias_tool("add_badge_v1").call(&badge_args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::InsertSubtree { nodes, .. }) => {
            let value = serde_json::to_value(&nodes[0]).expect("badge json");
            assert_eq!(value["role"], "badge");
            assert_eq!(value["padding"], serde_json::json!([4.0, 10.0]));
            assert_eq!(value["children"][0]["content"], "BETA");
        }
        other => panic!("expected semantic badge InsertSubtree, got {other:?}"),
    }

    let mut status_args = BTreeMap::new();
    status_args.insert("label".to_string(), "Online".to_string());
    status_args.insert("tone".to_string(), "success".to_string());
    match alias_tool("add_status_badge_v1").call(&status_args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::InsertSubtree { nodes, .. }) => {
            let value = serde_json::to_value(&nodes[0]).expect("status badge json");
            assert_eq!(value["role"], "status-badge");
            assert_eq!(value["children"][0]["role"], "status-dot");
            assert_eq!(value["children"][0]["fill"][0]["color"], "#10B981");
            assert_eq!(value["children"][1]["content"], "Online");
        }
        other => panic!("expected semantic status badge InsertSubtree, got {other:?}"),
    }

    let mut progress_args = BTreeMap::new();
    progress_args.insert("value".to_string(), "25".to_string());
    progress_args.insert("bar_width".to_string(), "200".to_string());
    progress_args.insert("theme".to_string(), "dark".to_string());
    match alias_tool("add_progress_bar_v1").call(&progress_args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::InsertSubtree { nodes, .. }) => {
            let value = serde_json::to_value(&nodes[0]).expect("progress json");
            assert_eq!(value["role"], "progress-bar");
            assert_eq!(value["width"].as_f64(), Some(200.0));
            assert_eq!(value["fill"][0]["color"], "#334155");
            assert_eq!(value["children"][0]["role"], "progress-bar-fill");
            assert_eq!(value["children"][0]["width"].as_f64(), Some(50.0));
        }
        other => panic!("expected semantic progress bar InsertSubtree, got {other:?}"),
    }

    let mut rating_args = BTreeMap::new();
    rating_args.insert("filled".to_string(), "3".to_string());
    rating_args.insert("total".to_string(), "5".to_string());
    rating_args.insert("size".to_string(), "18".to_string());
    match alias_tool("add_rating_stars_v1").call(&rating_args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::InsertSubtree { nodes, .. }) => {
            let value = serde_json::to_value(&nodes[0]).expect("rating json");
            assert_eq!(value["role"], "rating-stars");
            assert_eq!(value["children"].as_array().expect("stars").len(), 5);
            assert_eq!(value["children"][0]["role"], "star-filled");
            assert_eq!(value["children"][3]["role"], "star-empty");
            assert_eq!(value["children"][0]["width"].as_f64(), Some(18.0));
        }
        other => panic!("expected semantic rating stars InsertSubtree, got {other:?}"),
    }

    let mut fab_args = BTreeMap::new();
    fab_args.insert("icon".to_string(), "plus".to_string());
    fab_args.insert("size".to_string(), "64".to_string());
    fab_args.insert("theme".to_string(), "system".to_string());
    match alias_tool("add_fab_v1").call(&fab_args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::InsertSubtree { nodes, .. }) => {
            let value = serde_json::to_value(&nodes[0]).expect("fab json");
            assert_eq!(value["role"], "fab");
            assert_eq!(value["cornerRadius"].as_f64(), Some(32.0));
            assert_eq!(value["fill"][0]["color"], "$color-accent");
            assert_eq!(value["children"][0]["iconFontName"], "plus");
        }
        other => panic!("expected semantic fab InsertSubtree, got {other:?}"),
    }

    let mut link_args = BTreeMap::new();
    link_args.insert("label".to_string(), "Learn more".to_string());
    link_args.insert("trailing_icon".to_string(), "arrow-right".to_string());
    match alias_tool("add_link_v1").call(&link_args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::InsertSubtree { nodes, .. }) => {
            let value = serde_json::to_value(&nodes[0]).expect("link json");
            assert_eq!(value["role"], "link");
            assert_eq!(value["children"][0]["role"], "link-label");
            assert_eq!(value["children"][1]["role"], "link-icon");
        }
        other => panic!("expected semantic link InsertSubtree, got {other:?}"),
    }

    let mut kbd_args = BTreeMap::new();
    kbd_args.insert("keys".to_string(), r#"["Cmd","K"]"#.to_string());
    kbd_args.insert("separator".to_string(), "+".to_string());
    kbd_args.insert("theme".to_string(), "system".to_string());
    match alias_tool("add_kbd_v1").call(&kbd_args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::InsertSubtree { nodes, .. }) => {
            let value = serde_json::to_value(&nodes[0]).expect("kbd json");
            assert_eq!(value["role"], "kbd");
            assert_eq!(value["children"][0]["role"], "kbd-key");
            assert_eq!(value["children"][0]["fill"][0]["color"], "$color-surface-2");
            assert_eq!(
                value["children"][0]["stroke"]["fill"][0]["color"],
                "$color-border"
            );
            assert_eq!(value["children"][1]["role"], "kbd-separator");
        }
        other => panic!("expected semantic kbd InsertSubtree, got {other:?}"),
    }
}
