//! 节点解析 —— sub-agent 的 LLM 文本输出 → canonical `PenNode` 树。
//!
//! sub-agent prompt(见 `prompt` 模块)要求模型输出 canonical
//! PenNode JSON 数组。本模块抽出 JSON 数组并 serde 反序列化。

use jian_ops_schema::node::PenNode;

/// 节点解析错误。
#[derive(Debug, Clone)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "node parse error: {}", self.0)
    }
}

/// 从 LLM 文本里抽 JSON 并反序列化为 canonical `PenNode` 树,鲁棒应对弱模型
/// 的各种脏输出:数组前的推理散文 / 不闭合的散文括号 / 数组里混入坏元素 /
/// 嵌套 children / DeepSeek 的 JSONL 裸对象 / `{"nodes":[…]}` 这类带标签的
/// 包裹对象。
///
/// 收集两类候选——(a) 每个平衡的 `[...]`(数组形态);(b) 所有顶层 `{...}`
/// 合起来当一个结果(JSONL / 裸对象形态)——逐个解析,取**总节点数(含后代)
/// 最多**的有效结果。按总数(而非顶层数)比较是关键:完整树天然压过任何被
/// 误抓的内层 children / 字段数组(`{root, children:[a,b]}` 里 root 子树 3 >
/// children 2),于是无需任何 colon/label 启发式,带标签的真数组也不会被误跳。
/// 全部候选失败 / 无候选视为错误。
pub fn parse_nodes(text: &str) -> Result<Vec<PenNode>, ParseError> {
    // 推理模型(MiniMax-M3 / DeepSeek-R 系等)先吐 `<think>…</think>` 再给真正
    // 答案,且 think 里常含**未完成的草稿 JSON**(扁平、带 `//` 注释)。先剥到
    // 最后一个 `</think>` 之后,避免把草稿误当输出(实测 M3 否则被扫出扁平
    // depth-2 残骸)。无闭合标签(在 think 中被 max_tokens 截断)则用原文。
    let text = strip_reasoning(text);
    let mut best: Option<Vec<PenNode>> = None;
    let mut best_total = 0usize;
    let mut last_err: Option<ParseError> = None;
    // (a) Array-form candidates: each balanced `[...]`.
    for candidate in balanced_spans(text, b'[', b']') {
        match try_parse_candidate(candidate) {
            Ok(nodes) => {
                let total = total_nodes(&nodes);
                if best.is_none() || total > best_total {
                    best_total = total;
                    best = Some(nodes);
                }
            }
            Err(e) => last_err = Some(e),
        }
    }
    // (b) Bare-object / JSONL form: all top-level `{...}` as one result.
    let objs = collect_bare_objects(text);
    if !objs.is_empty() {
        let total = total_nodes(&objs);
        if best.is_none() || total > best_total {
            best = Some(objs);
        }
    }
    best.ok_or_else(|| {
        last_err.unwrap_or_else(|| ParseError("no JSON array or objects found".into()))
    })
}

/// 剥掉推理模型的 `<think>…</think>`,返回真正答案部分。两步:
/// 1. **跳过所有已闭合 think 块**:取最后一个 `</think>` / `</thinking>` 之后的文本。
/// 2. 在其余文本里,**截掉任何仍未闭合的 `<think>` 块**(被 `max_tokens` 截断的
///    末尾 think):返回该开标签**之前**的部分。
///
/// 这样无论 think 在开头、还是"闭合块 + 内容 + 末尾被截断的第二个 think 块"
/// (Codex review 指出的二段式),都不会把未完成的 think 草稿当真节点解析。
/// 无 think 标签 → 原样返回(非推理模型安全);末尾截断 → 通常返回空 → 解析失败重试。
pub(crate) fn strip_reasoning(text: &str) -> &str {
    // 1) 跳过所有闭合块。
    let after_closed = ["</think>", "</thinking>"]
        .iter()
        .filter_map(|tag| text.rfind(tag).map(|i| i + tag.len()))
        .max()
        .map(|i| &text[i..])
        .unwrap_or(text);
    // 2) 截掉残留的未闭合 think 块。
    match ["<think>", "<thinking>"]
        .iter()
        .filter_map(|tag| after_closed.find(tag))
        .min()
    {
        Some(start) => &after_closed[..start],
        None => after_closed,
    }
}

/// 递归总节点数(含后代),复用 cleanup 的后代计数。用于在候选间取"最完整"
/// 的解析结果。
fn total_nodes(nodes: &[PenNode]) -> usize {
    nodes.len()
        + nodes
            .iter()
            .map(crate::cleanup::count_descendants)
            .sum::<usize>()
}

/// JSONL / bare-object 收集:取所有顶层 `{...}`,按 `_parent` 重组成树(扁平
/// `_parent` 格式——TS prompt 要求、DeepSeek 用的 JSONL 形态),再逐根反序列化。
/// 非节点对象(`{"nodes":[…]}` 包裹、散落的 `{type:solid}` 等)在重组阶段被
/// [`is_node_object`] 滤掉。由 [`parse_nodes`] 与数组形态按总节点数比较择优。
fn collect_bare_objects(text: &str) -> Vec<PenNode> {
    let items: Vec<serde_json::Value> = balanced_spans(text, b'{', b'}')
        .into_iter()
        .filter_map(|obj| serde_json::from_str(obj).ok())
        .collect();
    deserialize_roots(renest_by_parent(items)).unwrap_or_default()
}

/// 解析单个候选 `[...]`:serde `Value::Array` → 按 `_parent` 重组成树 →
/// 逐根反序列化为 `Vec<PenNode>`。
fn try_parse_candidate(json: &str) -> Result<Vec<PenNode>, ParseError> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| ParseError(format!("json: {e}")))?;
    let serde_json::Value::Array(items) = value else {
        return Err(ParseError("candidate is not a JSON array".into()));
    };
    deserialize_roots(renest_by_parent(items))
}

/// canonical `PenNode` 的 `type` 标签全集。重组阶段据此过滤掉非节点对象
/// (散落的 `{type:solid}` 字段对象、`{"nodes":[…]}` 包裹等)——既是弱模型
/// 容错(坏元素不毁整段),也避免把它们当节点重组进树。
const NODE_KINDS: &[&str] = &[
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
    "ref",
];

pub(crate) fn is_node_object(value: &serde_json::Value) -> bool {
    value
        .get("type")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|ty| NODE_KINDS.contains(&ty))
}

/// 把扁平节点列表按 `_parent` 重组成嵌套树(移植 TS
/// `design-parser.ts::parseJsonlToTree`)。
///
/// TS 的 sub-agent prompt 要求**扁平 `_parent` 格式**(每节点一行、`_parent`
/// 指父 id、root 为 `null`),TS 解析时据此建树。Rust 此前漏了这步——扁平
/// 节点被直接插成兄弟,横向容器的子项全跑到 root 下 → 布局竖排破损。M2.7 /
/// DeepSeek 都用这个格式;方舟用嵌套 children。
///
/// 行为:先用 [`is_node_object`] 滤掉非节点对象;若没有任何 `_parent` 字段,
/// 原样返回(嵌套 / 纯 root 形态);否则按出现顺序建 id→对象 + 父→子表,从
/// root(`_parent:null` 或父不存在的孤儿)递归装配。已取走的 id 不再装配(防环)。
fn renest_by_parent(items: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
    use std::collections::HashMap;
    let mut by_id: HashMap<String, serde_json::Value> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut parent_of: HashMap<String, Option<String>> = HashMap::new();
    let mut seen_parent = false;
    let mut dropped = 0usize;
    for mut item in items {
        if !is_node_object(&item) {
            continue;
        }
        let Some(obj) = item.as_object_mut() else {
            continue;
        };
        let id = match obj.get("id").and_then(serde_json::Value::as_str) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => continue,
        };
        let parent_id = match obj.remove("_parent") {
            Some(serde_json::Value::String(p)) if !p.is_empty() => {
                seen_parent = true;
                Some(p)
            }
            Some(_) => {
                seen_parent = true; // null / anything else → root
                None
            }
            None => None,
        };
        if by_id.contains_key(&id) {
            continue; // duplicate id — keep the first
        }
        // Per-node tolerance (restored after re-nesting): a node whose OWN
        // fields still don't deserialize after normalize would otherwise fail
        // its whole assembled root (re-nesting collapsed the flat array, so the
        // per-element tolerance the array path used to have was lost). Probe
        // each as a childless leaf and drop the unsalvageable ones, keeping the
        // rest of the section instead of losing the entire subtask.
        let mut probe = item.clone();
        if let Some(po) = probe.as_object_mut() {
            po.remove("children");
        }
        normalize_generated_node_json(&mut probe);
        if serde_json::from_value::<PenNode>(probe).is_err() {
            dropped += 1;
            continue;
        }
        order.push(id.clone());
        parent_of.insert(id.clone(), parent_id);
        by_id.insert(id, item);
    }
    if dropped > 0 {
        eprintln!("[parse] renest dropped {dropped} node(s) failing schema validation");
    }
    if !seen_parent {
        // 没有 `_parent`:已是嵌套 / 纯 root 形态,原样返回(保持顺序)。
        return order
            .into_iter()
            .filter_map(|id| by_id.remove(&id))
            .collect();
    }
    let mut children_of: HashMap<String, Vec<String>> = HashMap::new();
    let mut roots: Vec<String> = Vec::new();
    for id in &order {
        match parent_of.get(id).cloned().flatten() {
            Some(parent) if parent != *id && by_id.contains_key(&parent) => {
                children_of.entry(parent).or_default().push(id.clone());
            }
            _ => roots.push(id.clone()), // root / 孤儿 / 自指
        }
    }
    let mut out: Vec<serde_json::Value> = roots
        .into_iter()
        .filter_map(|id| assemble(&id, &mut by_id, &children_of))
        .collect();
    // 无丢失保证:若有节点不可从任何 root 到达(纯 `_parent` 环),按原始顺序
    // 把剩下的提升为 root 装配出来,不静默丢节点(环会被 assemble 的 take 打断)。
    for id in &order {
        if by_id.contains_key(id) {
            if let Some(node) = assemble(id, &mut by_id, &children_of) {
                out.push(node);
            }
        }
    }
    out
}

/// 递归装配 `id` 的子树:从 `by_id` 取出,按 `children_of` 把子节点装进
/// `children`(保留任何已有的内联 children)。已取走的 id 返回 `None`(防环)。
fn assemble(
    id: &str,
    by_id: &mut std::collections::HashMap<String, serde_json::Value>,
    children_of: &std::collections::HashMap<String, Vec<String>>,
) -> Option<serde_json::Value> {
    let mut node = by_id.remove(id)?;
    if let Some(child_ids) = children_of.get(id) {
        let mut children: Vec<serde_json::Value> = Vec::new();
        if let Some(existing) = node
            .get_mut("children")
            .and_then(serde_json::Value::as_array_mut)
        {
            children.append(existing);
        }
        for cid in child_ids {
            if let Some(child) = assemble(cid, by_id, children_of) {
                children.push(child);
            }
        }
        if let Some(obj) = node.as_object_mut() {
            obj.insert("children".to_string(), serde_json::Value::Array(children));
        }
    }
    Some(node)
}

/// 逐根反序列化为 `Vec<PenNode>`(逐根容错:坏根跳过并记录;全有效时与直接
/// 反序列化一致)。空 / 全坏返回 `Err` 以便上层试下一个候选。
fn deserialize_roots(roots: Vec<serde_json::Value>) -> Result<Vec<PenNode>, ParseError> {
    let total = roots.len();
    let mut nodes = Vec::with_capacity(total);
    let mut last_err: Option<String> = None;
    for mut value in roots {
        normalize_generated_node_json(&mut value);
        match serde_json::from_value::<PenNode>(value) {
            Ok(node) => nodes.push(node),
            Err(e) => last_err = Some(e.to_string()),
        }
    }
    if nodes.is_empty() {
        let detail = last_err.map(|e| format!("; last: {e}")).unwrap_or_default();
        return Err(ParseError(format!(
            "deserialize: 0/{total} roots valid{detail}"
        )));
    }
    if nodes.len() < total {
        eprintln!(
            "[parse] tolerated {} malformed root(s) of {total}, kept {}",
            total - nodes.len(),
            nodes.len()
        );
    }
    Ok(nodes)
}

pub(crate) fn normalize_generated_node_json(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Array(items) => {
            for item in items {
                normalize_generated_node_json(item);
            }
        }
        serde_json::Value::Object(object) => {
            if object
                .get("type")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|ty| ty == "image")
                && matches!(object.get("src"), None | Some(serde_json::Value::Null))
            {
                object.insert("src".into(), serde_json::Value::String(String::new()));
            }

            // `fill` given as a bare color/ref or one fill object → wrap into
            // the canonical fill array. PenNode's fill is a sequence; models
            // commonly shorthand it (实测方舟/DeepSeek 否则整 root 报废)。
            if let Some(fill) = object.get_mut("fill") {
                normalize_fill_json(fill);
            }
            normalize_stroke_json(object);
            normalize_layout_enum_json(object);

            for (key, child) in object.iter_mut() {
                // 数值型设计 token(`$type-*-size` 等)只在**数值字段**上就地解析
                // 成数字 —— canonical PenNode 的 fontSize/gap/… 是 f64,模型却按
                // prompt 用这些 `$ref` 字符串(否则整 root 报废,实测方舟 metrics)。
                // 但**只限数值字段**:文本 `content`/`name` 里若恰好是 token 串
                // (如设计系统展示页),必须保持字符串(Codex review)。
                if NUMERIC_TOKEN_FIELDS.contains(&key.as_str()) {
                    if let serde_json::Value::String(s) = child {
                        if let Some(n) = resolve_numeric_design_token(s) {
                            // Whole numbers MUST serialize as integers: `fontWeight`
                            // is `FontWeight::Number(u32)`, which rejects a float
                            // (700.0) — only `700` deserializes. f64 fields
                            // (fontSize/gap/…) accept an integer just fine (Codex
                            // review).
                            *child = if n.fract() == 0.0 {
                                serde_json::json!(n as i64)
                            } else {
                                serde_json::json!(n)
                            };
                            continue;
                        }
                    }
                }
                normalize_generated_node_json(child);
            }
        }
        _ => {}
    }
}

fn normalize_layout_enum_json(object: &mut serde_json::Map<String, serde_json::Value>) {
    if let Some(serde_json::Value::String(layout)) = object.get_mut("layout") {
        if let Some(normalized) = normalize_layout_mode(layout) {
            *layout = normalized.to_string();
        }
    }
    if let Some(serde_json::Value::String(justify)) = object.get_mut("justifyContent") {
        if let Some(normalized) = normalize_justify_content(justify) {
            *justify = normalized.to_string();
        }
    }
}

fn normalize_layout_mode(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "row" | "rows" | "hstack" | "horizontal" => Some("horizontal"),
        "column" | "columns" | "vstack" | "vertical" => Some("vertical"),
        "none" => Some("none"),
        _ => None,
    }
}

fn normalize_justify_content(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "start" | "flex-start" | "left" | "top" => Some("start"),
        "center" | "middle" => Some("center"),
        "end" | "flex-end" | "right" | "bottom" => Some("end"),
        "space_between" | "space-between" | "space between" => Some("space_between"),
        "space_around" | "space-around" | "space around" => Some("space_around"),
        "space_evenly" | "space-evenly" | "space evenly" => Some("space_around"),
        _ => None,
    }
}

fn normalize_stroke_json(object: &mut serde_json::Map<String, serde_json::Value>) {
    let stroke_width = object
        .remove("strokeWidth")
        .or_else(|| object.remove("stroke-width"))
        .and_then(number_from_json);
    let Some(mut stroke) = object.remove("stroke") else {
        return;
    };

    if let serde_json::Value::Array(mut items) = stroke {
        stroke = if items.is_empty() {
            serde_json::Value::Null
        } else {
            items.remove(0)
        };
    }

    let normalized = match stroke {
        serde_json::Value::String(color) => serde_json::json!({
            "thickness": stroke_width.unwrap_or(1.0),
            "fill": [{ "type": "solid", "color": color }]
        }),
        serde_json::Value::Object(mut stroke_obj) => {
            let color = stroke_obj
                .remove("color")
                .and_then(string_from_json)
                .or_else(|| {
                    stroke_obj.get("type").and_then(|ty| {
                        (ty.as_str() == Some("solid")).then(|| "#000000".to_string())
                    })
                });
            stroke_obj.remove("type");
            if let Some(fill) = stroke_obj.get_mut("fill") {
                normalize_fill_json(fill);
            } else if let Some(color) = color {
                stroke_obj.insert(
                    "fill".into(),
                    serde_json::json!([{ "type": "solid", "color": color }]),
                );
            }
            if !stroke_obj.contains_key("thickness") {
                stroke_obj.insert(
                    "thickness".into(),
                    serde_json::json!(stroke_width.unwrap_or(1.0)),
                );
            }
            serde_json::Value::Object(stroke_obj)
        }
        _ => return,
    };

    object.insert("stroke".into(), normalized);
}

fn normalize_fill_json(fill: &mut serde_json::Value) {
    match fill {
        serde_json::Value::String(color) => {
            *fill = serde_json::json!([{ "type": "solid", "color": color.clone() }]);
        }
        serde_json::Value::Object(_) => {
            let single = std::mem::take(fill);
            *fill = serde_json::Value::Array(vec![single]);
        }
        serde_json::Value::Array(items) => {
            for item in items {
                if let serde_json::Value::String(color) = item {
                    *item = serde_json::json!({ "type": "solid", "color": color.clone() });
                }
            }
        }
        _ => {}
    }
}

fn number_from_json(value: serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s
            .parse::<f64>()
            .ok()
            .or_else(|| resolve_numeric_design_token(&s)),
        _ => None,
    }
}

fn string_from_json(value: serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) if !s.trim().is_empty() => Some(s),
        _ => None,
    }
}

/// 接受数值型设计 token 解析的字段(canonical PenNode 里是 f64 的几何 / 排版
/// 字段)。其他字段(content / name / color / iconFontName / …)里的 token 串
/// 保持原样,不被改写成数字。
const NUMERIC_TOKEN_FIELDS: &[&str] = &[
    "fontSize",
    "fontWeight",
    "lineHeight",
    "letterSpacing",
    "gap",
    "padding",
    "cornerRadius",
    "thickness",
    "strokeWidth",
    "width",
    "height",
];

/// 把**数值型**设计 token 字符串解析成 prompt 文档化的默认数值:
/// `$type-{tier}-{size|weight|line-height|letter-spacing}`、`$spacing-{1..5}`、
/// `$radius-{sm|md|lg}`。weight 也解析成数字 —— `FontWeight` 是 untagged
/// `{Number, Keyword}`,接受数字;未解析的 `$type-*-weight` 串会被当 keyword
/// 反序列化、再被字重解析器当未知值退回默认字重(标题该 700 却渲成 400,Codex
/// review)。颜色 `$color-*` 等非数值 token 返回 `None`(在 fill 里合法、保持字符串)。
fn resolve_numeric_design_token(s: &str) -> Option<f64> {
    if let Some(rest) = s.strip_prefix("$type-") {
        if let Some(t) = rest.strip_suffix("-letter-spacing") {
            return match t {
                "display" => Some(-0.5),
                "uppercase-label" => Some(1.5),
                _ => None,
            };
        }
        let (tier, prop) = if let Some(t) = rest.strip_suffix("-line-height") {
            (t, "lh")
        } else if let Some(t) = rest.strip_suffix("-size") {
            (t, "size")
        } else if let Some(t) = rest.strip_suffix("-weight") {
            (t, "weight")
        } else {
            return None;
        };
        // (size, weight, line-height) per the prompt's typography scale.
        let (size, weight, lh) = match tier {
            "display" => (64.0, 700.0, 1.0),
            "h1" => (24.0, 600.0, 1.2),
            "h2" => (20.0, 600.0, 1.25),
            "h3" => (16.0, 600.0, 1.3),
            "body" => (14.0, 400.0, 1.5),
            "caption" => (12.0, 400.0, 1.4),
            _ => return None,
        };
        return Some(match prop {
            "size" => size,
            "weight" => weight,
            _ => lh,
        });
    }
    if let Some(n) = s.strip_prefix("$spacing-") {
        return match n {
            "1" => Some(4.0),
            "2" => Some(8.0),
            "3" => Some(12.0),
            "4" => Some(16.0),
            "5" => Some(24.0),
            _ => None,
        };
    }
    if let Some(r) = s.strip_prefix("$radius-") {
        return match r {
            "sm" => Some(4.0),
            "md" => Some(8.0),
            "lg" => Some(12.0),
            _ => None,
        };
    }
    None
}

/// 收集文本里可作为候选的 `open..close` 子串(`[...]` 或 `{...}`),按出现
/// 顺序返回。字符串字面量内的括号忽略。两条规则保证对脏输入鲁棒:
/// - **独立尝试**每个 `open`:`balanced_end` 未闭合即跳过该位置(+1 前进)。
///   于是散文里**不闭合的 `[`**(如 "options [1,2…")不会污染后续——其后真正
///   的数组仍被独立找到。(不能用累积深度,否则一个不闭合括号会把后面全压到
///   深度 >0、整段漏掉。)
/// - **跳过整段**(skip-past):捕获一段后跳到其尾后,内层子串(数组里的嵌套
///   children、对象里的字段数组等)不会被当独立候选。
///
/// 不在此处按"是否字段值 / 嵌套"过滤——交给 [`parse_nodes`] 按总节点数择优:
/// 完整树天然压过被误抓的内层数组,也不会误伤 `{"nodes":[…]}` 这类带标签的
/// 真数组(colon-skip 曾把它一并跳掉)。
pub(crate) fn balanced_spans(text: &str, open: u8, close: u8) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut in_str = false;
    let mut esc = false;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if in_str {
            if esc {
                esc = false;
            } else if c == b'\\' {
                esc = true;
            } else if c == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        if c == b'"' {
            in_str = true;
            i += 1;
            continue;
        }
        if c == open {
            if let Some(end) = balanced_end(bytes, i, open, close) {
                out.push(&text[i..=end]);
                i = end + 1; // skip the captured span — nested sub-spans aren't separate candidates
                continue;
            }
        }
        i += 1;
    }
    out
}

/// 从 `start`(指向 `open`)起找平衡的 `close` 字节下标;忽略字符串内的
/// 括号。未闭合返回 `None`。
fn balanced_end(bytes: &[u8], start: usize, open: u8, close: u8) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    for (i, &c) in bytes.iter().enumerate().skip(start) {
        if in_str {
            if esc {
                esc = false;
            } else if c == b'\\' {
                esc = true;
            } else if c == b'"' {
                in_str = false;
            }
            continue;
        }
        if c == b'"' {
            in_str = true;
        } else if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;
