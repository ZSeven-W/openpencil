//! Element Manifest 解析 — 弱模型的 JSONL 元素清单 → `PenNode` forest。
//! Spec: openpencil-docs/superpowers/specs/2026-06-10-element-manifest-v2.md。
//!
//! 清单行形如 `{"el":"stat_card","label":"MRR","value":"$48k"}`，每行一个
//! 元素声明；`{"el":"section",...}` 是唯一的纯结构容器，后续行用
//! `"in": <1-based 行号>` 嵌入其中。**协议禁止任何 id 字段**——`in` 只能
//! 引用本清单中更早的行，绝不查文档 id，parent_id 幻觉在协议层物理消失。
//! 带 `"type"` 的行按既有裸 PenNode 路径解析（逃生舱），与清单行可混排。
//! 逐行容错：坏行丢弃 + warning，不毁整段（对齐 `parse` 模块的容错哲学）。

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use jian_ops_schema::node::PenNode;
use serde_json::Value;

use crate::parse::{
    balanced_spans, is_node_object, normalize_generated_node_json, strip_reasoning,
};

/// 清单解析结果。`nodes` 已按 `in` 引用组装成 forest，可直接送入与裸
/// PenNode 路径相同的后处理管线（role_infer → role_post_pass → …）。
#[derive(Debug)]
pub struct ManifestOutcome {
    pub nodes: Vec<PenNode>,
    /// 修复/降级轨迹（W-* 条目），上层应转发到 progress/log 流。
    pub warnings: Vec<String>,
    /// 成功构建的元素行数（`"el"` 行）。
    pub element_lines: usize,
    /// 走逃生舱的裸 PenNode 行数。
    pub raw_node_lines: usize,
    /// 解析/构建失败被丢弃的行数。
    pub dropped_lines: usize,
}

/// 仅 `section` 在 M1 接受 `in` 子元素（shell 槽位定向是后续工作）。
const SECTION_KIND: &str = "section";
/// section 嵌套深度上限（spec D1）：section-in-section 封顶，元素叶子不限。
const MAX_SECTION_DEPTH: usize = 2;

static NEXT_SECTION_ID: AtomicU64 = AtomicU64::new(1);

/// Element-manifest protocol switch — **off by default**.
///
/// The manifest (a.k.a. "N-tool") element-scaffold path is opt-in: it
/// runs only when `OPENPENCIL_MANIFEST` is explicitly truthy
/// (`1` / `true` / `on`). Any other value, or the var being unset,
/// keeps it off and routes generation through the bare-PenNode JSONL
/// path (`parse::parse_nodes`).
///
/// The earlier M4 per-model-family auto-routing (minimax / glm-5.1 /
/// deepseek / ark-code routed ON without env) was retired so the Rust
/// build matches the TS `ENABLE_ELEMENT_TOOLS_IN_ORCHESTRATOR`
/// flag-off-by-default posture. `model_id` is therefore no longer
/// consulted; the parameter is kept for call-site stability and a
/// possible future re-introduction of model-aware routing.
pub fn manifest_enabled_for_model(_model_id: &str) -> bool {
    matches!(
        std::env::var("OPENPENCIL_MANIFEST").as_deref(),
        Ok("1") | Ok("true") | Ok("on")
    )
}

/// 从 LLM 文本解析元素清单。文本里没有任何 `"el"` 行时返回 `None`，
/// 调用方应回落到 [`crate::parse::parse_nodes`] 的裸 PenNode 路径。
pub fn parse_manifest(text: &str) -> Option<ManifestOutcome> {
    let text = strip_reasoning(text);
    let mut lines: Vec<Line> = Vec::new();
    let mut warnings = Vec::new();
    for span in balanced_spans(text, b'{', b'}') {
        let Ok(value) = serde_json::from_str::<Value>(span) else {
            continue; // 散文里的花括号片段，与 parse 模块同样静默跳过
        };
        let Value::Object(object) = value else {
            continue;
        };
        if object.get("el").and_then(Value::as_str).is_some() {
            lines.push(Line::Element(object));
        } else if is_node_object(&Value::Object(object.clone())) {
            lines.push(Line::RawNode(object));
        }
        // 其余对象（散落的 {type:solid} 字段对象等）按既有惯例过滤。
    }
    if !lines
        .iter()
        .any(|line| matches!(line, Line::Element(object) if object.contains_key("el")))
    {
        return None;
    }

    let mut entries: Vec<Entry> = Vec::with_capacity(lines.len());
    let (mut element_lines, mut raw_node_lines, mut dropped_lines) = (0usize, 0usize, 0usize);
    for (idx, line) in lines.into_iter().enumerate() {
        let line_no = idx + 1;
        let entry = match line {
            Line::Element(object) => build_element_entry(line_no, object, &entries, &mut warnings),
            Line::RawNode(object) => build_raw_entry(line_no, object, &entries, &mut warnings),
        };
        match &entry {
            Entry::Section { .. } | Entry::Leaf { .. } => {
                if matches!(entry, Entry::Leaf { raw: true, .. }) {
                    raw_node_lines += 1;
                } else {
                    element_lines += 1;
                }
            }
            Entry::Dropped => dropped_lines += 1,
        }
        entries.push(entry);
    }

    Some(ManifestOutcome {
        nodes: assemble_forest(entries),
        warnings,
        element_lines,
        raw_node_lines,
        dropped_lines,
    })
}

enum Line {
    Element(serde_json::Map<String, Value>),
    RawNode(serde_json::Map<String, Value>),
}

enum Entry {
    /// 纯结构容器（`el:"section"`）。`depth` 为 1（根）或 2（子 section）。
    Section {
        node: Option<PenNode>,
        parent: Option<usize>,
        depth: usize,
    },
    /// 元素行 / 裸节点行——叶子，不接受 `in` 引用。
    Leaf {
        node: Option<PenNode>,
        parent: Option<usize>,
        raw: bool,
    },
    /// 构建失败：占住行号（模型按自己输出的行计数），不产节点。
    Dropped,
}

/// 协议禁止的字段——出现即剥除 + warning（id 全部系统分配）。
const FORBIDDEN_KEYS: &[&str] = &[
    "id",
    "parent_id",
    "parentId",
    "pageId",
    "page_id",
    "filePath",
];

/// 模型常把裸节点类型当 `el` kind 写（`{"el":"text",...}`）——按逃生舱
/// 路由而不是丢弃。
const RAW_NODE_KINDS: &[&str] = &[
    "frame",
    "group",
    "rectangle",
    "ellipse",
    "line",
    "polygon",
    "path",
    "text",
    "text_input",
    "image",
    "icon_font",
];

fn build_element_entry(
    line_no: usize,
    mut object: serde_json::Map<String, Value>,
    entries: &[Entry],
    warnings: &mut Vec<String>,
) -> Entry {
    let kind = object
        .get("el")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if RAW_NODE_KINDS.contains(&kind.as_str()) {
        warnings.push(format!(
            "W-RAW-KIND line {line_no}: el:{kind:?} is a raw node type — built as a raw node"
        ));
        object.remove("el");
        object.insert("type".to_string(), Value::String(kind));
        return build_raw_entry(line_no, object, entries, warnings);
    }
    strip_forbidden_keys(line_no, &mut object, warnings);
    let parent = resolve_in_ref(line_no, &object, entries, warnings);

    if kind.trim().eq_ignore_ascii_case(SECTION_KIND) {
        let depth = match parent {
            Some(parent_idx) => section_depth(entries, parent_idx) + 1,
            None => 1,
        };
        let (parent, depth) = if depth > MAX_SECTION_DEPTH {
            warnings.push(format!(
                "W-DEPTH line {line_no}: section nesting exceeds {MAX_SECTION_DEPTH} — clamped to top level"
            ));
            (None, 1)
        } else {
            (parent, depth)
        };
        return match build_section_node(&object) {
            Ok(node) => Entry::Section {
                node: Some(node),
                parent,
                depth,
            },
            Err(message) => {
                warnings.push(format!("W-DROPPED line {line_no} (section): {message}"));
                Entry::Dropped
            }
        };
    }

    let args = element_args(&object);
    match op_mcp::element_manifest::build_element(&kind, &args) {
        Ok(built) => {
            for warning in built.warnings {
                warnings.push(format!("line {line_no}: {warning}"));
            }
            Entry::Leaf {
                node: Some(built.node),
                parent,
                raw: false,
            }
        }
        // Unknown kind = the model invented a semantic name not in the
        // catalog. Most often it's a *container* (`card` / `container` /
        // `box` / `wrapper` / `list` …) the model groups children into via
        // `in:`. Dropping it loses the whole subtree — children then dangle
        // and flatten to siblings (the横排 overlap we saw with glm-5.2's
        // `card`). Coerce to a raw frame instead: layout/style attrs
        // (cornerRadius / fill / layout / gap / padding) carry over, it
        // counts as a container (raw Frame, see resolve_in_ref), and
        // children nest correctly. A *known* kind that fails on args
        // (BuildFailed) still drops — that's a real builder rejection.
        Err(op_mcp::element_manifest::ElementBuildError::UnknownKind(_)) => {
            warnings.push(format!(
                "W-COERCED-FRAME line {line_no}: unknown element kind {kind:?} → raw frame container"
            ));
            object.remove("el");
            object.insert("type".to_string(), Value::String("frame".to_string()));
            // Frame needs concrete sizing/layout; the model usually gives a
            // card cornerRadius/fill but not width/height — fill flex-section
            // defaults so `from_value::<PenNode>` succeeds and the frame lays
            // out as a normal vertical container.
            object
                .entry("width")
                .or_insert_with(|| Value::String("fill_container".to_string()));
            object
                .entry("height")
                .or_insert_with(|| Value::String("fit_content".to_string()));
            object
                .entry("layout")
                .or_insert_with(|| Value::String("vertical".to_string()));
            build_raw_entry(line_no, object, entries, warnings)
        }
        Err(err) => {
            warnings.push(format!("W-DROPPED line {line_no} ({kind}): {err}"));
            Entry::Dropped
        }
    }
}

fn build_raw_entry(
    line_no: usize,
    mut object: serde_json::Map<String, Value>,
    entries: &[Entry],
    warnings: &mut Vec<String>,
) -> Entry {
    // 逃生舱行同样不许引用文档 id；`_parent` 链在清单模式下不支持，
    // 嵌套要么行内 children 要么 `in`。
    if object.remove("_parent").is_some() {
        warnings.push(format!(
            "W-RAW-PARENT line {line_no}: `_parent` is ignored in manifest mode — use `in`"
        ));
    }
    // 逃生舱行允许 `in`（与元素行同规则）；raw Frame 行还可被后续行
    // 的 `in` 当容器（方舟实测会用自绘 frame 组卡片再往里挂内容）。
    let parent = resolve_in_ref(line_no, &object, entries, warnings);
    object.remove("in");
    let mut value = Value::Object(object);
    // 协议教模型"不写 id"，但 PenNode 的 id 必填——整棵子树（含行内
    // `children`）里缺 id 的节点都由系统补发。
    inject_missing_ids(&mut value);
    normalize_generated_node_json(&mut value);
    match serde_json::from_value::<PenNode>(value) {
        Ok(node) => Entry::Leaf {
            node: Some(node),
            parent,
            raw: true,
        },
        Err(err) => {
            warnings.push(format!("W-DROPPED line {line_no} (raw node): {err}"));
            Entry::Dropped
        }
    }
}

/// 清单行剩余字段 → builder 的 `BTreeMap<String, String>` 参数契约：
/// 字符串原样、数字/布尔字符串化、数组/对象序列化为 JSON 字符串
/// （builder 侧的 `parse_array_items` 等按 JSON 字符串解析）。
fn element_args(object: &serde_json::Map<String, Value>) -> BTreeMap<String, String> {
    object
        .iter()
        .filter(|(key, _)| key.as_str() != "el" && key.as_str() != "in")
        .filter_map(|(key, value)| {
            let text = match value {
                Value::String(text) => text.clone(),
                Value::Number(number) => number.to_string(),
                Value::Bool(flag) => flag.to_string(),
                Value::Array(_) | Value::Object(_) => serde_json::to_string(value).ok()?,
                Value::Null => return None,
            };
            Some((key.clone(), text))
        })
        .collect()
}

/// 给 raw 子树里所有缺 id 的节点对象补发系统 id（递归 `children`）。
fn inject_missing_ids(value: &mut Value) {
    let Value::Object(object) = value else { return };
    let missing_id = object
        .get("id")
        .and_then(Value::as_str)
        .map(|id| id.trim().is_empty())
        .unwrap_or(true);
    if missing_id {
        let id = NEXT_SECTION_ID.fetch_add(1, Ordering::Relaxed);
        object.insert(
            "id".to_string(),
            Value::String(format!("__op_man_raw_{id}")),
        );
    }
    if let Some(Value::Array(children)) = object.get_mut("children") {
        for child in children {
            inject_missing_ids(child);
        }
    }
}

fn strip_forbidden_keys(
    line_no: usize,
    object: &mut serde_json::Map<String, Value>,
    warnings: &mut Vec<String>,
) {
    for key in FORBIDDEN_KEYS {
        if object.remove(*key).is_some() {
            warnings.push(format!(
                "W-ID-FORBIDDEN line {line_no}: {key:?} is system-assigned — ignored"
            ));
        }
    }
}

/// `in` 引用 → 父 entry 下标。只接受指向更早 section 行的 1-based 行号；
/// 悬空 / 前向 / 非容器 / 自指 → 当根处理 + warning。
fn resolve_in_ref(
    line_no: usize,
    object: &serde_json::Map<String, Value>,
    entries: &[Entry],
    warnings: &mut Vec<String>,
) -> Option<usize> {
    let value = object.get("in")?;
    let Some(target) = value_as_line_no(value) else {
        warnings.push(format!(
            "W-IN-INVALID line {line_no}: `in` must be a 1-based line number — treated as top level"
        ));
        return None;
    };
    if target == 0 || target >= line_no {
        warnings.push(format!(
            "W-IN-DANGLING line {line_no}: `in: {target}` does not point to an earlier line — treated as top level"
        ));
        return None;
    }
    let idx = target - 1;
    match entries.get(idx) {
        Some(Entry::Section { node: Some(_), .. }) => Some(idx),
        // 模型自绘的 raw frame 是它自己的结构——接受为容器（方舟用
        // 这个模式组卡片）。元素行的内部结构归 builder 所有，仍退化
        // 为兄弟（见下）。
        Some(Entry::Leaf {
            node: Some(PenNode::Frame(_)),
            raw: true,
            ..
        }) => Some(idx),
        Some(Entry::Dropped) => {
            warnings.push(format!(
                "W-IN-DANGLING line {line_no}: `in: {target}` points to a dropped line — treated as top level"
            ));
            None
        }
        // 指向元素（模型想做槽位嵌套，M1 仅 section 可容纳）→ 退化为
        // 该元素的兄弟，保住局部性：目标在哪个 section 里，本行就进
        // 哪个 section，而不是被甩到顶层。
        Some(Entry::Leaf {
            node: Some(_),
            parent,
            ..
        }) => {
            warnings.push(format!(
                "W-IN-NOT-CONTAINER line {line_no}: line {target} is not a section — treated as its sibling"
            ));
            *parent
        }
        _ => {
            warnings.push(format!(
                "W-IN-NOT-CONTAINER line {line_no}: line {target} is not a section — treated as top level"
            ));
            None
        }
    }
}

fn value_as_line_no(value: &Value) -> Option<usize> {
    match value {
        Value::Number(number) => number.as_u64().map(|n| n as usize),
        Value::String(text) => text.trim().parse::<usize>().ok(),
        _ => None,
    }
}

fn section_depth(entries: &[Entry], idx: usize) -> usize {
    match entries.get(idx) {
        Some(Entry::Section { depth, .. }) => *depth,
        _ => 0,
    }
}

/// `el:"section"` → 一个干净的 flex frame。role 进 `name` 供下游
/// role resolver 识别；padding 透传数组/数字，其它形态忽略。
fn build_section_node(object: &serde_json::Map<String, Value>) -> Result<PenNode, String> {
    let direction = object
        .get("direction")
        .and_then(Value::as_str)
        .unwrap_or("vertical");
    let layout = if direction.eq_ignore_ascii_case("horizontal") {
        "horizontal"
    } else {
        "vertical"
    };
    let gap = object
        .get("gap")
        .and_then(|value| match value {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.trim().parse::<f64>().ok(),
            _ => None,
        })
        .unwrap_or(20.0);
    let padding = object
        .get("padding")
        .filter(|value| value.is_array() || value.is_number())
        .cloned()
        .unwrap_or(Value::from(0));
    let name = object
        .get("role")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|role| !role.is_empty())
        .unwrap_or("Section");
    let id = NEXT_SECTION_ID.fetch_add(1, Ordering::Relaxed);
    serde_json::from_value(serde_json::json!({
        "id": format!("__op_man_section_{id}"),
        "type": "frame",
        "name": name,
        "width": "fill_container",
        "height": "fit_content",
        "layout": layout,
        "gap": gap,
        "padding": padding,
        "fill": [],
    }))
    .map_err(|err| err.to_string())
}

/// 把扁平 entries 按 parent 引用组装成 forest（清单顺序即兄弟顺序）。
fn assemble_forest(mut entries: Vec<Entry>) -> Vec<PenNode> {
    // 先登记每个 section 的孩子（顺序 = 行序）。越界 / 指向非 section 的
    // parent 在 resolve 阶段已清洗；raw 行的简化路径在这里兜底。
    let mut children_of: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let mut roots: Vec<usize> = Vec::new();
    for idx in 0..entries.len() {
        let parent = match &entries[idx] {
            Entry::Section {
                node: Some(_),
                parent,
                ..
            } => *parent,
            Entry::Leaf {
                node: Some(_),
                parent,
                ..
            } => parent.filter(|p| {
                *p < idx
                    && matches!(
                        entries.get(*p),
                        Some(Entry::Section { node: Some(_), .. })
                            | Some(Entry::Leaf {
                                node: Some(PenNode::Frame(_)),
                                raw: true,
                                ..
                            })
                    )
            }),
            _ => continue,
        };
        match parent {
            Some(parent_idx) => children_of.entry(parent_idx).or_default().push(idx),
            None => roots.push(idx),
        }
    }
    // 自底向上摘取：行号引用只能向前，倒序遍历保证孩子先于父被装配。
    for idx in (0..entries.len()).rev() {
        let Some(child_indices) = children_of.remove(&idx) else {
            continue;
        };
        let mut children = Vec::with_capacity(child_indices.len());
        for child_idx in child_indices {
            if let Some(node) = take_node(&mut entries[child_idx]) {
                children.push(node);
            }
        }
        if children.is_empty() {
            continue;
        }
        if let Some(PenNode::Frame(frame)) = peek_node_mut(&mut entries[idx]) {
            frame.children.get_or_insert_with(Vec::new).extend(children);
        }
    }
    roots
        .into_iter()
        .filter_map(|idx| take_node(&mut entries[idx]))
        .collect()
}

fn take_node(entry: &mut Entry) -> Option<PenNode> {
    match entry {
        Entry::Section { node, .. } | Entry::Leaf { node, .. } => node.take(),
        Entry::Dropped => None,
    }
}

fn peek_node_mut(entry: &mut Entry) -> Option<&mut PenNode> {
    match entry {
        Entry::Section { node, .. } | Entry::Leaf { node, .. } => node.as_mut(),
        Entry::Dropped => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(nodes: &[PenNode]) -> Vec<String> {
        nodes
            .iter()
            .map(|node| match node {
                PenNode::Frame(frame) => frame.base.name.clone().unwrap_or_default(),
                PenNode::Text(text) => text.base.name.clone().unwrap_or_default(),
                _ => String::new(),
            })
            .collect()
    }

    fn frame_children(node: &PenNode) -> &[PenNode] {
        match node {
            PenNode::Frame(frame) => frame.children.as_deref().unwrap_or(&[]),
            _ => &[],
        }
    }

    #[test]
    fn parses_elements_nested_in_a_section() {
        let text = r#"
Here is the design:
{"el":"section","direction":"vertical","gap":16,"role":"hero"}
{"el":"heading","in":1,"content":"Revenue Overview"}
{"el":"stat_card","in":1,"label":"MRR","value":"$48.2k"}
"#;
        let outcome = parse_manifest(text).expect("manifest lines present");
        assert_eq!(outcome.element_lines, 3);
        assert_eq!(outcome.dropped_lines, 0);
        assert_eq!(outcome.nodes.len(), 1, "one section root");
        assert_eq!(frame_children(&outcome.nodes[0]).len(), 2);
    }

    #[test]
    fn returns_none_without_element_lines() {
        assert!(parse_manifest(r#"[{"type":"frame","id":"a"}]"#).is_none());
        assert!(parse_manifest("just prose, no JSON at all").is_none());
    }

    #[test]
    fn dangling_and_forward_in_refs_degrade_to_top_level() {
        let text = r#"
{"el":"stat_card","in":7,"label":"A","value":"1"}
{"el":"stat_card","in":2,"label":"B","value":"2"}
"#;
        let outcome = parse_manifest(text).expect("manifest");
        assert_eq!(outcome.nodes.len(), 2, "both degrade to roots");
        assert!(
            outcome.warnings.iter().any(|w| w.contains("W-IN-DANGLING")),
            "{:?}",
            outcome.warnings
        );
    }

    #[test]
    fn in_pointing_at_non_container_degrades() {
        let text = r#"
{"el":"heading","content":"Title"}
{"el":"badge","in":1,"label":"New"}
"#;
        let outcome = parse_manifest(text).expect("manifest");
        assert_eq!(outcome.nodes.len(), 2);
        assert!(outcome
            .warnings
            .iter()
            .any(|w| w.contains("W-IN-NOT-CONTAINER")));
    }

    #[test]
    fn unknown_kind_used_as_container_coerces_to_frame() {
        // glm-5.2 invents `card` (not in the catalog) as a container and
        // nests children into it via `in:`. Dropping it would dangle the
        // children → they flatten to siblings → horizontal overflow/overlap
        // (the崩 layout reported 2026-06-18). It must coerce to a raw frame
        // that holds the children, keeping the card's cornerRadius/fill.
        let text = r##"
{"el":"section","direction":"vertical","role":"popular"}
{"el":"card","in":1,"cornerRadius":16,"fill":[{"type":"solid","color":"#FFFFFF"}]}
{"el":"stat_card","in":2,"label":"MRR","value":"$48k"}
"##;
        let outcome = parse_manifest(text).expect("manifest");
        assert_eq!(outcome.nodes.len(), 1, "one section root");
        assert!(
            outcome
                .warnings
                .iter()
                .any(|w| w.contains("W-COERCED-FRAME")),
            "expected a coerce warning, got {:?}",
            outcome.warnings
        );
        assert!(
            !outcome.warnings.iter().any(|w| w.contains("W-DROPPED")),
            "card must NOT be dropped: {:?}",
            outcome.warnings
        );
        // section > card(frame) > stat_card — nested, NOT flattened to siblings.
        let section_kids = frame_children(&outcome.nodes[0]);
        assert_eq!(
            section_kids.len(),
            1,
            "section holds the coerced card frame"
        );
        assert!(
            matches!(&section_kids[0], PenNode::Frame(_)),
            "card coerced to a frame container"
        );
        assert_eq!(
            frame_children(&section_kids[0]).len(),
            1,
            "stat_card nested inside the card, not flattened to a sibling"
        );
    }

    #[test]
    fn forbidden_id_fields_are_stripped_with_warning() {
        let text = r#"{"el":"badge","label":"New","id":"my-id","parent_id":"root"}"#;
        let outcome = parse_manifest(text).expect("manifest");
        assert_eq!(outcome.nodes.len(), 1);
        assert_eq!(
            outcome
                .warnings
                .iter()
                .filter(|w| w.contains("W-ID-FORBIDDEN"))
                .count(),
            2
        );
    }

    #[test]
    fn section_depth_is_clamped_at_two() {
        let text = r#"
{"el":"section","role":"page"}
{"el":"section","in":1,"role":"row"}
{"el":"section","in":2,"role":"too-deep"}
"#;
        let outcome = parse_manifest(text).expect("manifest");
        assert!(outcome.warnings.iter().any(|w| w.contains("W-DEPTH")));
        // 第三个 section 被钳到顶层：根 = page + too-deep。
        assert_eq!(outcome.nodes.len(), 2);
        assert_eq!(names(&outcome.nodes), vec!["page", "too-deep"]);
    }

    #[test]
    fn el_with_raw_node_kind_routes_to_escape_hatch() {
        // 模型把裸文本写成 `el:"text"` —— 路由到逃生舱而不是丢弃；
        // 协议不写 id，系统补发。
        let text = r#"
{"el":"section","role":"hero"}
{"el":"text","in":1,"content":"Hand-written copy","fontSize":14}
"#;
        let outcome = parse_manifest(text).expect("manifest");
        assert_eq!(outcome.dropped_lines, 0);
        assert_eq!(outcome.raw_node_lines, 1);
        assert_eq!(outcome.nodes.len(), 1);
        assert_eq!(frame_children(&outcome.nodes[0]).len(), 1);
        assert!(outcome.warnings.iter().any(|w| w.contains("W-RAW-KIND")));
    }

    #[test]
    fn in_pointing_at_element_becomes_its_sibling() {
        // `in` 指向元素（想做槽位嵌套）→ 退化为该元素的兄弟，留在
        // 同一个 section 里而不是甩到顶层。
        let text = r#"
{"el":"section","role":"stats"}
{"el":"stat_card","in":1,"label":"MRR","value":"$48k"}
{"el":"badge","in":2,"label":"New"}
"#;
        let outcome = parse_manifest(text).expect("manifest");
        assert_eq!(outcome.nodes.len(), 1, "one section root");
        assert_eq!(
            frame_children(&outcome.nodes[0]).len(),
            2,
            "badge lands beside the stat_card inside the section"
        );
        assert!(outcome
            .warnings
            .iter()
            .any(|w| w.contains("treated as its sibling")));
    }

    #[test]
    fn raw_frame_line_acts_as_container_with_injected_ids() {
        // 方舟实测模式:自绘 frame（含行内 children，全程无 id）当
        // 容器，后续行 `in` 进去。
        let text = r#"
{"el":"section","role":"metrics"}
{"type":"frame","in":1,"name":"Card","layout":"vertical","children":[{"type":"text","content":"Total"}]}
{"el":"badge","in":2,"label":"+12%"}
"#;
        let outcome = parse_manifest(text).expect("manifest");
        assert_eq!(outcome.dropped_lines, 0, "{:?}", outcome.warnings);
        assert_eq!(outcome.nodes.len(), 1);
        let section_children = frame_children(&outcome.nodes[0]);
        assert_eq!(section_children.len(), 1, "card inside section");
        // 卡片 = 行内 text + 追加的 badge。
        assert_eq!(frame_children(&section_children[0]).len(), 2);
        assert!(
            !outcome
                .warnings
                .iter()
                .any(|w| w.contains("W-IN-NOT-CONTAINER")),
            "raw frame accepted as container: {:?}",
            outcome.warnings
        );
    }

    #[test]
    fn raw_pen_node_lines_mix_with_manifest_lines() {
        let text = r#"
{"el":"section","role":"hero"}
{"type":"text","id":"t1","content":"Hand-rolled","in":1}
{"el":"stat_card","in":1,"label":"MRR","value":"$48k"}
"#;
        let outcome = parse_manifest(text).expect("manifest");
        assert_eq!(outcome.raw_node_lines, 1);
        assert_eq!(outcome.element_lines, 2);
        assert_eq!(outcome.nodes.len(), 1);
        assert_eq!(frame_children(&outcome.nodes[0]).len(), 2);
    }

    #[test]
    fn reasoning_blocks_are_stripped_before_parsing() {
        let text = r#"
<think>{"el":"stat_card","label":"draft","value":"0"}</think>
{"el":"badge","label":"Live"}
"#;
        let outcome = parse_manifest(text).expect("manifest");
        assert_eq!(outcome.element_lines, 1);
        assert_eq!(outcome.nodes.len(), 1);
    }

    #[test]
    fn unknown_element_line_coerces_to_frame_without_killing_the_batch() {
        // An invented kind no longer drops — dropping loses container
        // subtrees (the glm-5.2 `card` regression). It coerces to a frame;
        // the rest of the batch is unaffected.
        let text = r#"
{"el":"nonexistent_widget","label":"x"}
{"el":"badge","label":"Live"}
"#;
        let outcome = parse_manifest(text).expect("manifest");
        assert_eq!(
            outcome.dropped_lines, 0,
            "unknown coerces now, no longer drops"
        );
        assert_eq!(
            outcome.element_lines, 1,
            "badge is the only catalog element"
        );
        assert_eq!(outcome.nodes.len(), 2, "coerced frame + badge");
        assert!(outcome
            .warnings
            .iter()
            .any(|w| w.contains("W-COERCED-FRAME")));
    }

    #[test]
    fn line_numbers_survive_a_coerced_line() {
        // 行 1 是未知 kind（coerce 成 frame、占住行号）；行 3 的 `in:2` 仍
        // 正确指向行 2 的 section，badge 进 section 而非错位到顶层。
        let text = r#"
{"el":"nonexistent_widget"}
{"el":"section","role":"body"}
{"el":"badge","in":2,"label":"Live"}
"#;
        let outcome = parse_manifest(text).expect("manifest");
        assert_eq!(
            outcome.nodes.len(),
            2,
            "coerced frame (line 1) + section (line 2)"
        );
        // The section is the 2nd root and still holds the badge — proving
        // the coerced line 1 占住了行号 so `in:2` resolved to the section.
        assert_eq!(
            frame_children(&outcome.nodes[1]).len(),
            1,
            "badge nested in the section"
        );
    }

    // ── Manifest opt-in routing (off by default) ──────────────────────
    // Every case holds MANIFEST_ENV_LOCK: the gate reads process-global
    // env, so running in parallel with subagent's set/remove tests would
    // clobber each other.

    #[test]
    fn manifest_off_by_default_without_env() {
        let _guard = crate::test_support::MANIFEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("OPENPENCIL_MANIFEST");
        // Off for EVERY model — the retired M4 family auto-routing
        // (minimax / glm-5.1 / deepseek / ark-code) no longer opts in.
        for id in [
            "MiniMax-M3",
            "glm-5.1",
            "deepseek-v4-pro",
            "ark-code-latest",
            "opencode/glm-5.1",
            "claude-sonnet-4-6",
            "gpt-4o",
            "gemini-2.5-pro",
            "",
        ] {
            assert!(
                !manifest_enabled_for_model(id),
                "{id:?} should be OFF by default"
            );
        }
    }

    #[test]
    fn manifest_env_force_on_enables_any_model() {
        let _guard = crate::test_support::MANIFEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        for truthy in ["1", "true", "on"] {
            std::env::set_var("OPENPENCIL_MANIFEST", truthy);
            for id in ["claude-sonnet-4-6", "MiniMax-M3", "glm-5.2", ""] {
                assert!(
                    manifest_enabled_for_model(id),
                    "force-on ({truthy:?}) must enable {id:?}"
                );
            }
        }
        std::env::remove_var("OPENPENCIL_MANIFEST");
    }

    #[test]
    fn manifest_env_non_truthy_stays_off() {
        let _guard = crate::test_support::MANIFEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        // Only 1/true/on opt in; everything else (incl. the legacy
        // 0/false/off rollback values and unknown strings) stays off.
        for falsy in ["0", "false", "off", "yes", "garbage"] {
            std::env::set_var("OPENPENCIL_MANIFEST", falsy);
            assert!(
                !manifest_enabled_for_model("MiniMax-M3"),
                "{falsy:?} must keep manifest OFF"
            );
        }
        std::env::remove_var("OPENPENCIL_MANIFEST");
    }
}
