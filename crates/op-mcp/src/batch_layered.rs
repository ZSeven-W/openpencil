//! Layered design workflow helpers that need TS-shaped structured args.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use jian_ops_schema::node::PenNode;
use op_editor_core::{NodeId, PenNodeExt};
use serde_json::{json, Value};

use super::batch_design::normalize_node_shape;
use super::batch_page::optional_page_id;
use super::{EditorCommand, ToolErrorCode, ToolOutcome};

static NEXT_SKELETON_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn dispatch_design_skeleton(args: &BTreeMap<String, String>) -> ToolOutcome {
    let root_frame = match parse_object_arg(args, "rootFrame") {
        Ok(value) => value,
        Err((code, message)) => return ToolOutcome::Err(code, message),
    };
    let sections = match args.get("sections") {
        Some(raw) => match serde_json::from_str::<Value>(raw) {
            Ok(Value::Array(items)) => items,
            Ok(_) => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    "sections must be a JSON array".into(),
                );
            }
            Err(e) => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("sections must be valid JSON: {e}"),
                );
            }
        },
        None => {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "sections is required".into(),
            );
        }
    };
    if sections.is_empty() {
        return ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            "sections must contain at least one section".into(),
        );
    }
    let style_guide = match args.get("styleGuide") {
        Some(raw) => match serde_json::from_str::<Value>(raw) {
            Ok(value @ Value::Object(_)) => Some(value),
            Ok(_) => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    "styleGuide must be a JSON object".into(),
                );
            }
            Err(e) => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("styleGuide must be valid JSON: {e}"),
                );
            }
        },
        None => None,
    };
    let canvas_width = parse_canvas_width(args).unwrap_or_else(|| {
        root_frame
            .get("width")
            .and_then(Value::as_f64)
            .map(|n| n.round() as i32)
            .unwrap_or(1200)
    });
    let (root, section_rows) =
        match build_skeleton_root(root_frame, sections, canvas_width, style_guide.as_ref()) {
            Ok(value) => value,
            Err(message) => return ToolOutcome::Err(ToolErrorCode::InvalidArgument, message),
        };
    let root_id = root.id_str().to_string();
    let section_ids = section_rows
        .iter()
        .filter_map(|row| row.get("id").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join(",");
    let sections_json = serde_json::to_string(&section_rows).unwrap_or_else(|_| "[]".to_string());

    let mut out = BTreeMap::new();
    out.insert("wrote".into(), "true".into());
    out.insert("phase".into(), "skeleton".into());
    out.insert("rootId".into(), root_id.clone());
    out.insert("sectionIds".into(), section_ids);
    out.insert("sections".into(), sections_json);
    out.insert(
        "nextSteps".into(),
        format!(
            "For each section, call design_content with the sectionId. After content is populated, call design_refine with rootId=\"{root_id}\"."
        ),
    );
    ToolOutcome::OkWithCommand(
        out,
        EditorCommand::InsertAuthoredSubtree {
            nodes: vec![root],
            parent_id: NodeId::NONE,
            page_id: optional_page_id(args),
        },
    )
}

pub(crate) fn dispatch_design_content(args: &BTreeMap<String, String>) -> ToolOutcome {
    let Some(section_id) = args
        .get("sectionId")
        .or_else(|| args.get("section_id"))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    else {
        return ToolOutcome::Err(
            ToolErrorCode::MissingArgument,
            "sectionId is required".into(),
        );
    };
    let Some(children) = args.get("children") else {
        return ToolOutcome::Err(
            ToolErrorCode::MissingArgument,
            "children is required".into(),
        );
    };
    let mut parsed = match parse_children(children) {
        Ok(parsed) => parsed,
        Err(message) => return ToolOutcome::Err(ToolErrorCode::InvalidArgument, message),
    };
    if parsed.nodes.is_empty() {
        return ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            "children must contain at least one node".into(),
        );
    }
    let post_processed = args
        .get("postProcess")
        .or_else(|| args.get("post_process"))
        .map(|raw| !matches!(raw.trim(), "false" | "0"))
        .unwrap_or(true);
    // `postProcessed: true` must be TRUE: run the deterministic
    // refine passes (emoji strip, unique ids, layout-child position
    // sanitize, screen-bounds clamp) on the parsed content before it
    // is inserted. The TS live-only hook passes (role / icon-path
    // resolution) are not covered here — that residual divergence is
    // documented in `command_refine.rs`.
    let mut post_process_fixes = 0usize;
    if post_processed {
        for node in &mut parsed.nodes {
            post_process_fixes += op_editor_core::command_refine::refine_subtree(node).len();
        }
    }
    let warnings_json =
        serde_json::to_string(&parsed.warnings).unwrap_or_else(|_| "[]".to_string());

    let mut out = BTreeMap::new();
    out.insert("wrote".into(), "true".into());
    out.insert("phase".into(), "content".into());
    out.insert("sectionId".into(), section_id.to_string());
    out.insert("insertedCount".into(), parsed.count.to_string());
    out.insert("warnings".into(), warnings_json);
    out.insert("postProcessed".into(), post_processed.to_string());
    out.insert("postProcessFixes".into(), post_process_fixes.to_string());
    ToolOutcome::OkWithCommand(
        out,
        EditorCommand::InsertSubtree {
            nodes: parsed.nodes,
            parent_id: NodeId::new(section_id.to_string()),
            page_id: optional_page_id(args),
        },
    )
}

pub(crate) fn dispatch_design_refine(args: &BTreeMap<String, String>) -> ToolOutcome {
    let Some(root_id) = args
        .get("rootId")
        .or_else(|| args.get("root_id"))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    else {
        return ToolOutcome::Err(ToolErrorCode::MissingArgument, "rootId is required".into());
    };
    let canvas_width = match parse_canvas_width(args) {
        Some(width) => Some(width),
        None if args.contains_key("canvasWidth") || args.contains_key("canvas_width") => {
            let raw = args
                .get("canvasWidth")
                .or_else(|| args.get("canvas_width"))
                .cloned()
                .unwrap_or_default();
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                format!("canvasWidth must be a positive integer, got {raw:?}"),
            );
        }
        None => None,
    };

    let mut out = BTreeMap::new();
    out.insert("wrote".into(), "true".into());
    out.insert("phase".into(), "refine".into());
    out.insert("rootId".into(), root_id.to_string());
    ToolOutcome::OkWithCommand(
        out,
        EditorCommand::RefineDesign {
            root_id: NodeId::new(root_id.to_string()),
            canvas_width,
            page_id: optional_page_id(args),
        },
    )
}

fn parse_object_arg(
    args: &BTreeMap<String, String>,
    key: &str,
) -> Result<Value, (ToolErrorCode, String)> {
    let Some(raw) = args.get(key) else {
        return Err((ToolErrorCode::MissingArgument, format!("{key} is required")));
    };
    match serde_json::from_str::<Value>(raw) {
        Ok(value @ Value::Object(_)) => Ok(value),
        Ok(_) => Err((
            ToolErrorCode::InvalidArgument,
            format!("{key} must be a JSON object"),
        )),
        Err(e) => Err((
            ToolErrorCode::InvalidArgument,
            format!("{key} must be valid JSON: {e}"),
        )),
    }
}

fn parse_canvas_width(args: &BTreeMap<String, String>) -> Option<i32> {
    args.get("canvasWidth")
        .or_else(|| args.get("canvas_width"))
        .and_then(|raw| raw.trim().parse::<i32>().ok())
        .filter(|width| *width > 0)
}

fn build_skeleton_root(
    root_frame: Value,
    sections: Vec<Value>,
    canvas_width: i32,
    style_guide: Option<&Value>,
) -> Result<(PenNode, Vec<Value>), String> {
    let Value::Object(mut root) = root_frame else {
        return Err("rootFrame must be a JSON object".into());
    };
    if !root.contains_key("width") || !root.contains_key("height") {
        return Err("rootFrame must contain width and height".into());
    }
    root.insert("type".into(), Value::String("frame".into()));
    root.entry("id")
        .or_insert_with(|| Value::String(next_skeleton_id("root")));
    root.entry("name")
        .or_insert_with(|| Value::String("Page".into()));
    root.entry("layout")
        .or_insert_with(|| Value::String("vertical".into()));
    root.entry("gap").or_insert_with(|| json!(0));
    root.entry("fill")
        .or_insert_with(|| json!([{ "type": "solid", "color": "#F8FAFC" }]));

    let mut section_nodes = Vec::new();
    let mut section_rows = Vec::new();
    for (index, section) in sections.into_iter().enumerate() {
        let Value::Object(mut obj) = section else {
            return Err(format!("sections[{index}] must be a JSON object"));
        };
        let Some(name) = obj
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
        else {
            return Err(format!("sections[{index}].name is required"));
        };
        let id = obj
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| next_skeleton_id("section"));
        let content_width = canvas_width - horizontal_padding(&obj);
        let role = obj.get("role").and_then(Value::as_str);
        let (guidelines, suggested_roles) = generate_section_guidelines(
            &name,
            role,
            content_width.max(0),
            canvas_width,
            style_guide,
        );
        obj.insert("type".into(), Value::String("frame".into()));
        obj.insert("id".into(), Value::String(id.clone()));
        obj.entry("width")
            .or_insert_with(|| Value::String("fill_container".into()));
        obj.entry("height")
            .or_insert_with(|| Value::String("fit_content".into()));
        obj.entry("layout")
            .or_insert_with(|| Value::String("vertical".into()));
        obj.entry("children")
            .or_insert_with(|| Value::Array(Vec::new()));
        section_rows.push(json!({
            "id": id,
            "name": name,
            "contentWidth": content_width.max(0),
            "guidelines": guidelines,
            "suggestedRoles": suggested_roles,
        }));
        section_nodes.push(Value::Object(obj));
    }
    root.insert("children".into(), Value::Array(section_nodes));
    let mut root_value = Value::Object(root);
    normalize_node_shape(&mut root_value);
    let root = serde_json::from_value(root_value)
        .map_err(|e| format!("rootFrame/sections must form valid PenNode objects: {e}"))?;
    Ok((root, section_rows))
}

fn generate_section_guidelines(
    name: &str,
    role: Option<&str>,
    content_width: i32,
    canvas_width: i32,
    style_guide: Option<&Value>,
) -> (String, Vec<&'static str>) {
    let name = name.to_lowercase();
    let is_mobile = canvas_width <= 500;
    let accent_color = style_guide
        .and_then(|guide| guide.get("palette"))
        .and_then(|palette| palette.get("accent"))
        .and_then(Value::as_str)
        .unwrap_or("#2563EB");

    if name.contains("nav") || role == Some("navbar") {
        return (
            format!(
                "Horizontal layout with 3 child groups: logo frame, nav-links frame, CTA button. \
Use justifyContent=\"space_between\", alignItems=\"center\". Logo as text (fontSize 18-20, \
fontWeight 700) or frame with icon. Nav links: horizontal frame with gap={}, each link as text \
node. CTA: button with accent fill [{{\"type\":\"solid\",\"color\":\"{}\"}}], white text. \
Content width: {}px.",
                if is_mobile { 16 } else { 32 },
                accent_color,
                content_width
            ),
            vec!["navbar", "nav-links", "nav-link", "button", "label", "icon"],
        );
    }

    if name.contains("hero") || role == Some("hero") {
        let layout_note = if is_mobile {
            "Stack vertically with gap 16-24. Center-align content."
        } else {
            "For desktop with phone mockup: two-column horizontal layout (left text, right phone). \
Without mockup: center-aligned vertical stack."
        };
        return (
            format!(
                "Large headline ({}px, fontWeight 700), subtitle (16-18px, secondary color), \
CTA button(s). {} Use gap=24 between elements. Headline text: textGrowth=\"fixed-width\" if >15 \
chars. Content width: {}px.",
                if is_mobile { "28-36" } else { "40-56" },
                layout_note,
                content_width
            ),
            vec![
                "hero",
                "heading",
                "subheading",
                "body-text",
                "button",
                "phone-mockup",
                "row",
            ],
        );
    }

    if name.contains("feature") || name.contains("功能") {
        return (
            format!(
                "Section title (heading, 28-36px) + subtitle, then {} feature cards in a {} \
layout. Each card: frame with role=\"feature-card\", containing icon (path 20-24px), title \
(text 18-20px), description (text 14-16px). Cards in horizontal row: ALL must use \
width=\"fill_container\" + height=\"fill_container\". Use gap={} between cards. clipContent=true \
+ cornerRadius=12 on cards. Content width: {}px.",
                if is_mobile { "2-3" } else { "3-4" },
                if is_mobile { "vertical" } else { "horizontal" },
                if is_mobile { 16 } else { 24 },
                content_width
            ),
            vec![
                "section",
                "heading",
                "subheading",
                "feature-card",
                "feature-grid",
                "icon",
                "body-text",
            ],
        );
    }

    if name.contains("footer") || role == Some("footer") {
        return (
            format!(
                "{}: logo+tagline, navigation links grouped by category, social icons. Use muted \
text colors for secondary content. Add a divider (height=1, fill border color) above footer if \
needed. Bottom row: copyright text, small links. Content width: {}px.",
                if is_mobile {
                    "Vertical stack"
                } else {
                    "Horizontal layout with 3-4 column groups"
                },
                content_width
            ),
            vec![
                "footer",
                "row",
                "column",
                "nav-links",
                "label",
                "caption",
                "divider",
                "icon",
            ],
        );
    }

    if name.contains("cta") || name.contains("call to action") || role == Some("cta-section") {
        return (
            format!(
                "Centered content: bold headline (28-36px), short subtitle, prominent CTA button. \
Use accent background or gradient for visual distinction. Button: large (padding [16, 40]), \
contrasting color, cornerRadius 8-12. Content width: {}px.",
                content_width
            ),
            vec![
                "cta-section",
                "heading",
                "subheading",
                "button",
                "centered-content",
            ],
        );
    }

    if name.contains("testimonial") || name.contains("review") || name.contains("评价") {
        return (
            format!(
                "Section title + {} testimonial cards in {} layout. Each card: quote text \
(italic or normal, 14-16px), author name, author title/company. Optional: avatar (circle, 48px), \
star rating (5 star icons). Cards in horizontal: width=\"fill_container\" + \
height=\"fill_container\". Content width: {}px.",
                if is_mobile { "1-2" } else { "2-3" },
                if is_mobile { "vertical" } else { "horizontal" },
                content_width
            ),
            vec![
                "section",
                "card",
                "heading",
                "body-text",
                "caption",
                "avatar",
                "row",
            ],
        );
    }

    if name.contains("pricing") || name.contains("价格") || name.contains("plan") {
        return (
            format!(
                "Section title + {} pricing cards in {} layout. Each card: plan name, price \
(large text 36-48px), feature list (each item with check icon + text), CTA button. Highlight the \
recommended plan with accent border or fill. Cards in horizontal: width=\"fill_container\" + \
height=\"fill_container\". Content width: {}px.",
                if is_mobile { "1-2" } else { "2-3" },
                if is_mobile { "vertical" } else { "horizontal" },
                content_width
            ),
            vec![
                "section",
                "pricing-card",
                "heading",
                "label",
                "body-text",
                "button",
                "icon",
                "divider",
            ],
        );
    }

    if name.contains("stat") || name.contains("数据") || name.contains("metric") {
        return (
            format!(
                "{} layout. Each stat: large number (fontSize 36-48px, fontWeight 700), label \
text (14px, secondary color). Optional: icon or trend indicator. Cards: width=\"fill_container\" \
+ height=\"fill_container\". Content width: {}px.",
                if is_mobile {
                    "2x2 grid"
                } else {
                    "3-4 stat cards in horizontal"
                },
                content_width
            ),
            vec![
                "stats-section",
                "stat-card",
                "heading",
                "caption",
                "icon",
                "row",
            ],
        );
    }

    if name.contains("form")
        || name.contains("login")
        || name.contains("signup")
        || name.contains("register")
        || name.contains("表单")
        || name.contains("登录")
        || name.contains("注册")
    {
        return (
            format!(
                "Vertical layout with gap=16-20. ALL inputs MUST use width=\"fill_container\". \
Input fields: frame with role=\"form-input\", height=48, light bg, subtle border. Include \
placeholder text nodes inside inputs. Submit button: width=\"fill_container\", height=48, accent \
fill, white text. Keep form elements (inputs + submit button) together - do NOT split. Optional: \
social login buttons (horizontal frame, each width=\"fit_content\"). Content width: {}px.",
                content_width
            ),
            vec![
                "form-group",
                "form-input",
                "input",
                "button",
                "label",
                "caption",
                "divider",
                "icon",
            ],
        );
    }

    if name.contains("header") || name.contains("顶部") {
        return (
            format!(
                "Horizontal layout with justifyContent=\"space_between\", alignItems=\"center\". \
Left: back icon or menu icon. Center: title text. Right: action icon(s). Height: {}px. Content \
width: {}px.",
                if is_mobile { 56 } else { 64 },
                content_width
            ),
            vec!["row", "heading", "icon-button", "icon", "label"],
        );
    }

    if name.contains("sidebar") || name.contains("侧边栏") {
        return (
            format!(
                "Vertical layout with gap=4-8. Fixed width (240-280px). Items: horizontal frame \
with icon (20px) + text label, padding=[8,16], gap=12, alignItems=\"center\". Active item: accent \
fill or left border indicator. Group labels: uppercase caption text, letterSpacing=1-2. Content \
width: {}px.",
                content_width
            ),
            vec![
                "column", "nav-link", "icon", "label", "caption", "divider", "heading",
            ],
        );
    }

    (
        format!(
            "Vertical layout section. Content should be wrapped in a centered content frame if \
desktop. Use heading (28-36px) for section title, body-text (16px) for descriptions. All text >15 \
chars: textGrowth=\"fixed-width\" + width=\"fill_container\". Content width: {}px.",
            content_width
        ),
        vec![
            "section",
            "heading",
            "subheading",
            "body-text",
            "button",
            "row",
            "card",
        ],
    )
}

fn horizontal_padding(section: &serde_json::Map<String, Value>) -> i32 {
    match section.get("padding") {
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0).max(0) as i32 * 2,
        Some(Value::Array(items)) if items.len() == 2 => {
            items[1].as_i64().unwrap_or(0).max(0) as i32 * 2
        }
        Some(Value::Array(items)) if items.len() == 4 => {
            (items[1].as_i64().unwrap_or(0).max(0) + items[3].as_i64().unwrap_or(0).max(0)) as i32
        }
        _ => 0,
    }
}

fn next_skeleton_id(kind: &str) -> String {
    let seq = NEXT_SKELETON_ID.fetch_add(1, Ordering::Relaxed);
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("op-skeleton-{stamp}-{kind}-{seq}")
}

struct ParsedChildren {
    nodes: Vec<PenNode>,
    count: usize,
    warnings: Vec<String>,
}

fn parse_children(raw: &str) -> Result<ParsedChildren, String> {
    let mut value: Value =
        serde_json::from_str(raw).map_err(|e| format!("children must be a JSON array: {e}"))?;
    let Some(children) = value.as_array_mut() else {
        return Err("children must be a JSON array".into());
    };
    let mut next_id = 1usize;
    let mut warnings = Vec::new();
    for child in children.iter_mut() {
        default_missing_type(child, &mut warnings);
        normalize_node_shape(child);
        ensure_child_node_ids(child, &mut next_id);
        collect_content_warnings(child, &mut warnings);
    }
    let count = count_json_nodes(&value);
    let nodes = serde_json::from_value(value)
        .map_err(|e| format!("children must contain valid PenNode objects: {e}"))?;
    Ok(ParsedChildren {
        nodes,
        count,
        warnings,
    })
}

/// TS `assignIds` (design-content.ts) defaults a node missing `type` to
/// "frame" and emits a warning, recursively. Without this a missing-type
/// node hard-errors at PenNode deserialization in Rust, diverging from TS
/// which renders it as a frame and just warns the LLM.
fn default_missing_type(value: &mut Value, warnings: &mut Vec<String>) {
    let Value::Object(obj) = value else {
        return;
    };
    if !obj.contains_key("type") {
        let name = obj
            .get("name")
            .or_else(|| obj.get("id"))
            .and_then(Value::as_str)
            .unwrap_or("node")
            .to_string();
        warnings.push(format!(
            "Node \"{name}\" missing type - defaulting to \"frame\""
        ));
        obj.insert("type".into(), Value::String("frame".into()));
    }
    if let Some(Value::Array(children)) = obj.get_mut("children") {
        for child in children {
            default_missing_type(child, warnings);
        }
    }
}

fn ensure_child_node_ids(value: &mut Value, next: &mut usize) {
    let Value::Object(obj) = value else {
        return;
    };
    if obj.contains_key("type") && !obj.contains_key("id") {
        obj.insert(
            "id".into(),
            Value::String(format!("__op_tmp_content_{next}")),
        );
        *next += 1;
    }
    if let Some(Value::Array(children)) = obj.get_mut("children") {
        for child in children {
            ensure_child_node_ids(child, next);
        }
    }
}

fn count_json_nodes(value: &Value) -> usize {
    match value {
        Value::Array(items) => items.iter().map(count_json_nodes).sum(),
        Value::Object(obj) => {
            1 + obj
                .get("children")
                .map(count_json_nodes)
                .unwrap_or_default()
        }
        _ => 0,
    }
}

fn collect_content_warnings(value: &Value, warnings: &mut Vec<String>) {
    let Value::Object(obj) = value else {
        return;
    };
    if obj.get("type").and_then(Value::as_str) == Some("text") {
        let content = obj.get("content").and_then(Value::as_str).unwrap_or("");
        if content.chars().count() > 15 && !obj.contains_key("textGrowth") {
            warnings.push(format!(
                "Text \"{}...\" is >15 chars without textGrowth=\"fixed-width\" - may overflow",
                content.chars().take(20).collect::<String>()
            ));
        }
        if obj.get("height").and_then(Value::as_f64).is_some() {
            let name = obj
                .get("name")
                .or_else(|| obj.get("id"))
                .and_then(Value::as_str)
                .unwrap_or("text");
            warnings.push(format!(
                "Text \"{name}\" has explicit pixel height - will be removed by post-processing"
            ));
        }
    }
    if obj.get("type").and_then(Value::as_str) == Some("frame")
        && obj
            .get("cornerRadius")
            .and_then(Value::as_f64)
            .is_some_and(|radius| radius > 0.0)
        && !obj.contains_key("clipContent")
        && obj
            .get("children")
            .and_then(Value::as_array)
            .is_some_and(|children| {
                children
                    .iter()
                    .any(|child| child.get("type").and_then(Value::as_str) == Some("image"))
            })
    {
        let name = obj
            .get("name")
            .or_else(|| obj.get("id"))
            .and_then(Value::as_str)
            .unwrap_or("frame");
        warnings.push(format!(
            "Frame \"{name}\" has cornerRadius + image child but no clipContent - will be auto-added"
        ));
    }
    if let Some(Value::Array(children)) = obj.get("children") {
        for child in children {
            collect_content_warnings(child, warnings);
        }
    }
}
