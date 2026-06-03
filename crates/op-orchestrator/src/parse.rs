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

/// 从 LLM 文本里抽 JSON 数组并反序列化为 canonical `PenNode` 树。
///
/// 弱模型常在真正的 ```json 数组之前写带 `[` 的推理散文(例如
/// `[step 1]`)。老逻辑只取第一个 `[` 会抓到散文括号,serde 在裸词处
/// 报 `expected ident`,整段子任务直接丢失。这里改为扫描文本里**每一
/// 个**平衡的 `[...]` 候选,逐个走完整 parse 管线,返回**节点数最多**
/// 的非空结果(真正的节点数组通常最大;散文括号 parse 失败被跳过)。
/// 该逻辑严格泛化老的"第一个括号"——老的候选仍在集合内。
/// 空数组 / 全部候选失败视为错误。
pub fn parse_nodes(text: &str) -> Result<Vec<PenNode>, ParseError> {
    let mut best: Option<Vec<PenNode>> = None;
    let mut last_err: Option<ParseError> = None;
    for candidate in balanced_arrays(text) {
        match try_parse_candidate(candidate) {
            Ok(nodes) => {
                if best.as_ref().is_none_or(|b| nodes.len() > b.len()) {
                    best = Some(nodes);
                }
            }
            Err(e) => last_err = Some(e),
        }
    }
    best.ok_or_else(|| last_err.unwrap_or_else(|| ParseError("no JSON array found".into())))
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

/// 收集文本里所有平衡的 `[...]` 子串(忽略字符串字面量内的方括号),
/// 按出现顺序返回。嵌套数组的起点也会进来,但调用方按"节点数最多"
/// 取舍,外层真实数组自然胜出。
fn balanced_arrays(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'[' {
            if let Some(end) = balanced_end(bytes, i) {
                out.push(&text[i..=end]);
            }
        }
    }
    out
}

/// 从 `start`(指向 `[`)起找平衡的 `]` 字节下标;忽略字符串内的方
/// 括号。未闭合返回 `None`。
fn balanced_end(bytes: &[u8], start: usize) -> Option<usize> {
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
        match c {
            b'"' => in_str = true,
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
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
}
