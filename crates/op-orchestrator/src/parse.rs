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

/// 递归总节点数(含后代),复用 cleanup 的后代计数。用于在候选间取"最完整"
/// 的解析结果。
fn total_nodes(nodes: &[PenNode]) -> usize {
    nodes.len()
        + nodes
            .iter()
            .map(crate::cleanup::count_descendants)
            .sum::<usize>()
}

/// JSONL / bare-object 收集:把每个顶层 `{...}` 解析为一个 `PenNode`(非节点
/// 对象——如 `{"nodes":[…]}` 这种无 `type` 的包裹——被跳过)。返回全部成功
/// 解析的顶层节点;由 [`parse_nodes`] 与数组形态按总节点数比较择优。
fn collect_bare_objects(text: &str) -> Vec<PenNode> {
    let mut nodes = Vec::new();
    for obj in balanced_spans(text, b'{', b'}') {
        let Ok(mut value) = serde_json::from_str::<serde_json::Value>(obj) else {
            continue;
        };
        normalize_generated_node_json(&mut value);
        if let Ok(node) = serde_json::from_value::<PenNode>(value) {
            nodes.push(node);
        }
    }
    nodes
}

/// 走完整 parse 管线解析单个候选 `[...]`:serde `Value` → normalize →
/// **逐元素**反序列化为 `Vec<PenNode>`。
///
/// 弱模型常在节点数组里混入个别坏元素(例如把 fill 的
/// `{"type":"solid"}` 当成节点写进数组)。老的整组
/// `from_value::<Vec<PenNode>>` 只要一个元素坏就丢掉**整段子任务**
/// (实测 Dashboard charts-row 因此 0 节点、后续 table-section 也被
/// 中断)。这里改为逐元素反序列化:保留有效节点、跳过并记录坏的。
/// 全有效时结果与老逻辑一致(严格不回退);全坏 / 空数组返回 `Err`
/// 以便上层试下一个候选。
fn try_parse_candidate(json: &str) -> Result<Vec<PenNode>, ParseError> {
    let mut value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| ParseError(format!("json: {e}")))?;
    normalize_generated_node_json(&mut value);
    let serde_json::Value::Array(items) = value else {
        return Err(ParseError("candidate is not a JSON array".into()));
    };
    let total = items.len();
    let mut nodes = Vec::with_capacity(total);
    let mut last_err: Option<String> = None;
    for item in items {
        match serde_json::from_value::<PenNode>(item) {
            Ok(node) => nodes.push(node),
            Err(e) => last_err = Some(e.to_string()),
        }
    }
    if nodes.is_empty() {
        let detail = last_err.map(|e| format!("; last: {e}")).unwrap_or_default();
        return Err(ParseError(format!(
            "deserialize: 0/{total} nodes valid{detail}"
        )));
    }
    if nodes.len() < total {
        eprintln!(
            "[parse] tolerated {} malformed node(s) of {total}, kept {}",
            total - nodes.len(),
            nodes.len()
        );
    }
    Ok(nodes)
}

fn normalize_generated_node_json(value: &mut serde_json::Value) {
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

            for child in object.values_mut() {
                normalize_generated_node_json(child);
            }
        }
        _ => {}
    }
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
fn balanced_spans(text: &str, open: u8, close: u8) -> Vec<&str> {
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
mod tests {
    use super::*;
    use op_editor_core::PenNodeExt as _;

    #[test]
    fn parse_nodes_reads_fenced_pennode_array() {
        // 一个最小的 frame 节点(canonical schema:`type` 标签 +
        // base 字段)。
        let text = r#"Sure, here are the nodes:
```json
[
  { "type": "frame", "id": "hero", "name": "Hero", "x": 0, "y": 0, "width": 1200, "height": 400, "children": [] }
]
```"#;
        let nodes = parse_nodes(text).expect("parse");
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].id_str(), "hero");
    }

    #[test]
    fn parse_nodes_rejects_empty_array() {
        assert!(parse_nodes("[]").is_err());
    }

    #[test]
    fn parse_nodes_rejects_no_array() {
        assert!(parse_nodes("the model wrote prose only").is_err());
    }

    #[test]
    fn parse_nodes_defaults_missing_image_src_to_empty_string() {
        let text = r#"[
  { "type": "image", "id": "photo", "name": "Restaurant photo", "x": 0, "y": 0, "width": 240, "height": 160 }
]"#;

        let nodes = parse_nodes(text).expect("missing image src should be tolerated");
        let PenNode::Image(image) = &nodes[0] else {
            panic!("expected image node");
        };
        assert_eq!(image.src, "");
    }

    #[test]
    fn parse_nodes_skips_prose_bracket_before_fenced_array() {
        // 弱模型在真正 JSON 之前写了带 `[` 的推理散文。老逻辑取第一个
        // `[` 会抓到 `[step 1]` 报 "expected ident";新逻辑扫描所有平衡
        // 数组、取节点数最多的有效结果,跳过散文括号。
        let text = "Let me plan this [step 1]: build the card.\n```json\n[\n  { \"type\": \"frame\", \"id\": \"card\", \"name\": \"Card\", \"x\": 0, \"y\": 0, \"width\": 300, \"height\": 200, \"children\": [] }\n]\n```";
        let nodes = parse_nodes(text).expect("should skip prose bracket and parse real array");
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].id_str(), "card");
    }

    #[test]
    fn parse_nodes_tolerates_one_bad_node_keeps_rest() {
        // 弱模型在节点数组里混入一个 `type:"solid"`(fill 类型当节点)。
        // 老的整组 from_value 会因这一个坏元素丢掉整段;新逻辑逐元素
        // 反序列化、跳过坏的、保留有效节点(实测 Dashboard charts-row)。
        let text = r##"[
          { "type": "frame", "id": "a", "name": "A", "x": 0, "y": 0, "width": 100, "height": 50, "children": [] },
          { "type": "solid", "color": "#06B6D4" },
          { "type": "frame", "id": "b", "name": "B", "x": 0, "y": 0, "width": 100, "height": 50, "children": [] }
        ]"##;
        let nodes = parse_nodes(text).expect("should keep the valid nodes, skip the bad one");
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].id_str(), "a");
        assert_eq!(nodes[1].id_str(), "b");
    }

    #[test]
    fn parse_nodes_returns_outer_array_not_nested_children() {
        // 嵌套格式:外层只有 1 个 root frame,其 children 有 3 个。只收
        // 顶层数组,返回外层 [root],而不是内层 [a,b,c](Codex review)。
        let text = r#"[
          { "type": "frame", "id": "root", "name": "Root", "x": 0, "y": 0, "width": 300, "height": 200, "children": [
            { "type": "frame", "id": "a", "name": "A", "x": 0, "y": 0, "width": 100, "height": 50, "children": [] },
            { "type": "frame", "id": "b", "name": "B", "x": 0, "y": 0, "width": 100, "height": 50, "children": [] },
            { "type": "frame", "id": "c", "name": "C", "x": 0, "y": 0, "width": 100, "height": 50, "children": [] }
          ] }
        ]"#;
        let nodes = parse_nodes(text).expect("parse");
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].id_str(), "root");
    }

    #[test]
    fn parse_nodes_reads_jsonl_bare_objects() {
        // DeepSeek 输出 JSONL:```json 围栏里每行一个裸对象,无 [ ] 包裹。
        // 数组路径只看到内层 children:[] 空数组(失败),JSONL 回退按顶层
        // {...} 逐个解析。
        let text = "```json\n{\"type\":\"frame\",\"id\":\"a\",\"name\":\"A\",\"x\":0,\"y\":0,\"width\":100,\"height\":50,\"children\":[]}\n{\"type\":\"frame\",\"id\":\"b\",\"name\":\"B\",\"x\":0,\"y\":0,\"width\":100,\"height\":50,\"children\":[]}\n```";
        let nodes = parse_nodes(text).expect("JSONL fallback should parse bare objects");
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].id_str(), "a");
        assert_eq!(nodes[1].id_str(), "b");
    }

    #[test]
    fn parse_nodes_bare_object_returns_top_level_not_nested_children() {
        // 单个裸对象(无数组包裹)带真实嵌套 children。合并深度判定下,
        // children:[...] 在 {} 内(深度>0)不被数组路径当节点数组,返回顶层
        // root 而不是内层 [a,b](Codex review)。
        let text = r#"{ "type": "frame", "id": "root", "name": "Root", "x": 0, "y": 0, "width": 300, "height": 200, "children": [
            { "type": "frame", "id": "a", "name": "A", "x": 0, "y": 0, "width": 100, "height": 50, "children": [] },
            { "type": "frame", "id": "b", "name": "B", "x": 0, "y": 0, "width": 100, "height": 50, "children": [] }
        ] }"#;
        let nodes = parse_nodes(text).expect("parse");
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].id_str(), "root");
    }

    #[test]
    fn parse_nodes_finds_array_after_unmatched_prose_bracket() {
        // 散文里有个不闭合的 `[`(如 "options [1, 2 …"),其后才是真数组。
        // 独立尝试每个 `[` + 不闭合即跳过,后面的真数组不被漏掉(Codex
        // review;修上一轮 combined-depth 引入的回归)。
        let text = "Consider options [a and b, then build:\n```json\n[{ \"type\": \"frame\", \"id\": \"hero\", \"name\": \"Hero\", \"x\": 0, \"y\": 0, \"width\": 200, \"height\": 100, \"children\": [] }]\n```";
        let nodes =
            parse_nodes(text).expect("should find the real array after an unmatched prose bracket");
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].id_str(), "hero");
    }

    #[test]
    fn parse_nodes_reads_labelled_wrapper_object() {
        // 模型把节点数组包进带标签的对象 {"nodes":[...]}。colon-skip 曾把
        // "nodes":[...] 一并跳掉;现在按总节点数择优——wrapper 无 type 解析
        // 失败,数组路径取出 [a,b](Codex review)。
        let text = r#"```json
{ "nodes": [
  { "type": "frame", "id": "a", "name": "A", "x": 0, "y": 0, "width": 100, "height": 50, "children": [] },
  { "type": "frame", "id": "b", "name": "B", "x": 0, "y": 0, "width": 100, "height": 50, "children": [] }
] }
```"#;
        let nodes = parse_nodes(text).expect("labelled wrapper array should be parsed");
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].id_str(), "a");
        assert_eq!(nodes[1].id_str(), "b");
    }
}
