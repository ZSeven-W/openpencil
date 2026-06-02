//! UIKit element tools — one `insert_<comp>` MCP tool per built-in
//! [`UIKit`] component, so an LLM client can drop a Primary Button
//! (etc.) onto the canvas without first having to learn the kit-id
//! / component-id pair. The TS counterpart is `pen-mcp`'s ~100
//! `add_card_v0` / `add_toast_v0` element tools.
//!
//! Each tool returns `ToolOutcome::OkWithCommand(map,
//! EditorCommand::InstantiateKitComponent { … })` so the host's
//! applier runs the same `EditorState::instantiate_kit_component`
//! path the Component-Browser panel uses — deep-clone, fresh-id,
//! subtree-translate, select, history-snapshot.

use std::collections::BTreeMap;

use op_editor_core::{EditorCommand, EditorState, NodeId, UIKit};

use super::{McpTool, ToolErrorCode, ToolOutcome};
use crate::element_alias_builders::semantic_alias_node;

/// Sanitize a `kit-id` / `component-id` for embedding in a tool name
/// — MCP tool names are `[a-zA-Z0-9_]+`, so dashes become underscores.
fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// The MCP tool name for a kit component: `insert_<comp_sanitized>`.
/// v1 ships one starter kit so the kit prefix is dropped for terseness;
/// a future imported-kits surface can fold the kit id back in once
/// collisions are possible.
pub fn element_tool_name(component_id: &str) -> String {
    format!("insert_{}", sanitize(component_id))
}

const STARTER_KIT_ID: &str = "openpencil-starter";

/// Exact-name aliases for the TS `pen-mcp` production element tools.
/// Rust does not yet ship TS's full per-tool templates, so aliases
/// instantiate the closest built-in starter-kit component instead of
/// failing with `UnknownTool`.
const TS_ELEMENT_ALIASES: &[&str] = &[
    "add_action_menu_v0",
    "add_action_menu_v1",
    "add_activity_log_v0",
    "add_activity_log_v1",
    "add_activity_ring_v0",
    "add_activity_ring_v1",
    "add_alert_v0",
    "add_alert_v1",
    "add_attachment_row_v0",
    "add_attachment_row_v1",
    "add_avatar_group_v0",
    "add_avatar_group_v1",
    "add_avatar_v0",
    "add_avatar_v1",
    "add_badge_v0",
    "add_badge_v1",
    "add_body_text_v0",
    "add_body_text_v1",
    "add_bottom_nav_v0",
    "add_bottom_nav_v1",
    "add_breadcrumb_v0",
    "add_breadcrumb_v1",
    "add_calendar_grid_v0",
    "add_calendar_grid_v1",
    "add_callout_v0",
    "add_callout_v1",
    "add_card_row_v0",
    "add_card_row_v1",
    "add_carousel_dots_v0",
    "add_carousel_dots_v1",
    "add_chart_bars_v0",
    "add_chart_bars_v1",
    "add_chart_line_v0",
    "add_chart_line_v1",
    "add_chart_pie_v0",
    "add_chart_pie_v1",
    "add_chat_bubble_v0",
    "add_chat_bubble_v1",
    "add_checkbox_v0",
    "add_checkbox_v1",
    "add_chip_input_v0",
    "add_chip_input_v1",
    "add_code_block_v0",
    "add_code_block_v1",
    "add_color_swatch_v0",
    "add_color_swatch_v1",
    "add_combobox_v0",
    "add_combobox_v1",
    "add_comment_v0",
    "add_comment_v1",
    "add_cookie_banner_v0",
    "add_cookie_banner_v1",
    "add_data_table_row_v0",
    "add_data_table_row_v1",
    "add_date_picker_v0",
    "add_date_picker_v1",
    "add_divider_v0",
    "add_divider_v1",
    "add_drawer_shell_v0",
    "add_drawer_shell_v1",
    "add_empty_chart_v0",
    "add_empty_chart_v1",
    "add_empty_state_v0",
    "add_empty_state_v1",
    "add_event_card_v0",
    "add_event_card_v1",
    "add_fab_v0",
    "add_fab_v1",
    "add_faq_item_v0",
    "add_faq_item_v1",
    "add_filter_group_v0",
    "add_filter_group_v1",
    "add_form_field_v0",
    "add_form_field_v1",
    "add_heading_v0",
    "add_heading_v1",
    "add_icon_button_v0",
    "add_icon_button_v1",
    "add_icon_label_v0",
    "add_icon_label_v1",
    "add_image_placeholder_v0",
    "add_image_placeholder_v1",
    "add_inbox_message_v0",
    "add_inbox_message_v1",
    "add_inline_action_v0",
    "add_inline_action_v1",
    "add_input_with_action_v0",
    "add_input_with_action_v1",
    "add_invite_row_v0",
    "add_invite_row_v1",
    "add_kbd_v0",
    "add_kbd_v1",
    "add_legend_item_v0",
    "add_legend_item_v1",
    "add_link_v0",
    "add_link_v1",
    "add_list_row_v0",
    "add_list_row_v1",
    "add_member_row_v0",
    "add_member_row_v1",
    "add_metric_comparison_v0",
    "add_metric_comparison_v1",
    "add_metric_row_v0",
    "add_metric_row_v1",
    "add_modal_shell_v0",
    "add_modal_shell_v1",
    "add_nav_chip_row_v0",
    "add_nav_chip_row_v1",
    "add_notification_row_v0",
    "add_notification_row_v1",
    "add_otp_input_v0",
    "add_otp_input_v1",
    "add_pagination_v0",
    "add_pagination_v1",
    "add_phone_input_v0",
    "add_phone_input_v1",
    "add_price_v0",
    "add_price_v1",
    "add_pricing_card_v0",
    "add_pricing_card_v1",
    "add_profile_header_v0",
    "add_profile_header_v1",
    "add_progress_bar_v0",
    "add_progress_bar_v1",
    "add_quote_block_v0",
    "add_quote_block_v1",
    "add_radio_v0",
    "add_radio_v1",
    "add_range_slider_v0",
    "add_range_slider_v1",
    "add_rating_stars_v0",
    "add_rating_stars_v1",
    "add_search_bar_v0",
    "add_search_bar_v1",
    "add_section_header_v0",
    "add_section_header_v1",
    "add_segmented_control_v0",
    "add_segmented_control_v1",
    "add_select_v0",
    "add_select_v1",
    "add_setting_row_v0",
    "add_setting_row_v1",
    "add_share_row_v0",
    "add_share_row_v1",
    "add_sidebar_nav_v0",
    "add_sidebar_nav_v1",
    "add_skeleton_v0",
    "add_skeleton_v1",
    "add_social_login_row_v0",
    "add_social_login_row_v1",
    "add_spinner_v0",
    "add_spinner_v1",
    "add_stat_card_v0",
    "add_stat_card_v1",
    "add_stat_grid_v0",
    "add_stat_grid_v1",
    "add_status_badge_v0",
    "add_status_badge_v1",
    "add_step_card_v0",
    "add_step_card_v1",
    "add_stepper_v0",
    "add_stepper_v1",
    "add_switch_v0",
    "add_switch_v1",
    "add_tabs_v0",
    "add_tabs_v1",
    "add_tag_v0",
    "add_tag_v1",
    "add_text_button_v0",
    "add_text_button_v1",
    "add_textarea_v0",
    "add_textarea_v1",
    "add_timeline_v0",
    "add_timeline_v1",
    "add_toast_v0",
    "add_toast_v1",
    "add_toolbar_v0",
    "add_toolbar_v1",
    "add_tooltip_v0",
    "add_tooltip_v1",
    "add_top_nav_bar_v0",
    "add_top_nav_bar_v1",
    "add_upload_dropzone_v0",
    "add_upload_dropzone_v1",
    "add_user_card_v0",
    "add_user_card_v1",
    "add_video_placeholder_v0",
    "add_video_placeholder_v1",
];

fn component_id_for_ts_element_alias(alias_name: &str) -> &'static str {
    if alias_name.contains("divider") {
        return "divider";
    }
    if contains_any(
        alias_name,
        &[
            "nav",
            "breadcrumb",
            "pagination",
            "tabs",
            "segmented",
            "stepper",
            "carousel_dots",
        ],
    ) {
        return "nav-bar";
    }
    if contains_any(
        alias_name,
        &[
            "input",
            "form_field",
            "textarea",
            "search_bar",
            "select",
            "checkbox",
            "radio",
            "switch",
            "combobox",
            "date_picker",
            "otp",
            "phone",
            "range_slider",
            "upload_dropzone",
            "chip_input",
            "filter_group",
        ],
    ) {
        return "input-text";
    }
    if contains_any(
        alias_name,
        &[
            "button",
            "fab",
            "inline_action",
            "link",
            "action_menu",
            "share_row",
            "toolbar",
        ],
    ) {
        return "btn-primary";
    }
    if contains_any(
        alias_name,
        &[
            "badge",
            "tag",
            "status",
            "alert",
            "toast",
            "tooltip",
            "spinner",
            "progress",
            "activity_ring",
            "avatar",
            "kbd",
            "rating",
            "color_swatch",
            "legend_item",
            "icon_label",
            "price",
        ],
    ) {
        return "badge";
    }
    "card-basic"
}

fn contains_any(s: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| s.contains(needle))
}

/// `insert_<comp>` MCP tool — instantiates one UIKit component onto
/// the active page through the editor command bus.
pub struct InsertKitComponent {
    name: String,
    kit_id: String,
    component_id: String,
}

impl InsertKitComponent {
    pub fn new(kit_id: impl Into<String>, component_id: impl Into<String>) -> Self {
        let component_id = component_id.into();
        Self {
            name: element_tool_name(&component_id),
            kit_id: kit_id.into(),
            component_id,
        }
    }

    fn alias(
        name: impl Into<String>,
        kit_id: impl Into<String>,
        component_id: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            kit_id: kit_id.into(),
            component_id: component_id.into(),
        }
    }
}

impl McpTool for InsertKitComponent {
    fn name(&self) -> &str {
        &self.name
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        match semantic_alias_node(&self.name, args) {
            Ok(Some(node)) => {
                let mut result = BTreeMap::new();
                result.insert("wrote".into(), "true".into());
                result.insert("tool".into(), self.name.clone());
                result.insert("semantic".into(), "true".into());
                return ToolOutcome::OkWithCommand(
                    result,
                    EditorCommand::InsertSubtree {
                        nodes: vec![node],
                        parent_id: parent_id_arg(args),
                        page_id: page_id_arg(args),
                    },
                );
            }
            Ok(None) => {}
            Err(e) => return e,
        }

        // `x` / `y` are optional doc-px floats; omitted slots default
        // to 0.0 at apply time.
        let doc_x = match parse_optional_f64(args, "x") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let doc_y = match parse_optional_f64(args, "y") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let mut result = BTreeMap::new();
        result.insert("kit_id".into(), self.kit_id.clone());
        result.insert("component_id".into(), self.component_id.clone());
        ToolOutcome::OkWithCommand(
            result,
            EditorCommand::InstantiateKitComponent {
                kit_id: self.kit_id.clone(),
                component_id: self.component_id.clone(),
                doc_x,
                doc_y,
            },
        )
    }
}

fn parent_id_arg(args: &BTreeMap<String, String>) -> NodeId {
    args.get("parent_id")
        .or_else(|| args.get("parentId"))
        .or_else(|| args.get("parent"))
        .map(|raw| {
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed == "0" || trimmed == "null" {
                NodeId::NONE
            } else {
                NodeId::new(trimmed)
            }
        })
        .unwrap_or(NodeId::NONE)
}

fn page_id_arg(args: &BTreeMap<String, String>) -> Option<String> {
    args.get("pageId")
        .or_else(|| args.get("page_id"))
        .or_else(|| args.get("page"))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Parse a number arg as `Option<f64>`. An absent slot returns `None`
/// (the command falls back to 0.0); a malformed slot is a hard error
/// so the LLM client retries with a valid value.
///
/// The `Err` variant carries a full `ToolOutcome::Err` — the call site
/// returns it verbatim, so the large size is intentional.
#[allow(clippy::result_large_err)]
fn parse_optional_f64(
    args: &BTreeMap<String, String>,
    key: &str,
) -> Result<Option<f64>, ToolOutcome> {
    match args.get(key) {
        None => Ok(None),
        Some(v) if v.is_empty() => Ok(None),
        Some(v) => v.parse::<f64>().map(Some).map_err(|_| {
            ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                format!("{key} must be a number"),
            )
        }),
    }
}

/// Walk every loaded kit and emit one [`InsertKitComponent`] per
/// component. The host's `rebuild_registry` chains this into the live
/// `ToolRegistry`.
pub fn insert_kit_component_tools(state: &EditorState) -> Vec<InsertKitComponent> {
    let mut tools: Vec<_> = state
        .ui_kits
        .iter()
        .flat_map(|kit: &UIKit| {
            kit.components
                .iter()
                .map(|c| InsertKitComponent::new(kit.id.clone(), c.id.clone()))
        })
        .collect();

    for kit in state.ui_kits.iter().filter(|kit| kit.id == STARTER_KIT_ID) {
        for &alias_name in TS_ELEMENT_ALIASES {
            let component_id = component_id_for_ts_element_alias(alias_name);
            if kit.components.iter().any(|c| c.id == component_id) {
                tools.push(InsertKitComponent::alias(
                    alias_name,
                    kit.id.clone(),
                    component_id,
                ));
            }
        }
    }

    tools
}

/// JSON-encoded `tools/list` schema for one element tool. The host
/// concatenates this into the `tools/list` response next to the static
/// `TOOL_SCHEMAS`.
pub fn element_tool_schema(component_name: &str, component_id: &str) -> String {
    let tool = element_tool_name(component_id);
    canonical_element_tool_schema(&tool, component_name)
}

fn canonical_element_tool_schema(tool: &str, component_name: &str) -> String {
    format!(
        r#"{{"name":"{tool}","description":"Insert a {component_name} from the built-in UIKit onto the active page. Optional x/y doc-px floats place the top-left; defaults to (0, 0).","inputSchema":{{"type":"object","properties":{{"x":{{"type":"string","description":"top-left doc-px (float)"}},"y":{{"type":"string","description":"top-left doc-px (float)"}}}}}}}}"#
    )
}

fn ts_alias_element_tool_schema(tool: &str, component_name: &str) -> String {
    format!(
        r#"{{"name":"{tool}","description":"TS pen-mcp compatible alias. Inserts the Rust starter-kit {component_name} onto the active page. Optional x/y doc-px floats place the top-left; parent_id, pageId, filePath, and schemaVersion are accepted for client compatibility.","inputSchema":{{"type":"object","properties":{{"schemaVersion":{{"type":"string","description":"Accepted for TS element-tool compatibility"}},"filePath":{{"type":"string","description":"Optional target .op file path; omit to use the server document"}},"pageId":{{"type":"string","description":"Accepted for TS compatibility; semantic aliases target the requested page when applicable"}},"parent_id":{{"type":"string","description":"Accepted for TS compatibility; semantic aliases insert under the requested parent when applicable"}},"x":{{"type":"string","description":"top-left doc-px (float)"}},"y":{{"type":"string","description":"top-left doc-px (float)"}}}}}}}}"#
    )
}

/// JSON-encoded schemas for every element tool the live state has —
/// matches the iterator order of [`insert_kit_component_tools`] so
/// counts agree.
pub fn element_tool_schemas(state: &EditorState) -> Vec<String> {
    let mut schemas: Vec<_> = state
        .ui_kits
        .iter()
        .flat_map(|kit| {
            kit.components
                .iter()
                .map(|c| element_tool_schema(&c.name, &c.id))
        })
        .collect();

    for kit in state.ui_kits.iter().filter(|kit| kit.id == STARTER_KIT_ID) {
        for &alias_name in TS_ELEMENT_ALIASES {
            let component_id = component_id_for_ts_element_alias(alias_name);
            if let Some(component) = kit.components.iter().find(|c| c.id == component_id) {
                schemas.push(ts_alias_element_tool_schema(alias_name, &component.name));
            }
        }
    }

    schemas
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_sanitizes_dashes() {
        assert_eq!(element_tool_name("btn-primary"), "insert_btn_primary");
        assert_eq!(element_tool_name("nav-bar"), "insert_nav_bar");
    }

    #[test]
    fn tool_emits_instantiate_command() {
        let tool = InsertKitComponent::new("openpencil-starter", "btn-primary");
        assert_eq!(tool.name(), "insert_btn_primary");
        let mut args = BTreeMap::new();
        args.insert("x".to_string(), "120".to_string());
        args.insert("y".to_string(), "80".to_string());
        match tool.call(&args) {
            ToolOutcome::OkWithCommand(_, cmd) => match cmd {
                EditorCommand::InstantiateKitComponent {
                    kit_id,
                    component_id,
                    doc_x,
                    doc_y,
                } => {
                    assert_eq!(kit_id, "openpencil-starter");
                    assert_eq!(component_id, "btn-primary");
                    assert_eq!(doc_x, Some(120.0));
                    assert_eq!(doc_y, Some(80.0));
                }
                _ => panic!("expected InstantiateKitComponent"),
            },
            other => panic!("expected OkWithCommand, got {other:?}"),
        }
    }

    #[test]
    fn missing_x_y_default_to_none() {
        let tool = InsertKitComponent::new("openpencil-starter", "badge");
        match tool.call(&BTreeMap::new()) {
            ToolOutcome::OkWithCommand(
                _,
                EditorCommand::InstantiateKitComponent { doc_x, doc_y, .. },
            ) => {
                assert_eq!(doc_x, None);
                assert_eq!(doc_y, None);
            }
            other => panic!("expected OkWithCommand, got {other:?}"),
        }
    }

    #[test]
    fn malformed_x_is_a_hard_error() {
        let tool = InsertKitComponent::new("openpencil-starter", "badge");
        let mut args = BTreeMap::new();
        args.insert("x".to_string(), "not-a-number".to_string());
        match tool.call(&args) {
            ToolOutcome::Err(ToolErrorCode::InvalidArgument, _) => {}
            other => panic!("expected InvalidArgument, got {other:?}"),
        }
    }

    #[test]
    fn registry_covers_every_starter_kit_component() {
        let state = EditorState::new();
        let tools = insert_kit_component_tools(&state);
        let schemas = element_tool_schemas(&state);
        assert_eq!(
            tools
                .iter()
                .filter(|tool| tool.name().starts_with("insert_"))
                .count(),
            6,
            "starter kit ships 6 canonical components"
        );
        assert_eq!(schemas.len(), tools.len(), "schema + tool counts agree");
        // Each tool name appears verbatim in its schema.
        for tool in &tools {
            assert!(
                schemas
                    .iter()
                    .any(|s| s.contains(&format!("\"name\":\"{}\"", tool.name()))),
                "schema set must include {}",
                tool.name(),
            );
        }
    }

    #[test]
    fn registry_includes_ts_element_tool_aliases() {
        let state = EditorState::new();
        let tools = insert_kit_component_tools(&state);
        let schemas = element_tool_schemas(&state);

        for name in [
            "add_text_button_v0",
            "add_form_field_v0",
            "add_top_nav_bar_v0",
            "add_divider_v0",
            "add_badge_v0",
            "add_stat_card_v0",
        ] {
            assert!(
                tools.iter().any(|tool| tool.name() == name),
                "registry must include TS-compatible element tool alias {name}"
            );
            assert!(
                schemas
                    .iter()
                    .any(|s| s.contains(&format!("\"name\":\"{name}\""))),
                "tools/list must include TS-compatible element tool alias {name}",
            );
        }
    }

    #[test]
    fn registry_covers_ts_production_element_tool_catalog() {
        let mut ts_names = ts_production_element_tool_names();
        ts_names.sort();
        ts_names.dedup();
        assert!(
            ts_names.len() > 100,
            "TS production element catalog should remain broad"
        );

        let state = EditorState::new();
        let rust_names: std::collections::BTreeSet<_> = insert_kit_component_tools(&state)
            .into_iter()
            .map(|tool| tool.name().to_string())
            .collect();
        let missing: Vec<_> = ts_names
            .into_iter()
            .filter(|name| !rust_names.contains(name))
            .collect();

        assert!(
            missing.is_empty(),
            "Rust MCP element registry is missing TS production tools: {missing:?}",
        );
    }

    fn ts_production_element_tool_names() -> Vec<String> {
        let routes_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../packages/pen-mcp/src/routes");
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
    fn ts_element_alias_emits_instantiate_command() {
        let state = EditorState::new();
        let tool = insert_kit_component_tools(&state)
            .into_iter()
            .find(|tool| tool.name() == "add_top_nav_bar_v0")
            .expect("TS add_top_nav_bar_v0 alias");

        match tool.call(&BTreeMap::new()) {
            ToolOutcome::OkWithCommand(_, cmd) => match cmd {
                EditorCommand::InstantiateKitComponent {
                    kit_id,
                    component_id,
                    ..
                } => {
                    assert_eq!(kit_id, "openpencil-starter");
                    assert_eq!(component_id, "nav-bar");
                }
                _ => panic!("expected InstantiateKitComponent"),
            },
            other => panic!("expected OkWithCommand, got {other:?}"),
        }
    }

    #[test]
    fn add_heading_alias_emits_semantic_text_subtree() {
        let tool = InsertKitComponent::alias("add_heading_v0", STARTER_KIT_ID, "card-basic");
        let mut args = BTreeMap::new();
        args.insert("content".to_string(), "Welcome".to_string());
        args.insert("level".to_string(), "h1".to_string());

        match tool.call(&args) {
            ToolOutcome::OkWithCommand(_, EditorCommand::InsertSubtree { nodes, .. }) => {
                assert_eq!(nodes.len(), 1);
                match &nodes[0] {
                    jian_ops_schema::node::PenNode::Text(text) => {
                        assert_eq!(text.base.name.as_deref(), Some("Heading (h1)"));
                        assert_eq!(text.font_size, Some(32.0));
                        assert_eq!(
                            text.font_weight,
                            Some(jian_ops_schema::node::FontWeight::Number(700))
                        );
                    }
                    other => panic!("expected semantic heading text node, got {other:?}"),
                }
            }
            other => panic!("expected semantic InsertSubtree, got {other:?}"),
        }
    }

    #[test]
    fn add_text_button_alias_emits_semantic_button_subtree() {
        let tool = InsertKitComponent::alias("add_text_button_v0", STARTER_KIT_ID, "btn-primary");
        let mut args = BTreeMap::new();
        args.insert("label".to_string(), "Continue".to_string());

        match tool.call(&args) {
            ToolOutcome::OkWithCommand(_, EditorCommand::InsertSubtree { nodes, .. }) => {
                assert_eq!(nodes.len(), 1);
                match &nodes[0] {
                    jian_ops_schema::node::PenNode::Frame(frame) => {
                        assert_eq!(frame.base.name.as_deref(), Some("Text Button"));
                        let children = frame.children.as_ref().expect("button children");
                        assert_eq!(children.len(), 1);
                        assert!(matches!(
                            &children[0],
                            jian_ops_schema::node::PenNode::Text(text)
                                if text.base.name.as_deref() == Some("Label")
                        ));
                    }
                    other => panic!("expected semantic button frame, got {other:?}"),
                }
            }
            other => panic!("expected semantic InsertSubtree, got {other:?}"),
        }
    }
}
