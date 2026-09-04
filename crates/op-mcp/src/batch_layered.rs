//! Layered design workflow helpers that need TS-shaped structured args.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use jian_ops_schema::node::PenNode;
use op_editor_core::{NodeId, PenNodeExt};
use serde_json::{json, Value};

use super::batch_design::normalize_node_shape;
use super::batch_layered_error::LayeredError;
use super::batch_layered_guidelines::generate_section_guidelines;
use super::batch_page::optional_page_id;
use super::{EditorCommand, ToolErrorCode, ToolOutcome};

static NEXT_SKELETON_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn dispatch_design_skeleton(args: &BTreeMap<String, String>) -> ToolOutcome {
    if let Some(err) = reject_script_with_structured_payload(args) {
        return err;
    }
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
            Err(error) => {
                return ToolOutcome::Err(ToolErrorCode::InvalidArgument, error.to_string())
            }
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
    let mut nodes = vec![root];
    let hoist = super::batch_design::hoist_generation_state(&mut nodes);
    ToolOutcome::OkWithCommand(
        out,
        super::batch_design::with_hoisted_state(
            hoist,
            EditorCommand::InsertAuthoredSubtree {
                nodes,
                parent_id: NodeId::NONE,
                page_id: optional_page_id(args),
            },
        ),
    )
}

pub(crate) fn dispatch_design_content(args: &BTreeMap<String, String>) -> ToolOutcome {
    if let Some(err) = reject_script_with_structured_payload(args) {
        return err;
    }
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
        Err(error) => return ToolOutcome::Err(ToolErrorCode::InvalidArgument, error.to_string()),
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
            post_process_fixes += op_editor_core::command_refine::refine_child_subtree(node).len();
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
    let hoist = super::batch_design::hoist_generation_state(&mut parsed.nodes);
    ToolOutcome::OkWithCommand(
        out,
        super::batch_design::with_hoisted_state(
            hoist,
            EditorCommand::InsertSubtree {
                nodes: parsed.nodes,
                parent_id: NodeId::new(section_id.to_string()),
                page_id: optional_page_id(args),
            },
        ),
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

/// `script` is a `batch_design`-only shorthand expanded by
/// `batch_design::expand_script_arg` before the flat-dispatch path ever runs.
/// The structured phase-tool branches (TS-shaped `rootFrame`+`sections` /
/// `sectionId`+`children` / `rootId`) route straight here without going
/// through `expand_script_arg`, so a call carrying BOTH a structured payload
/// AND `script` would otherwise silently ignore `script` instead of erroring
/// like `batch_design` does. Reject the combination outright — this check is
/// about arg shape, not script expansion, so it applies regardless of the
/// `script` cargo feature.
pub(crate) fn reject_script_with_structured_payload(
    args: &BTreeMap<String, String>,
) -> Option<ToolOutcome> {
    if args.contains_key("script") {
        return Some(ToolOutcome::Err(
            ToolErrorCode::InvalidArgument,
            "script cannot be combined with the structured payload; use script alone or the structured keys alone".into(),
        ));
    }
    None
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
) -> Result<(PenNode, Vec<Value>), LayeredError> {
    let Value::Object(mut root) = root_frame else {
        return Err(LayeredError::RootFrameNotObject);
    };
    if !root.contains_key("width") || !root.contains_key("height") {
        return Err(LayeredError::RootFrameMissingSize);
    }
    root.insert("type".into(), Value::String("frame".into()));
    root.entry("id")
        .or_insert_with(|| Value::String(next_skeleton_id("root")));
    root.entry("name")
        .or_insert_with(|| Value::String("Page".into()));
    root.entry("layout")
        .or_insert_with(|| Value::String("vertical".into()));
    root.entry("gap")
        .or_insert_with(|| json!(default_root_gap(canvas_width)));
    root.entry("fill")
        .or_insert_with(|| json!([{ "type": "solid", "color": "#F8FAFC" }]));

    let mut section_nodes = Vec::new();
    let mut section_rows = Vec::new();
    for (index, section) in sections.into_iter().enumerate() {
        let Value::Object(mut obj) = section else {
            return Err(LayeredError::SectionNotObject { index });
        };
        let Some(name) = obj
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
        else {
            return Err(LayeredError::SectionMissingName { index });
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
            "qualityGuidelines": quality_guidelines(canvas_width),
            "suggestedRoles": suggested_roles,
        }));
        section_nodes.push(Value::Object(obj));
    }
    root.insert("children".into(), Value::Array(section_nodes));
    let mut root_value = Value::Object(root);
    normalize_node_shape(&mut root_value);
    let root = serde_json::from_value(root_value)
        .map_err(|e| LayeredError::InvalidSkeletonNodes(e.to_string()))?;
    Ok((root, section_rows))
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

fn default_root_gap(canvas_width: i32) -> i32 {
    if canvas_width <= 500 {
        20
    } else {
        0
    }
}

fn quality_guidelines(canvas_width: i32) -> &'static str {
    if canvas_width <= 500 {
        "Avoid crowded output: keep one primary mobile task, use an App Content wrapper with 16-20px horizontal padding and 20-24px vertical gaps, and limit above-the-fold modules. Mobile top rhythm: keep the first primary module within 20-32px of the header/title group. Prefer a single compact search row with no oversized tinted shell. Add one signature moment and vary the composition instead of repeating a fixed template."
    } else {
        "Avoid crowded output: use generous section padding, clear type hierarchy, consistent card rhythm, and fewer stronger modules."
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

fn parse_children(raw: &str) -> Result<ParsedChildren, LayeredError> {
    let mut value: Value =
        serde_json::from_str(raw).map_err(|e| LayeredError::ChildrenNotJson(e.to_string()))?;
    let Some(children) = value.as_array_mut() else {
        return Err(LayeredError::ChildrenNotArray);
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
        .map_err(|e| LayeredError::InvalidChildNodes(e.to_string()))?;
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
